use std::{
    error::Error,
    path::{Path, PathBuf},
    io::Cursor,
};

use xmltree::Element;

type AnyResult<T> = Result<T, Box<dyn Error>>;

fn get_files_dir() -> PathBuf {
    let root_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(root_dir).join("src").join("tests").join("files")
}

#[test]
fn find_project() {
    let files_dir = get_files_dir();

    let proj = crate::find_fsproj(files_dir.join("test_file.fs").to_str().unwrap(), 1);
    let expected = files_dir
        .join("project.fsproj")
        .to_str()
        .unwrap()
        .to_owned();

    assert_eq!(proj, Some(expected))
}

#[test]
fn find_project_nested() {
    let files_dir = get_files_dir();

    let test_file = files_dir.join("directory").join("inside_directory.fs");

    let expected = files_dir
        .join("project.fsproj")
        .to_str()
        .unwrap()
        .to_owned();

    let proj = crate::find_fsproj(test_file.to_str().unwrap(), 1);
    assert_eq!(proj, None, "Should fail due to not enough depth");

    let proj = crate::find_fsproj(test_file.to_str().unwrap(), 2);
    assert_eq!(proj, Some(expected));
}

#[test]
fn xml_parse() -> AnyResult<()> {
    let projects_dir = get_files_dir().join("projects");

    let with_version = projects_dir
        .join("with_version.fsproj")
        .display()
        .to_string();
    let without_version = projects_dir
        .join("without_version.fsproj")
        .display()
        .to_string();

    let files = crate::get_files_from_project(crate::open_file(&with_version)?)?;

    assert_eq!(
        files,
        vec![
            "One.fs".to_string(),
            "Two.fs".to_string(),
            "Three.fs".to_string(),
            "Four.fs".to_string(),
            "Five.fs".to_string(),
        ]
    );

    let files = crate::get_files_from_project(crate::open_file(&without_version)?)?;

    assert_eq!(
        files,
        vec![
            "One.fs".to_string(),
            "Two.fs".to_string(),
            "Three.fs".to_string(),
            "Four.fs".to_string(),
            "Five.fs".to_string(),
        ]
    );

    Ok(())
}

#[test]
fn set_files() -> AnyResult<()> {
    let original = Cursor::new(include_str!("files/projects/set_files_original.fsproj"));

    let expected = {
        let src = include_str!("files/projects/set_files_expected.fsproj");
        let cursor = Cursor::new(src);
        Element::parse(cursor)
    }.unwrap();

    let files = [
        "FileA",
        "FileB",
        "FileC",
    ];

    let result = crate::set_files_in_project(original, &files)?;

    assert_eq!(result, expected);

    Ok(())
}

#[test]
fn get_file_name() {
    let path = "files/projects/set_files_expected.fsproj";

    assert_eq!(
        crate::get_filename(&path),
        Some("set_files_expected".to_owned())
    );
}
