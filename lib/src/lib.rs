// Functions to expose
//  1. find nearest fsproj
//  2. extract data from fsproj
//  3. reassemble fsproj file with new files

mod error;
#[cfg(test)]
mod tests;
use crate::error::Error;

use xmltree::Element;

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

fn get_files_from_project(file_path: &str) -> Result<Vec<String>, Error> {
    let reader = File::open(file_path)
        .map(BufReader::new)
        .map_err(|_| Error::FileError { path: file_path })?;

    let root = Element::parse(reader).map_err(|e| Error::ParseError(e.to_string()))?;

    let item_groups = root.children.iter().filter_map(|node| {
        let elem = node.as_element()?;

        if elem.name == "ItemGroup" {
            Some(elem)
        } else {
            None
        }
    });

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

use mlua::prelude::*;

#[mlua::lua_module(name = "libfsharp_tools_rs")]
fn module(lua: &Lua) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;

    table.set(
        "find_fsproj",
        lua.create_function(|_, (file_path, max_depth): (String, i32)| {
            let err_msg = format!("fsproj not found for file: {}", file_path);
            let result =
                find_fsproj(&file_path, max_depth).ok_or(LuaError::RuntimeError(err_msg))?;

            Ok(result)
        })?,
    )?;

    table.set(
        "get_files_from_project",
        lua.create_function(|_, file_path: String| {
            let result = get_files_from_project(&file_path)
                .map_err(|e| LuaError::RuntimeError(e.to_string()))?;

            Ok(result)
        })?,
    )?;

    Ok(table)
}
