use std::fs;

use crate::scripts::{build, review, ship, spec, tasks};
use crate::start::feature_name;
use crate::support::{Run, Sandbox};

const SHORT_SHA_CHARS: usize = 7;

fn merging_project() -> Sandbox {
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

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    let run = sandbox.surt(&sandbox.repo(), args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run
}

fn planned(sandbox: &Sandbox) -> String {
    let name = feature_name();
    let created = sandbox.feature(&["add login"]);
    assert_eq!(created.code, Some(0), "{}", created.stderr);
    surt(sandbox, &["approve", &name]);
    surt(sandbox, &["plan", &name]);
    surt(sandbox, &["approve", &name]);
    name
}

fn sha(sandbox: &Sandbox, reference: &str) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["rev-parse", reference])
        .trim()
        .to_owned()
}

fn base_branch(sandbox: &Sandbox) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["symbolic-ref", "--short", "HEAD"])
        .trim()
        .to_owned()
}

fn commit_news(sandbox: &Sandbox) {
    fs::write(sandbox.repo().join("NEWS.md"), "news\n").expect("a file");
    sandbox.git(&sandbox.repo(), &["add", "NEWS.md"]);
    sandbox.git(&sandbox.repo(), &["commit", "--quiet", "-m", "news"]);
}

fn short(sha: &str) -> &str {
    &sha[..SHORT_SHA_CHARS]
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
