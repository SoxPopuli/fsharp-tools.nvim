// Functions to expose
//  1. find nearest fsproj
//  2. extract data from fsproj
//  3. reassemble fsproj file with new files

#[cfg(test)]
mod tests;

mod error;
use crate::error::Error;

mod file;
use file::{ExclusiveFileLock, SharedFileLock};

use cfg_if::cfg_if;
use error::{OptionToLuaError, ResultToLuaError};
use xmltree::{Element, EmitterConfig, XMLNode};

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};

const LINE_ENDING: &str = if cfg!(unix) { "\n" } else { "\r\n" };


fn open_file_read(file_path: &str) -> Result<SharedFileLock, Error> {
    let file = File::open(file_path)
        .map_err(|_| Error::FileError(format!("Failed to open file: {file_path}")))?;

    SharedFileLock::new(file)
}

fn open_file_write(file_path: &str) -> Result<ExclusiveFileLock, Error> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .map_err(|e| Error::FileError(e.to_string()))
        .and_then(ExclusiveFileLock::new)
}

fn parse_root(project: impl Read) -> Result<Element, Error> {
    Element::parse(project)
        .map_err(|e| Error::FileError(format!("Failed to parse project: {}", e.to_string())))
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

    let mut paths: Vec<_> = files.map(|e| e.attributes["Include"].clone()).collect();

    for p in paths.iter_mut() {
        let (name, _) = p
            .split_once('.')
            .ok_or(Error::FileError(format!("missing extension for {}", p)))?;

        *p = name.to_owned();
    }

    Ok(paths)
}

