use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

const PATH: &str = "PATH";
const PATHEXT: &str = "PATHEXT";
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";
const PATHEXT_SEPARATOR: char = ';';
const LAUNCHES_BY_EXTENSION: bool = cfg!(windows);

pub(crate) fn find_executable(program: &Path, environment: &[(OsString, OsString)]) -> PathBuf {
    let extensions = launchable_extensions(variable(environment, PATHEXT));
    if extensions.is_empty() || program.components().count() != 1 {
        return program.to_owned();
    }
    search(program, environment, &extensions).unwrap_or_else(|| program.to_owned())
}

fn launchable_extensions(pathext_variable: Option<&OsStr>) -> Vec<OsString> {
    if LAUNCHES_BY_EXTENSION {
        pathext(pathext_variable)
    } else {
        Vec::new()
    }
}

fn pathext(variable: Option<&OsStr>) -> Vec<OsString> {
    variable
        .and_then(OsStr::to_str)
        .unwrap_or(DEFAULT_PATHEXT)
        .split(PATHEXT_SEPARATOR)
        .filter(|extension| !extension.is_empty())
        .map(OsString::from)
        .collect()
}

fn search(
    program: &Path,
    environment: &[(OsString, OsString)],
    extensions: &[OsString],
) -> Option<PathBuf> {
    let directories = variable(environment, PATH)?;
    std::env::split_paths(directories).find_map(|directory| {
        extensions.iter().find_map(|extension| {
            let mut name = program.as_os_str().to_owned();
            name.push(extension);
            let candidate = directory.join(name);
            candidate.is_file().then_some(candidate)
        })
    })
}

fn variable<'a>(environment: &'a [(OsString, OsString)], name: &str) -> Option<&'a OsStr> {
    environment
        .iter()
        .find(|(key, _)| {
            key.to_str()
                .is_some_and(|key| key.eq_ignore_ascii_case(name))
        })
        .map(|(_, value)| value.as_os_str())
}

#[cfg(test)]
mod tests;
