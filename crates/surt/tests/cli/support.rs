use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

pub const REFUSED_EXIT_CODE: i32 = 3;
pub const INTERNAL_ERROR_EXIT_CODE: i32 = 1;

const IDENTITY_CONFIG: &str = "[user]\n\tname = Surt Tester\n\temail = tester@example.com\n";
const NO_IDENTITY_CONFIG: &str = "[user]\n\tuseConfigOnly = true\n";

const INHERITED_VARIABLES: [&str; 11] = [
    "EMAIL",
    "GIT_AUTHOR_NAME",
    "GIT_AUTHOR_EMAIL",
    "GIT_COMMITTER_NAME",
    "GIT_COMMITTER_EMAIL",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "SURT_LOG",
    "NO_COLOR",
    "CLICOLOR",
    "CLICOLOR_FORCE",
];

pub struct Sandbox {
    root: TempDir,
}

pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl From<Output> for Run {
    fn from(output: Output) -> Self {
        Run {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

impl Sandbox {
    pub fn new() -> Self {
        let sandbox = Sandbox {
            root: TempDir::new().expect("a temporary directory"),
        };
        fs::create_dir_all(sandbox.home()).expect("the home directory");
        fs::create_dir_all(sandbox.repo()).expect("the repository directory");
        sandbox.write_git_config(IDENTITY_CONFIG);
        sandbox
    }

    pub fn with_repository() -> Self {
        let sandbox = Self::new();
        sandbox.git(&sandbox.repo(), &["init", "--quiet"]);
        sandbox
    }

    pub fn without_identity(&self) {
        self.write_git_config(NO_IDENTITY_CONFIG);
    }

    pub fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    pub fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    pub fn dot_surt(&self) -> PathBuf {
        self.repo().join(".surt")
    }

    pub fn subdirectory(&self, name: &str) -> PathBuf {
        let path = self.repo().join(name);
        fs::create_dir_all(&path).expect("the subdirectory");
        path
    }

    fn git_config(&self) -> PathBuf {
        self.root.path().join("gitconfig")
    }

    fn write_git_config(&self, contents: &str) {
        fs::write(self.git_config(), contents).expect("the git config");
    }

    fn command(&self, program: &Path, cwd: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(cwd)
            .env("HOME", self.home())
            .env("USERPROFILE", self.home())
            .env("GIT_CONFIG_GLOBAL", self.git_config())
            .env("GIT_CONFIG_NOSYSTEM", "1");
        for variable in INHERITED_VARIABLES {
            command.env_remove(variable);
        }
        command
    }

    pub fn git(&self, cwd: &Path, args: &[&str]) {
        self.git_output(cwd, args);
    }

    pub fn git_output(&self, cwd: &Path, args: &[&str]) -> String {
        let output = self
            .command(Path::new("git"), cwd)
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    pub fn commit_something(&self) {
        fs::write(self.repo().join("README.md"), "hello\n").expect("a file");
        self.git(&self.repo(), &["add", "README.md"]);
        self.git(&self.repo(), &["commit", "--quiet", "-m", "first"]);
    }

    pub fn write_config(&self, contents: &str) {
        fs::write(self.dot_surt().join("config.toml"), contents).expect("the config");
    }

    pub fn surt(&self, cwd: &Path, args: &[&str]) -> Run {
        self.surt_with(cwd, args, &[])
    }

    pub fn surt_with(&self, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
        self.command(Path::new(env!("CARGO_BIN_EXE_surt")), cwd)
            .args(args)
            .envs(env.iter().copied())
            .output()
            .expect("the surt binary runs")
            .into()
    }

    pub fn init(&self) -> Run {
        let run = self.surt(&self.repo(), &["init"]);
        assert_eq!(run.code, Some(0), "surt init failed: {}", run.stderr);
        run
    }
}
