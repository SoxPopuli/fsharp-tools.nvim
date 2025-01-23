use crate::{open_file_read, Project};
use pretty_assertions::assert_eq;
use std::{
    error::Error,
    io::Cursor,
    path::{Path, PathBuf},
};

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
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    assert_eq!(proj, Some(expected))
}

#[test]
fn find_project_nested() -> AnyResult<()> {
    let files_dir = get_files_dir();

    let test_file = files_dir.join("directory").join("inside_directory.fs");

    let expected = files_dir
        .join("project.fsproj")
        .canonicalize()?
        .to_str()
        .unwrap()
        .to_owned();

    let proj = crate::find_fsproj(test_file.to_str().unwrap(), 1);
    assert_eq!(proj, None, "Should fail due to not enough depth");

    let proj = crate::find_fsproj(test_file.to_str().unwrap(), 2);
    assert_eq!(proj, Some(expected));

    Ok(())
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

    let project = Project::open(open_file_read(&with_version)?)?;

    assert_eq!(
        project.get_files()?,
        vec![
            "One".to_string(),
            "Two".to_string(),
            "Three".to_string(),
            "Four".to_string(),
            "Five".to_string(),
        ]
    );

    let project = Project::open(open_file_read(&without_version)?)?;

    assert_eq!(
        project.get_files()?,
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

    let files = ["FileA", "FileB", "FileC"];

    let project = Project::open_with_indent(original, "  ")?;
    let project = project.with_files(&files)?;

    assert_eq!(project.content, expected_file);

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
fn ignore_empty_lines() -> AnyResult<()> {
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

    let project = Project::open_with_indent(input, "  ")?.with_files(&[
        "a",
        "b",
        "",
        " ",
        "                ",
        "c",
    ])?;

    let files = {
        let mut buf = Cursor::new(vec![]);
        project.write(&mut buf)?;
        buf.set_position(0);

        Project::open_with_indent(&mut buf, "  ")?.get_files()?
    };

    assert_eq!(files, vec!["a", "b", "c"]);

    Ok(())
}
