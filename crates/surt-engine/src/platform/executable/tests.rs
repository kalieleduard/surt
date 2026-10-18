use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{find_executable, pathext, search};

fn environment(path: &[&Path]) -> Vec<(OsString, OsString)> {
    let joined = std::env::join_paths(path).expect("joinable paths");
    vec![(OsString::from("Path"), joined)]
}

fn shim(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, "@echo off\n").expect("a shim");
    path
}

#[test]
fn pathext_lists_the_extensions_a_windows_shell_tries_in_order() {
    let table = [
        (None, vec![".COM", ".EXE", ".BAT", ".CMD"]),
        (Some(".EXE;.CMD;;.PS1"), vec![".EXE", ".CMD", ".PS1"]),
    ];

    for (variable, expected) in table {
        let expected: Vec<OsString> = expected.into_iter().map(OsString::from).collect();
        assert_eq!(pathext(variable.map(OsStr::new)), expected);
    }
}

#[test]
fn a_bare_name_resolves_to_the_first_path_entry_holding_it_with_an_extension() {
    let first = TempDir::new().expect("a directory");
    let second = TempDir::new().expect("a directory");
    shim(first.path(), "claude.ps1");
    let found = shim(second.path(), "claude.cmd");
    shim(second.path(), "claude.bat");
    let extensions = [".exe", ".cmd", ".bat"].map(OsString::from);

    let resolved = search(
        Path::new("claude"),
        &environment(&[first.path(), second.path()]),
        &extensions,
    );

    assert_eq!(resolved, Some(found));
}

#[test]
fn a_name_on_no_path_entry_is_not_found() {
    let empty = TempDir::new().expect("a directory");
    let extensions = [".exe", ".cmd"].map(OsString::from);

    let resolved = search(
        Path::new("claude"),
        &environment(&[empty.path()]),
        &extensions,
    );

    assert_eq!(resolved, None);
}

#[test]
fn a_program_given_with_a_directory_is_used_as_given() {
    let dir = TempDir::new().expect("a directory");
    let program = dir.path().join("claude");
    shim(dir.path(), "claude.cmd");

    assert_eq!(
        find_executable(&program, &environment(&[dir.path()])),
        program
    );
}
