use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{AssertSqlSafe, SqlitePool};
use tempfile::TempDir;

pub const REFUSED_EXIT_CODE: i32 = 3;
pub const INTERNAL_ERROR_EXIT_CODE: i32 = 1;

const RUNTIME_VARIABLE: &str = "SURT_RUNTIME";
#[cfg(unix)]
const SYSTEM_DIRECTORIES: [&str; 2] = ["/usr/bin", "/bin"];

const IDENTITY_CONFIG: &str = "[user]\n\tname = Surt Tester\n\temail = tester@example.com\n";
const NO_IDENTITY_CONFIG: &str = "[user]\n\tuseConfigOnly = true\n";

const INHERITED_VARIABLES: [&str; 14] = [
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
    "LC_ALL",
    "LC_CTYPE",
    "LANG",
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

    fn script_path(&self) -> PathBuf {
        self.root.path().join("script.toml")
    }

    pub fn script(&self, entries: &[String]) {
        fs::write(self.script_path(), entries.concat()).expect("the Fake Runtime script");
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
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(RUNTIME_VARIABLE, self.script_path());
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

    #[cfg(unix)]
    pub fn tools(&self) -> PathBuf {
        self.root.path().join("tools")
    }

    #[cfg(unix)]
    pub fn surt_on_claude(&self, args: &[&str]) -> Run {
        assert_no_real_claude();
        let tools = self.tools();
        fs::create_dir_all(&tools).expect("the tools directory");
        let git = tools.join("git");
        if !git.exists() {
            std::os::unix::fs::symlink(on_parent_path("git"), git).expect("a git link");
        }
        let path = std::env::join_paths(
            std::iter::once(tools).chain(SYSTEM_DIRECTORIES.iter().map(PathBuf::from)),
        )
        .expect("a joinable PATH");
        self.command(Path::new(env!("CARGO_BIN_EXE_surt")), &self.repo())
            .args(args)
            .env_remove(RUNTIME_VARIABLE)
            .env("PATH", path)
            .output()
            .expect("the surt binary runs")
            .into()
    }

    pub fn spawn(&self, args: &[&str]) -> Child {
        self.command(Path::new(env!("CARGO_BIN_EXE_surt")), &self.repo())
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the surt binary starts")
    }

    pub fn init(&self) -> Run {
        let run = self.surt(&self.repo(), &["init"]);
        assert_eq!(run.code, Some(0), "surt init failed: {}", run.stderr);
        run
    }

    pub fn feature(&self, args: &[&str]) -> Run {
        let mut all = vec!["feature"];
        all.extend_from_slice(args);
        self.surt(&self.repo(), &all)
    }

    pub fn texts(&self, sql: &str) -> Vec<String> {
        let options = SqliteConnectOptions::new()
            .filename(self.database())
            .read_only(true);
        block_on(async {
            let pool = SqlitePool::connect_with(options)
                .await
                .expect("the database opens read-only");
            let rows = sqlx::query_scalar::<_, String>(AssertSqlSafe(sql.to_owned()))
                .fetch_all(&pool)
                .await
                .expect("the rows are readable");
            pool.close().await;
            rows
        })
    }

    pub fn execute(&self, sql: &str) {
        let options = SqliteConnectOptions::new().filename(self.database());
        block_on(async {
            let pool = SqlitePool::connect_with(options)
                .await
                .expect("the database opens");
            sqlx::query(AssertSqlSafe(sql.to_owned()))
                .execute(&pool)
                .await
                .expect("the statement runs");
            pool.close().await;
        });
    }

    fn database(&self) -> PathBuf {
        self.dot_surt().join("surt.db")
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}

#[cfg(unix)]
fn on_parent_path(program: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("a PATH");
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} is not on PATH"))
}

#[cfg(unix)]
fn assert_no_real_claude() {
    for directory in SYSTEM_DIRECTORIES {
        assert!(
            !Path::new(directory).join("claude").exists(),
            "a real Claude Code in {directory} would run instead of the stub"
        );
    }
}
