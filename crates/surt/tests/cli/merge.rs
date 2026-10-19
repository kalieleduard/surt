use std::fs;

use crate::scripts::{build, review, ship, spec, tasks};
use crate::start::feature_name;
use crate::support::{Run, Sandbox};

const SHORT_SHA_CHARS: usize = 7;

pub(super) fn merging_project() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.write_config("[git]\nmerge_into_base = \"ff-only\"\n");
    sandbox.script(&[
        spec(1, "# Spec\n"),
        tasks(1, &["A"]),
        build("A", 1, 1, "login.txt", "login\n"),
        review("A", 1, "approved"),
        ship(),
    ]);
    sandbox
}

pub(super) fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    let run = sandbox.surt(&sandbox.repo(), args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run
}

pub(super) fn planned(sandbox: &Sandbox) -> String {
    let name = feature_name();
    let created = sandbox.run_feature("add login", &[]);
    assert_eq!(created.code, Some(0), "{}", created.stderr);
    surt(sandbox, &["approve", &name]);
    surt(sandbox, &["plan", &name]);
    surt(sandbox, &["approve", &name]);
    name
}

pub(super) fn sha(sandbox: &Sandbox, reference: &str) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["rev-parse", reference])
        .trim()
        .to_owned()
}

pub(super) fn base_branch(sandbox: &Sandbox) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["symbolic-ref", "--short", "HEAD"])
        .trim()
        .to_owned()
}

pub(super) fn commit_news(sandbox: &Sandbox) {
    fs::write(sandbox.repo().join("NEWS.md"), "news\n").expect("a file");
    sandbox.git(&sandbox.repo(), &["add", "NEWS.md"]);
    sandbox.git(&sandbox.repo(), &["commit", "--quiet", "-m", "news"]);
}

pub(super) fn short(sha: &str) -> &str {
    &sha[..SHORT_SHA_CHARS]
}

fn refuse_moving(sandbox: &Sandbox, branch: &str) {
    let hook = sandbox
        .repo()
        .join(".git")
        .join("hooks")
        .join("reference-transaction");
    let script =
        format!("#!/bin/sh\n[ \"$1\" = prepared ] || exit 0\n! grep -q ' refs/heads/{branch}$'\n");
    fs::write(&hook, script).expect("the hook is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))
            .expect("the hook is executable");
    }
}

#[test]
fn a_base_branch_checked_out_nowhere_is_fast_forwarded_after_the_shipper() {
    let sandbox = merging_project();
    let name = planned(&sandbox);
    let main = base_branch(&sandbox);
    let base = sha(&sandbox, &main);
    sandbox.git(&sandbox.repo(), &["checkout", "--quiet", "--detach"]);

    let started = surt(&sandbox, &["start", &name]);

    let tip = sha(&sandbox, &format!("surt/{name}"));
    assert_eq!(sha(&sandbox, &main), tip);
    assert!(
        started.stderr.contains(&format!(
            "✔ shipper delivered — surt/{name} @ {}\n✔ merged into {main} (fast-forward {} → {})\n✔ run succeeded\n",
            short(&tip),
            short(&base),
            short(&tip)
        )),
        "{}",
        started.stderr
    );
}

#[test]
fn a_fast_forward_interrupted_in_the_checked_out_worktree_blocks_with_how_to_finish_or_undo_it() {
    let sandbox = merging_project();
    let name = planned(&sandbox);
    let main = base_branch(&sandbox);
    let base = sha(&sandbox, &main);
    let worktree = sandbox
        .git_output(&sandbox.repo(), &["rev-parse", "--show-toplevel"])
        .trim()
        .to_owned();
    let argument = if worktree.contains(' ') {
        format!("\"{worktree}\"")
    } else {
        worktree.clone()
    };
    refuse_moving(&sandbox, &main);

    let started = sandbox.surt(&sandbox.repo(), &["start", &name]);

    assert_eq!(started.code, Some(4), "{}", started.stderr);
    assert_eq!(sha(&sandbox, &main), base);
    let tip = sha(&sandbox, &format!("surt/{name}"));
    assert!(
        started.stderr.ends_with(&format!(
            "\u{26d4} merge into {main} was interrupted; {worktree} may be partially updated\n  \
             finish it:  git -C {argument} merge --ff-only {}\n  \
             or undo it: git -C {argument} reset --hard {}   (discards changes)\n  \
             then: surt start {name}\n",
            short(&tip),
            short(&base)
        )),
        "{}",
        started.stderr
    );
}

#[test]
fn a_base_branch_that_moved_is_left_for_the_developer_to_merge() {
    let sandbox = merging_project();
    let name = planned(&sandbox);
    let main = base_branch(&sandbox);
    let base = sha(&sandbox, &main);
    commit_news(&sandbox);
    let moved = sha(&sandbox, &main);
    sandbox.git(&sandbox.repo(), &["checkout", "--quiet", "--detach"]);

    let started = surt(&sandbox, &["start", &name]);

    assert_eq!(sha(&sandbox, &main), moved);
    assert!(
        started.stderr.contains(&format!(
            "⚠ not merged: {main} moved since base ({} → {}); merge surt/{name} yourself\n✔ run succeeded\n",
            short(&base),
            short(&moved)
        )),
        "{}",
        started.stderr
    );
}
