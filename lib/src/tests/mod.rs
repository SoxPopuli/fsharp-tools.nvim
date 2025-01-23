use crate::{fix_start_and_end, write_project_to_string};
use pretty_assertions::assert_eq;
use std::{
    error::Error,
    io::Cursor,
    path::{Path, PathBuf},
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

    let files = crate::get_files_from_project(crate::open_file_read(&with_version)?)?;

    assert_eq!(
        files,
        vec![
            "One".to_string(),
            "Two".to_string(),
            "Three".to_string(),
            "Four".to_string(),
            "Five".to_string(),
        ]
    );

    let files = crate::get_files_from_project(crate::open_file_read(&without_version)?)?;

    assert_eq!(
        files,
        vec![
            "One".to_string(),
            "Two".to_string(),
            "Three".to_string(),
            "Four".to_string(),
            "Five".to_string(),
        ]
    );

    Ok(())
}

#[test]
fn set_files() -> AnyResult<()> {
    let original = Cursor::new(include_str!("files/projects/set_files_original.fsproj"));

    let expected_file = include_str!("files/projects/set_files_expected.fsproj");

    let expected = {
        let src = expected_file;
        let cursor = Cursor::new(src);
        Element::parse(cursor)
    }
    .unwrap();

    let files = ["FileA", "FileB", "FileC"];

    let result = crate::set_files_in_project(original, &files)?;

    assert_eq!(result, expected);

    let result_string = write_project_to_string(&result, "  ")?;

    let fixed = fix_start_and_end(Cursor::new(result_string), Cursor::new(expected_file))?;

    assert_eq!(fixed, expected_file);

    Ok(())
}

#[test]
fn get_file_name() {
    let path = "files/projects/set_files_expected.fsproj";

    assert_eq!(
        crate::get_file_name(&path),
        Some("set_files_expected".to_owned())
    );
}

#[test]
fn ignore_empty_lines() {
    let input = r#"
        <Project Sdk="Microsoft.NET.Sdk">
          <PropertyGroup>
            <OutputType>Exe</OutputType>
            <TargetFramework>net6.0</TargetFramework>
            <GenerateProgramFile>false</GenerateProgramFile>
          </PropertyGroup>
          <ItemGroup>
            <Compile Include="One.fs" />
            <Compile Include="Two.fs" />
            <Compile Include="Three.fs" />
            <Content Include="paket.references" />
          </ItemGroup>
          <ItemGroup>
            <ProjectReference Include="../dependency.fsproj" />
          </ItemGroup>
          <Import Project="..\..\.paket\Paket.Restore.targets" />
        </Project>
    "#
    .as_bytes();

    let tree =
        crate::set_files_in_project(input, &["a", "b", "", " ", "                ", "c"]).unwrap();

    let files = {
        let mut buf = Cursor::new(vec![]);
        crate::write_project(&mut buf, &tree, "  ").unwrap();
        buf.set_position(0);

        crate::get_files_from_project(buf).unwrap()
    };

    assert_eq!(files, vec!["a", "b", "c"]);
}
