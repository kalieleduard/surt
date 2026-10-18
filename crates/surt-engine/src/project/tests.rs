mod active;
mod agents;
mod checked;
mod created;
#[cfg(feature = "test-support")]
mod drive;
mod edits;
mod fixtures;
mod flows;
mod frozen;
mod local_assets;
mod override_refusals;
mod overrides;
mod parsing;
mod per_item;
mod policy;
mod references;
mod repository;
mod resolving;
mod runs;
mod shapes;
mod subjects;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use sqlx::sqlite::SqliteConnectOptions;
use tempfile::TempDir;

use super::discovery::SurtHome;
use super::{Access, Catalog, Config, Error, FileState, InitFile, Project, Refusal, init};

struct Sandbox {
    dir: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Sandbox {
            dir: TempDir::new().expect("a temporary directory"),
        };
        fs::create_dir_all(sandbox.home()).expect("the home directory");
        sandbox
    }

    fn with_repository() -> Self {
        let sandbox = Self::new();
        sandbox.create_repository(&sandbox.repo());
        sandbox
    }

    fn path(&self) -> PathBuf {
        dunce::canonicalize(self.dir.path()).expect("a canonical temporary directory")
    }

    fn home(&self) -> PathBuf {
        self.path().join("home")
    }

    fn surt_home(&self) -> SurtHome {
        SurtHome::at(&self.home())
    }

    fn repo(&self) -> PathBuf {
        self.path().join("repo")
    }

    fn directory(&self, relative: &str) -> PathBuf {
        let path = self.path().join(relative);
        fs::create_dir_all(&path).expect("the directory");
        path
    }

    fn create_repository(&self, path: &Path) {
        fs::create_dir_all(path).expect("the repository directory");
        git(path, &["init", "--quiet"]);
        git(path, &["config", "user.name", "Surt Tester"]);
        git(path, &["config", "user.email", "tester@example.com"]);
    }

    fn commit_something(&self) {
        fs::write(self.repo().join("README.md"), "hello\n").expect("a file");
        git(&self.repo(), &["add", "README.md"]);
        git(&self.repo(), &["commit", "--quiet", "-m", "first"]);
    }

    async fn init(&self, cwd: &Path) -> Result<super::Initialized, Error> {
        init::run(cwd, &self.surt_home()).await
    }

    async fn open(&self, cwd: &Path) -> Result<Project, Error> {
        Project::open_with(cwd, &self.surt_home(), Catalog::new(), Access::ReadWrite).await
    }

    fn write_global_config(&self, contents: &str) -> PathBuf {
        let dot_surt = self.home().join(".surt");
        fs::create_dir_all(&dot_surt).expect("the global .surt directory");
        let path = dot_surt.join("config.toml");
        fs::write(&path, contents).expect("the global config");
        path
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn refusal<T>(result: Result<T, Error>) -> Refusal {
    match result {
        Err(Error::Refused(refusal)) => refusal,
        Err(other) => panic!("expected a refusal, got {other:?}"),
        Ok(_) => panic!("expected a refusal, got success"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_initialized_at_the_toplevel_opens_from_any_subdirectory() {
    let sandbox = Sandbox::with_repository();
    let deep = sandbox.directory("repo/src/deep");

    let initialized = sandbox.init(&deep).await.expect("init succeeds");
    let project = sandbox.open(&deep).await.expect("the project opens");

    assert_eq!(initialized.dot_surt, sandbox.repo().join(".surt"));
    assert!(initialized.from_subdirectory);
    assert_eq!(initialized.identity, "tester@example.com");
    assert_eq!(project.root(), sandbox.repo());
    assert!(project.id().starts_with("repo-"), "{}", project.id());
    assert_eq!(project.config(), &Config::default());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_first_init_creates_every_file_and_a_rerun_keeps_them_all() {
    let sandbox = Sandbox::with_repository();
    let first = sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let config = sandbox.repo().join(".surt/config.toml");
    fs::write(&config, "max_attempts = 7\n").expect("an edited config");
    let first_id = sandbox
        .open(&sandbox.repo())
        .await
        .expect("the project opens")
        .id()
        .to_owned();

    let second = sandbox.init(&sandbox.repo()).await.expect("rerun succeeds");

    assert!(!first.from_subdirectory);
    assert_eq!(
        first.files,
        [
            (InitFile::Gitignore, FileState::Created),
            (InitFile::Config, FileState::Created),
            (InitFile::Database, FileState::Created),
        ]
    );
    assert!(!first.was_already_initialized());
    assert!(second.was_already_initialized());
    assert_eq!(
        fs::read_to_string(&config).expect("the config"),
        "max_attempts = 7\n"
    );
    let reopened = sandbox.open(&sandbox.repo()).await.expect("reopens");
    assert_eq!(reopened.id(), first_id);
    assert_eq!(reopened.config().max_attempts.get(), 7);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_gitignore_is_rewritten_on_rerun() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let gitignore = sandbox.repo().join(".surt/.gitignore");
    fs::remove_file(&gitignore).expect("the gitignore is removed");

    let rerun = sandbox.init(&sandbox.repo()).await.expect("rerun succeeds");

    assert_eq!(rerun.files[0], (InitFile::Gitignore, FileState::Created));
    assert!(gitignore.is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn init_from_a_linked_worktree_creates_the_project_in_the_main_worktree() {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    let linked = sandbox.path().join("linked");
    git(
        &sandbox.repo(),
        &["worktree", "add", "--quiet", &linked.to_string_lossy()],
    );

    let initialized = sandbox.init(&linked).await.expect("init succeeds");

    assert_eq!(initialized.dot_surt, sandbox.repo().join(".surt"));
    assert!(initialized.from_subdirectory);
    assert!(!linked.join(".surt").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn init_outside_a_repository_is_refused() {
    let sandbox = Sandbox::new();
    let outside = sandbox.directory("plain");

    assert_eq!(
        refusal(sandbox.init(&outside).await),
        Refusal::NotAGitRepository { path: outside }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bare_repository_is_refused() {
    let sandbox = Sandbox::new();
    let bare = sandbox.directory("bare.git");
    git(&bare, &["init", "--quiet", "--bare"]);

    assert!(matches!(
        refusal(sandbox.init(&bare).await),
        Refusal::BareRepository { .. }
    ));
    assert!(matches!(
        refusal(sandbox.open(&bare).await),
        Refusal::BareRepository { .. }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_an_uninitialized_repository_points_at_its_toplevel() {
    let sandbox = Sandbox::with_repository();
    let deep = sandbox.directory("repo/src");

    assert_eq!(
        refusal(sandbox.open(&deep).await),
        Refusal::NotAProject {
            root: sandbox.repo()
        }
    );
    assert!(!sandbox.repo().join(".surt").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_outside_any_repository_is_not_a_project() {
    let sandbox = Sandbox::new();
    let outside = sandbox.directory("plain");

    assert_eq!(
        refusal(sandbox.open(&outside).await),
        Refusal::NotAProject { root: outside }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_surt_managed_worktree_is_refused_by_open_and_init() {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let managed = sandbox
        .home()
        .join(".surt/worktrees/repo-a1b2c3/20261017-add-login/A");
    fs::create_dir_all(managed.parent().expect("a parent")).expect("the worktrees directory");
    git(
        &sandbox.repo(),
        &["worktree", "add", "--quiet", &managed.to_string_lossy()],
    );
    let inside = managed.join("src");
    fs::create_dir_all(&inside).expect("a subdirectory");

    let expected = Refusal::InsideManagedWorktree {
        path: inside.clone(),
        project: Some(sandbox.repo()),
    };
    assert_eq!(refusal(sandbox.open(&inside).await), expected);
    assert_eq!(refusal(sandbox.init(&inside).await), expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_database_from_a_newer_surt_is_refused() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let database = sandbox.repo().join(".surt/surt.db");
    let pool = sqlx::SqlitePool::connect_with(SqliteConnectOptions::new().filename(&database))
        .await
        .expect("the database opens");
    sqlx::query("INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (9, 'from a newer surt', 1, x'00', 0)")
        .execute(&pool)
        .await
        .expect("the newer migration is recorded");
    pool.close().await;

    assert_eq!(
        refusal(sandbox.open(&sandbox.repo()).await),
        Refusal::SchemaTooNew {
            db: 9,
            supported: 1
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_config_is_refused_before_the_database_is_touched() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let config = sandbox.repo().join(".surt/config.toml");
    fs::write(&config, "max_atempts = 4\n").expect("a config with a typo");

    assert!(matches!(
        refusal(sandbox.open(&sandbox.repo()).await),
        Refusal::InvalidConfig { path, line: Some(1), .. } if path == config
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_global_config_the_project_opens_with_a_ten_second_grace_period() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");

    let project = sandbox
        .open(&sandbox.repo())
        .await
        .expect("the project opens");

    assert_eq!(project.stop_grace_period(), Duration::from_secs(10));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_global_config_in_the_home_directory_sets_the_grace_period() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    sandbox.write_global_config("stop_grace_period = \"45s\"\n");

    let project = sandbox
        .open(&sandbox.repo())
        .await
        .expect("the project opens");

    assert_eq!(project.stop_grace_period(), Duration::from_secs(45));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_global_config_refuses_to_open_the_project() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let global = sandbox.write_global_config("stop_grace_period = \"ten\"\n");

    assert!(matches!(
        refusal(sandbox.open(&sandbox.repo()).await),
        Refusal::InvalidConfig { path, line: Some(1), .. } if path == global
    ));
}
