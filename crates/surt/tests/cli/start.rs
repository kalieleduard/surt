use std::time::{Duration, Instant};

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::scripts::{build, crash_in, limit_failure, review, ship, spec, tasks};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SHORT_SHA_CHARS: usize = 7;
const BLOCKED_EXIT_CODE: i32 = 4;
const RESET_DELAY_SECONDS: i64 = 3600;
const E2E_CEILING: Duration = Duration::from_secs(60);

pub fn feature_name() -> String {
    format!("{}-add-login", Zoned::now().date().strftime("%Y%m%d"))
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    run.stderr
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn count(sandbox: &Sandbox, table: &str) -> String {
    sandbox
        .texts(&format!("SELECT CAST(count(*) AS TEXT) FROM {table}"))
        .concat()
}

fn project(scripts: &[String]) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(scripts);
    sandbox
}

fn short(sha: &str) -> &str {
    &sha[..SHORT_SHA_CHARS]
}

fn text(sandbox: &Sandbox, sql: &str) -> String {
    sandbox.texts(sql).concat()
}

fn rev_parse(sandbox: &Sandbox, reference: &str) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["rev-parse", reference])
        .trim()
        .to_owned()
}

fn merged_as(sandbox: &Sandbox, item: &str) -> String {
    text(
        sandbox,
        &format!(
            "SELECT b.merged_as_sha FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
             WHERE w.item_key = '{item}'"
        ),
    )
}

#[test]
fn the_brief_delivers_a_feature_from_its_description_to_the_integration_branch() {
    let clock = Instant::now();
    let sandbox = project(&[
        spec(1, "# Spec 1\n"),
        spec(2, "# Spec 2 with OAuth\n"),
        tasks(1, &["A", "B"]),
        build("A", 1, 1, "login.txt", "login v1\n"),
        review("A", 1, "changes_requested"),
        build("A", 2, 1, "login.txt", "login v2\n"),
        review("A", 2, "approved"),
        crash_in("B/build", 1, 1),
        build("B", 1, 2, "logout.txt", "logout\n"),
        review("B", 1, "approved"),
        ship(),
    ]);
    let name = feature_name();
    let base = rev_parse(&sandbox, "HEAD");
    let base_branch = sandbox
        .git_output(&sandbox.repo(), &["symbolic-ref", "--short", "HEAD"])
        .trim()
        .to_owned();

    ok(sandbox.feature(&["add login"]));
    ok(surt(&sandbox, &["reject", &name, "-m", "needs OAuth"]));
    ok(sandbox.feature(&[&name]));
    ok(surt(&sandbox, &["approve", &name]));
    ok(surt(&sandbox, &["plan", &name]));
    ok(surt(&sandbox, &["approve", &name]));
    let started = ok(surt(&sandbox, &["start", &name]));

    let branch = format!("surt/{name}");
    let tip = rev_parse(&sandbox, &branch);
    for line in [
        format!("✔ task A merged ({})\n", short(&merged_as(&sandbox, "A"))),
        format!("✔ task B merged ({})\n", short(&merged_as(&sandbox, "B"))),
        "↻ task B build attempt 2\n".to_owned(),
        "● task A build iter 2 started\n".to_owned(),
    ] {
        assert!(started.contains(&line), "{line}\n{started}");
    }
    assert!(
        started.ends_with(&format!(
            "✔ shipper delivered — {branch} @ {}\n✔ run succeeded\n",
            short(&tip)
        )),
        "{started}"
    );
    assert_eq!(rev_parse(&sandbox, "HEAD"), base);
    assert_eq!(rev_parse(&sandbox, &base_branch), base);
    assert_eq!(
        sandbox.git_output(&sandbox.repo(), &["for-each-ref", "refs/remotes"]),
        ""
    );
    assert_eq!(
        sandbox.git_output(&sandbox.repo(), &["show", &format!("{tip}:login.txt")]),
        "login v2\n"
    );
    assert_eq!(
        sandbox.git_output(&sandbox.repo(), &["show", &format!("{tip}:logout.txt")]),
        "logout\n"
    );
    assert_eq!(
        text(
            &sandbox,
            "SELECT b.tip_sha || ' ' || w.state FROM run_branch b \
             JOIN workflow_run w ON w.id = b.workflow_run_id WHERE w.parent_step_run_id IS NULL"
        ),
        format!("{tip} succeeded")
    );
    assert_eq!(
        sandbox.texts(
            "SELECT w.item_key || ' ' || s.iteration || ' ' || s.step_key || ' ' || coalesce(s.outcome, s.state) \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.item_key IS NOT NULL AND s.step_key IN ('build', 'review', 'integrate') \
             ORDER BY w.item_key, s.iteration, s.created_at"
        ),
        [
            "A 1 build done",
            "A 1 review changes_requested",
            "A 1 integrate skipped",
            "A 2 build done",
            "A 2 review approved",
            "A 2 integrate merged",
            "B 1 build done",
            "B 1 review approved",
            "B 1 integrate merged",
        ]
    );
    assert_eq!(count(&sandbox, "execution WHERE state = 'failed'"), "1");
    assert_eq!(
        text(
            &sandbox,
            "SELECT s.outcome FROM step_run s WHERE s.step_key = 'ship'"
        ),
        "done"
    );
    assert!(
        clock.elapsed() < E2E_CEILING,
        "the E2E took {:?}",
        clock.elapsed()
    );
}