/// Takes the path to a file, then walks up the directory until it
/// finds a fsproj file or hits max depth
fn find_fsproj(file_path: &str, max_depth: i32) -> Option<String> {
    let path = {
        let tmp = Path::new(file_path)
            .canonicalize()
            .expect(&format!("failed to canonicalize path {file_path}"));
        if tmp.is_file() {
            tmp.parent()?.to_path_buf()
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

    fn find_until(path: impl AsRef<Path>, depth: i32, max_depth: i32) -> Option<PathBuf> {
        if depth >= max_depth {
            None
        } else {
            match find_from_path(path.as_ref()) {
                Some(path) => Some(path),
                None => find_until(path.as_ref().parent()?, depth + 1, max_depth),
            }
        }
    }

    find_until(path, 0, max_depth).and_then(|path| Some(path.to_str()?.to_owned()))
}

fn fix_start_and_end<Output, Original>(
    mut output_file: Output,
    mut original_file: Original,
) -> Result<String, Error>
where
    Output: Read + Seek,
    Original: Read + Seek,
{
    fn read_lines(file: impl Read + Seek) -> Result<Vec<String>, Error> {
        BufReader::new(file)
            .lines()
            .map(|line| line.map_err(Error::file_error))
            .collect::<Result<_, _>>()
    }

    output_file.rewind()?;
    original_file.rewind()?;

    let original = {
        let mut buf = String::new();
        BufReader::new(original_file)
            .read_to_string(&mut buf)
            .map_err(Error::file_error)?;
        buf
    };
    let original_lines = original.lines().collect::<Vec<_>>();
    let mut output = read_lines(&mut output_file)?;

    if original.is_empty() || output.is_empty() {
        return Ok("".to_string());
    }

    output[0] = original_lines[0].to_string();

    let mut joined = output.join(LINE_ENDING);
    if original.ends_with(LINE_ENDING) {
        if !joined.ends_with(LINE_ENDING) {
            joined.push_str(LINE_ENDING);
        }
    } else {
        if joined.ends_with(LINE_ENDING) {
            joined = joined
                .strip_suffix(LINE_ENDING)
                .map(|x| x.to_string())
                .unwrap_or(joined);
        }
    }

    Ok(joined)
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

        let file_names = file_names
            .iter()
            .rev()
            .filter(|s| s.as_ref().trim().len() > 0);

        for item in file_names {
            let mut element = Element::new("Compile");
            element
                .attributes
                .insert("Include".into(), format!("{}.fs", item.as_ref()));
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

fn write_project(buf: &mut impl Write, element: &Element, indent: &str) -> Result<(), Error> {
    let data_to_write = {
        let mut buffer = BufWriter::new(Vec::<u8>::new());

        let config = EmitterConfig::new()
            .perform_indent(true)
            .line_separator(LINE_ENDING)
            .indent_string(indent.to_string());

        element
            .write_with_config(&mut buffer, config)
            .map_err(|e| Error::FileError(e.to_string()))?;

        let buffer = buffer
            .into_inner()
            .map_err(|e| Error::FileError(e.to_string()))?;
        String::from_utf8(buffer).map_err(|e| Error::FileError(e.to_string()))?
    };

    cfg_if! {
        if #[cfg(debug_assertions)] {
            write_log("to_write", &data_to_write)?;
        }
    }

    buf.write_all(data_to_write.as_bytes())
        .map_err(Error::IOError)?;

    Ok(())
}

fn write_project_to_string(element: &Element, indent: &str) -> Result<String, Error> {
    // let mut file = open_file_write(file_path)?;
    let mut buf = BufWriter::new(Vec::new());
    write_project(&mut buf, element, indent)?;

    String::from_utf8(buf.into_inner().unwrap()).map_err(Error::file_error)
}

#[cfg(debug_assertions)]
fn write_log<T: std::fmt::Display>(name: &str, input: T) -> Result<(), Error> {
    let file_dir = if cfg!(unix) {
        "/tmp/fs-tools.log".to_string()
    } else {
        Path::new(&std::env::var("TEMP").unwrap())
            .join("fs-tools.log")
            .to_string_lossy()
            .to_string()
    };

    let mut file = File::options()
        .create(true)
        .write(true)
        .append(true)
        .open(file_dir)
        .map_err(Error::IOError)?;

    writeln!(file, "{}: {}", name, input).map_err(Error::IOError)?;

    Ok(())
}

fn get_file_name(file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    //let file_name = path.file_name()?;

    path.file_stem()
        .and_then(|x| x.to_str())
        .map(|x| x.to_owned())
}

/// Returns indent string
fn derive_file_indent_level(file: impl Read) -> Option<String> {
    let reader = BufReader::new(file);

    fn get_prefix(line: &str, prefix: char) -> String {
        line.chars().take_while(|c| c == &prefix).collect()
    }

    reader.lines().flatten().find_map(|line| {
        let first_char = line.chars().next();
        if first_char == Some(' ') {
            Some(get_prefix(&line, ' '))
        } else if first_char == Some('\t') {
            Some(get_prefix(&line, '\t'))
        } else {
            None
        }
    })
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
            let file = open_file_read(&file_path).to_lua_error()?;

            let result = get_files_from_project(file).to_lua_error()?;
            Ok(result)
        })?,
    )?;

    table.set(
        "write_files_to_project",
        lua.create_function(
            |_, (file_path, files, indent): (String, Vec<String>, Option<u8>)| {
                let mut original = open_file_read(&file_path)?;

                let mut original_content = {
                    let mut buf = String::new();
                    original.read_to_string(&mut buf)?;
                    Cursor::new(buf)
                };

                fn build_indent_string(size: u8) -> String {
                    (0..size).map(|_| ' ').collect()
                }

                let indent = derive_file_indent_level(&mut original_content)
                    .or(indent.map(build_indent_string))
                    .unwrap_or(build_indent_string(2));
                original_content.rewind()?;

                let project = set_files_in_project(&mut original_content, &files)?;
                original_content.rewind()?;
                let output = Cursor::new(write_project_to_string(&project, &indent)?);

                drop(original);

                let fixed = fix_start_and_end(output, original_content)?;

                let mut output_file = open_file_write(&file_path)?;
                output_file.write_all(fixed.as_bytes())?;

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
