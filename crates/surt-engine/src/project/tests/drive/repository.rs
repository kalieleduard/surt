use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

use super::super::fixtures::Catalog;
use super::super::fixtures::{ProjectDir, SUBJECT, initial, new_run};
use super::fixtures::Feature;
use crate::project::{Config, MergeIntoBase};

const PROJECT_ID: &str = "test-project";

pub(super) fn git(cwd: &Path, args: &[&str]) -> String {
    let output = git_output(cwd, args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub(super) fn git_succeeds(cwd: &Path, args: &[&str]) -> bool {
    git_output(cwd, args).status.success()
}

fn git_output(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs")
}

pub(super) fn commit_file(cwd: &Path, name: &str, text: &str) -> String {
    fs::write(cwd.join(name), text).expect("the file is written");
    git(cwd, &["add", name]);
    git(cwd, &["commit", "--quiet", "-m", name]);
    git(cwd, &["rev-parse", "HEAD"])
}

impl Feature {
    pub(super) async fn on_repository(catalog: Catalog, workflow: &str, config: Config) -> Feature {
        Feature::on_base(catalog, workflow, config, Some("main")).await
    }

    pub(super) async fn on_base(
        catalog: Catalog,
        workflow: &str,
        config: Config,
        base_ref: Option<&str>,
    ) -> Feature {
        Feature::on_base_choosing(catalog, workflow, config, base_ref, None).await
    }

    pub(super) async fn on_base_choosing(
        catalog: Catalog,
        workflow: &str,
        config: Config,
        base_ref: Option<&str>,
        merge_into_base: Option<MergeIntoBase>,
    ) -> Feature {
        let dir = ProjectDir::new();
        let root = dir.path();
        git(&root, &["init", "--quiet", "--initial-branch=main"]);
        git(&root, &["config", "user.name", "Surt Tester"]);
        git(&root, &["config", "user.email", "tester@example.com"]);
        git(&root, &["config", "core.autocrlf", "false"]);
        fs::write(root.join(".git/info/exclude"), ".surt/\n").expect("the exclude file");
        let base = commit_file(&root, "README.md", "hello\n");
        let project = dir.project(catalog, config).await;
        let mut new_run = new_run(workflow);
        new_run.merge_into_base = merge_into_base;
        let base_json = json!({ "ref": base_ref, "sha": base }).to_string();
        new_run.initial_artifacts[1] = initial("base", "base.json", base_json.as_bytes());
        let run = project
            .create_run(new_run)
            .await
            .expect("the run is created")
            .run;
        Feature { dir, project, run }
    }

    pub(super) fn root(&self) -> PathBuf {
        self.dir.path()
    }

    pub(super) fn git(&self, args: &[&str]) -> String {
        git(&self.root(), args)
    }

    pub(super) fn sha(&self, reference: &str) -> Option<String> {
        let peeled = format!("{reference}^{{commit}}");
        git_succeeds(&self.root(), &["rev-parse", "--verify", "--quiet", &peeled])
            .then(|| self.git(&["rev-parse", &peeled]))
    }

    pub(super) fn base(&self) -> String {
        self.git(&["rev-parse", "main"])
    }

    pub(super) fn listed_worktree(&self, path: &Path) -> Option<String> {
        let listed = self.git(&["worktree", "list", "--porcelain"]);
        listed.split("\n\n").find_map(|record| {
            let (first, fields) = record.split_once('\n')?;
            let listed_path = first.strip_prefix("worktree ").map(Path::new)?;
            (listed_path == path).then(|| fields.to_owned())
        })
    }

    pub(super) fn worktree(&self, key: &str) -> PathBuf {
        self.dir
            .home()
            .join(".surt")
            .join("worktrees")
            .join(PROJECT_ID)
            .join(SUBJECT)
            .join(key)
    }
}
