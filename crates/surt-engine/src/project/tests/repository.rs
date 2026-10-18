use std::process::Command;

use super::{Sandbox, git};
use crate::project::{Commit, Project};

fn head_sha(sandbox: &Sandbox) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(sandbox.repo())
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

async fn project_on_branch(sandbox: &Sandbox) -> Project {
    sandbox.commit_something();
    git(&sandbox.repo(), &["checkout", "--quiet", "-b", "feature-x"]);
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    sandbox
        .open(&sandbox.repo())
        .await
        .expect("the project opens")
}

#[tokio::test(flavor = "multi_thread")]
async fn head_on_a_branch_resolves_to_its_commit_and_short_branch_name() {
    let sandbox = Sandbox::with_repository();
    let project = project_on_branch(&sandbox).await;

    let head = project.resolve_commit("HEAD").await.expect("git runs");
    let branch = project.resolve_commit("feature-x").await.expect("git runs");

    let expected = Commit {
        sha: head_sha(&sandbox),
        branch: Some("feature-x".to_owned()),
    };
    assert_eq!(head, Some(expected.clone()));
    assert_eq!(branch, Some(expected));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tag_or_a_detached_head_resolves_to_a_commit_without_a_branch() {
    let sandbox = Sandbox::with_repository();
    let project = project_on_branch(&sandbox).await;
    git(
        &sandbox.repo(),
        &[
            "-c",
            "tag.gpgSign=false",
            "tag",
            "--annotate",
            "v1",
            "-m",
            "v1",
        ],
    );
    let sha = head_sha(&sandbox);

    let tag = project.resolve_commit("v1").await.expect("git runs");
    git(&sandbox.repo(), &["checkout", "--quiet", "--detach"]);
    let detached = project.resolve_commit("HEAD").await.expect("git runs");

    let expected = Some(Commit { sha, branch: None });
    assert_eq!(tag, expected);
    assert_eq!(detached, expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_reference_or_an_option_resolves_to_nothing() {
    let sandbox = Sandbox::with_repository();
    let project = project_on_branch(&sandbox).await;

    for reference in ["no-such-branch", "--all", ""] {
        let resolved = project.resolve_commit(reference).await.expect("git runs");

        assert_eq!(resolved, None, "{reference:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_actor_is_the_email_of_the_git_identity() {
    let sandbox = Sandbox::with_repository();
    let project = project_on_branch(&sandbox).await;

    let actor = project.actor().await.expect("git runs");

    assert_eq!(actor, "tester@example.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn head_of_a_repository_without_commits_resolves_to_nothing() {
    let sandbox = Sandbox::with_repository();
    sandbox.init(&sandbox.repo()).await.expect("init succeeds");
    let project = sandbox
        .open(&sandbox.repo())
        .await
        .expect("the project opens");

    let resolved = project.resolve_commit("HEAD").await.expect("git runs");

    assert_eq!(resolved, None);
}
