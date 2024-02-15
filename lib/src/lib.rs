// Functions to expose
//  1. find nearest fsproj
//  2. extract data from fsproj
//  3. reassemble fsproj file with new files

mod error;
#[cfg(test)]
mod tests;
use crate::error::Error;

use error::{OptionToLuaError, ResultToLuaError};
use xmltree::{Element, EmitterConfig, XMLNode};

use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

fn open_file(file_path: &str) -> Result<BufReader<File>, Error> {
    File::open(file_path)
        .map(BufReader::new)
        .map_err(|_| Error::FileError(format!("Failed to open file: {file_path}")))
}

fn parse_root(project: impl Read) -> Result<Element, Error> {
    Element::parse(project).map_err(|_| Error::FileError("Failed to parse project".into()))
}

fn get_item_groups(element: &Element) -> impl Iterator<Item = &Element> {
    element.children.iter().filter_map(|node| {
        let elem = node.as_element()?;

        if elem.name == "ItemGroup" {
            Some(elem)
        } else {
            None
        }
    })
}

fn get_files_from_project(project: impl Read) -> Result<Vec<String>, Error> {
    let root = parse_root(project)?;
    let item_groups = get_item_groups(&root);

    let files = item_groups.flat_map(|ig| {
        ig.children
            .iter()
            .filter_map(|node| node.as_element())
            .filter(|elem| elem.name == "Compile")
    });

    let paths: Vec<_> = files.map(|e| e.attributes["Include"].clone()).collect();

    Ok(paths)
}

/// Takes the path to a file, then walks up the directory until it
/// finds a fsproj file or hits max depth
fn find_fsproj(file_path: &str, max_depth: i32) -> Option<String> {
    let path = {
        let tmp = Path::new(file_path);
        if tmp.is_file() {
            tmp.parent()?
        } else {
            tmp
        }
    };

    fn find_from_path(path: &Path) -> Option<PathBuf> {
        path.read_dir().ok()?.find_map(|entry| match entry {
            Ok(entry) => {
                if entry.file_name().to_str()?.ends_with(".fsproj") {
                    Some(entry.path())
                } else {
                    None
                }
            }
            Err(_) => None,
        })
    }

    fn find_until(path: &Path, depth: i32, max_depth: i32) -> Option<PathBuf> {
        if depth >= max_depth {
            None
        } else {
            match find_from_path(path) {
                Some(path) => Some(path),
                None => find_until(path.parent()?, depth + 1, max_depth),
            }
        }
    }

    find_until(path, 0, max_depth).and_then(|path| Some(path.to_str()?.to_owned()))
}

fn set_files_in_project<T: AsRef<str>>(
    project: impl Read,
    file_names: &[T],
) -> Result<Element, Error> {
    let mut root = parse_root(project)?;

    fn replace_files_in_group<T: AsRef<str>>(group: &mut Element, file_names: &[T]) {
        group.children.retain(|n| {
            if let Some(elem) = n.as_element() {
                return elem.name != "Compile";
            }
            true
        });

        for item in file_names.iter().rev() {
            let mut element = Element::new("Compile");
            element
                .attributes
                .insert("Include".into(), item.as_ref().to_string());
            group.children.push(XMLNode::Element(element));
        }

        group.children.reverse();
    }

    for child in root.children.iter_mut() {
        if let XMLNode::Element(elem) = child {
            if elem.name == "ItemGroup"
                && elem
                    .children
                    .iter()
                    .find(|n| n.as_element().map(|e| e.name == "Compile").is_some())
                    .is_some()
            {
                replace_files_in_group(elem, file_names);
                break;
            }
        }
    }

    Ok(root)
}

fn write_project_to_file(file_path: &str, element: &Element, indent: u8) -> Result<(), Error> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file_path)
        .map_err(|e| Error::FileError(e.to_string()))?;

    let original = {
        let mut s = String::new();
        file.read_to_string(&mut s).map_err(Error::IOError)?;
        s
    };
    file.set_len(0).map_err(Error::IOError)?;
    file.seek(SeekFrom::Start(0)).map_err(Error::IOError)?;

    let data_to_write = {
        let mut buffer = BufWriter::new(Vec::<u8>::new());

        let indent_string: String = (0..indent).map(|_| ' ').collect();
        let config = EmitterConfig::new()
            .perform_indent(true)
            .indent_string(indent_string);

        element
            .write_with_config(&mut buffer, config)
            .map_err(|e| Error::FileError(e.to_string()))?;

        let buffer = buffer
            .into_inner()
            .map_err(|e| Error::FileError(e.to_string()))?;
        String::from_utf8(buffer).map_err(|e| Error::FileError(e.to_string()))?
    };

    let output = choose_from_diff(&original, &data_to_write).collect::<Vec<_>>();

    let mut log = File::create("/tmp/fs-tools.log").unwrap();
    writeln!(log, "original: {:#?}", &original).unwrap();
    writeln!(log, "to_write: {:#?}", &data_to_write).unwrap();
    writeln!(log, "output: {:#?}", &output).unwrap();

    for i in 0..output.len() {
        let line = output[i].as_bytes();

        file.write(line).map_err(Error::IOError)?;

        if i < output.len() - 1 {
            file.write(b"\n").map_err(Error::IOError)?;
        }
    }

    Ok(())
}

fn choose_from_diff<'a>(
    original: &'a str,
    data_to_write: &'a str,
) -> impl Iterator<Item = &'a str> {
    let diff = diff::lines(original, data_to_write);

    // If line contains "Compile" prefer new, else prefer original
    let output = diff.into_iter().filter_map(|x| {
        use diff::Result;
        match x {
            Result::Left(l) => {
                if l.contains("Compile") {
                    None
                } else {
                    Some(l)
                }
            }
            Result::Both(l, _) => Some(l),
            Result::Right(r) => {
                if r.contains("Compile") {
                    Some(r)
                } else {
                    None
                }
            }
        }
    });
    output
}

fn get_file_name(file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    //let file_name = path.file_name()?;

    path.file_stem()
        .and_then(|x| x.to_str())
        .map(|x| x.to_owned())
}

use mlua::prelude::*;

#[mlua::lua_module(name = "fsharp_tools_rs")]
fn module(lua: &Lua) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;

    table.set(
        "find_fsproj",
        lua.create_function(|_, (file_path, max_depth): (String, i32)| {
            let err_msg = format!("fsproj not found for file: {}", file_path);
            let result = find_fsproj(&file_path, max_depth).to_lua_error(err_msg)?;
            Ok(result)
        })?,
    )?;

    table.set(
        "get_files_from_project",
        lua.create_function(|_, file_path: String| {
            let file = open_file(&file_path).to_lua_error()?;
            let result = get_files_from_project(file).to_lua_error()?;
            Ok(result)
        })?,
    )?;

    table.set(
        "write_files_to_project",
        lua.create_function(
            |_, (file_path, files, indent): (String, Vec<String>, Option<u8>)| {
                let indent = indent.unwrap_or(2);

                open_file(&file_path)
                    .and_then(|file| set_files_in_project(file, &files))
                    .and_then(|project| write_project_to_file(&file_path, &project, indent))
                    .to_lua_error()?;

                Ok(())
            },
        )?,
    )?;

    table.set(
        "get_file_name",
        lua.create_function(|_, file_path: String| {
            get_file_name(&file_path)
                .to_lua_error(format!("Could not get file name for: {file_path}"))
        })?,
    )?;

    Ok(table)
}