pub fn planned(name: &str, scripts: &[String]) -> Sandbox {
    let mut all = vec![spec(1, "# Spec 1\n")];
    all.extend_from_slice(scripts);
    let sandbox = project(&all);
    ok(sandbox.feature(&["add login"]));
    ok(surt(&sandbox, &["approve", name]));
    ok(surt(&sandbox, &["plan", name]));
    ok(surt(&sandbox, &["approve", name]));
    sandbox
}

fn last_failure(sandbox: &Sandbox, item: &str) -> (String, String) {
    let rows = sandbox.texts(&format!(
        "SELECT e.id || char(10) || e.error_message FROM execution e \
         JOIN step_run s ON s.id = e.step_run_id JOIN workflow_run w ON w.id = s.workflow_run_id \
         WHERE w.item_key = '{item}' AND e.state = 'failed' ORDER BY e.attempt DESC LIMIT 1"
    ));
    let (id, message) = rows[0].split_once('\n').expect("an id and a message");
    (id.to_owned(), message.to_owned())
}

#[test]
fn blocked_tasks_stop_start_with_exit_4_naming_each_last_error_and_log() {
    let name = feature_name();
    let resets_at = Zoned::now().timestamp().as_second() + RESET_DELAY_SECONDS;
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A", "B", "C"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            crash_in("B/build", 1, 1),
            crash_in("B/build", 1, 2),
            crash_in("B/build", 1, 3),
            limit_failure("C/build", resets_at),
        ],
    );

    let run = surt(&sandbox, &["start", &name]);

    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    let (b_execution, b_message) = last_failure(&sandbox, "B");
    let (c_execution, c_message) = last_failure(&sandbox, "C");
    let resets = Timestamp::from_second(resets_at)
        .expect("a timestamp")
        .to_zoned(TimeZone::system())
        .strftime("%H:%M")
        .to_string();
    assert!(
        run.stderr.ends_with(&format!(
            "⛔ task B build blocked: attempts exhausted\n  \
             attempt 3 failed (exit_code): {b_message}\n  \
             log: .surt/logs/{name}/{b_execution}/stream.jsonl\n\
             ⛔ task C build blocked: cannot be retried\n  \
             attempt 1 failed (rate_limit): {c_message} — resets {resets}\n  \
             log: .surt/logs/{name}/{c_execution}/stream.jsonl\n  \
             resume: surt start {name}\n"
        )),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains(&format!(
            "✔ task A merged ({})\n",
            short(&merged_as(&sandbox, "A"))
        )),
        "{}",
        run.stderr
    );
    assert_eq!(count(&sandbox, "step_run WHERE step_key = 'ship'"), "1");
    assert_eq!(
        text(
            &sandbox,
            "SELECT state FROM step_run WHERE step_key = 'ship'"
        ),
        "pending"
    );
}

#[test]
fn start_refuses_until_the_manifest_is_approved() {
    let sandbox = project(&[spec(1, "# Spec 1\n"), tasks(1, &["A"])]);
    let name = feature_name();
    ok(sandbox.feature(&["add login"]));
    ok(surt(&sandbox, &["approve", &name]));
    ok(surt(&sandbox, &["plan", &name]));
    let executions = count(&sandbox, "execution");

    let run = surt(&sandbox, &["start", &name]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        format!(
            "✖ manifest not approved yet (manifest rev 1 awaiting approval)\n  review manifest rev 1, then: surt approve {name}\n"
        )
    );
    assert_eq!(count(&sandbox, "execution"), executions);
}

#[test]
fn start_refuses_while_the_integration_branch_was_moved_and_resumes_once_restored() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A", "B"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            crash_in("B/build", 1, 1),
            crash_in("B/build", 1, 2),
            crash_in("B/build", 1, 3),
            build("B", 1, 4, "logout.txt", "logout\n"),
            review("B", 1, "approved"),
            ship(),
        ],
    );
    let blocked = surt(&sandbox, &["start", &name]);
    assert_eq!(blocked.code, Some(BLOCKED_EXIT_CODE), "{}", blocked.stderr);
    let branch = format!("surt/{name}");
    let tip = rev_parse(&sandbox, &branch);
    let base = rev_parse(&sandbox, "HEAD");
    sandbox.git(&sandbox.repo(), &["branch", "--force", &branch, &base]);
    let executions = count(&sandbox, "execution");

    let refused = surt(&sandbox, &["start", &name]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ {branch} moved outside surt (expected {}, found {})\n  \
             fix: git branch -f {branch} {tip}\n  \
             resume: surt start {name}\n",
            short(&tip),
            short(&base)
        )
    );
    assert_eq!(count(&sandbox, "execution"), executions);

    sandbox.git(&sandbox.repo(), &["branch", "--force", &branch, &tip]);
    let resumed = ok(surt(&sandbox, &["start", &name]));

    assert!(resumed.ends_with("✔ run succeeded\n"), "{resumed}");
}
