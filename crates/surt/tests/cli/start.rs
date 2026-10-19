use std::time::{Duration, Instant};

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::scripts::{build, crash_in, limit_failure_in, review, ship, spec, tasks};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SHORT_SHA_CHARS: usize = 7;
const BLOCKED_EXIT_CODE: i32 = 4;
const RESET_DELAY_SECONDS: i64 = 2;
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
            "✔ shipper delivered — {branch} @ {}\n✔ run succeeded\n\
             ↻ cleanup 2 worktrees · 2 task branches · working copy\n",
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
    assert_cleaned(&sandbox, &name);
    assert!(
        clock.elapsed() < E2E_CEILING,
        "the E2E took {:?}",
        clock.elapsed()
    );
}

fn assert_cleaned(sandbox: &Sandbox, name: &str) {
    let repo = sandbox.repo();
    let worktrees = sandbox.git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 1, "{worktrees}");
    assert_eq!(
        sandbox.git_output(
            &repo,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/heads/surt/",
                "refs/surt/"
            ]
        ),
        format!("refs/heads/surt/{name}\n")
    );
    let dot_surt = sandbox.dot_surt();
    assert!(!dot_surt.join("features").join(name).exists());
    let executions = dot_surt.join("runtime/executions");
    assert!(
        std::fs::read_dir(&executions).map_or(true, |mut entries| entries.next().is_none()),
        "{}",
        executions.display()
    );
    assert!(dot_surt.join("logs").join(name).is_dir());
    assert_eq!(
        count(
            sandbox,
            "workflow_run WHERE parent_step_run_id IS NULL AND cleaned_at IS NOT NULL"
        ),
        "1"
    );
}

pub fn planned(name: &str, scripts: &[String]) -> Sandbox {
    planned_with(name, scripts, None)
}

/// `config` is written before the run is created, since a run freezes its configuration.
pub fn planned_with(name: &str, scripts: &[String], config: Option<&str>) -> Sandbox {
    let mut all = vec![spec(1, "# Spec 1\n")];
    all.extend_from_slice(scripts);
    let sandbox = project(&all);
    if let Some(config) = config {
        sandbox.write_config(config);
    }
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
fn a_rate_limited_task_pauses_and_merges_while_a_blocked_task_stops_start_with_exit_4() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A", "B", "C"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            crash_in("B/build", 1, 1),
            crash_in("B/build", 1, 2),
            crash_in("B/build", 1, 3),
            limit_failure_in("C/build", RESET_DELAY_SECONDS),
            build("C", 1, 2, "cache.txt", "cache\n"),
            review("C", 1, "approved"),
        ],
    );

    let run = surt(&sandbox, &["start", &name]);

    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    let (b_execution, b_message) = last_failure(&sandbox, "B");
    let resets_at: i64 = text(
        &sandbox,
        "SELECT CAST(json_extract(e.runtime_info, '$.resets_at') AS TEXT) FROM execution e \
         JOIN step_run s ON s.id = e.step_run_id JOIN workflow_run w ON w.id = s.workflow_run_id \
         WHERE w.item_key = 'C' AND e.failure_class = 'transient'",
    )
    .parse()
    .expect("the reset time C's failure recorded");
    let resets = Timestamp::from_second(resets_at)
        .expect("a timestamp")
        .to_zoned(TimeZone::system())
        .strftime("%H:%M")
        .to_string();
    let paused = format!("⏸ agent launches paused until {resets} — ");
    let at = run.stderr.find(&paused).expect(&run.stderr);
    let rest = &run.stderr[at..];
    assert!(
        rest.lines()
            .next()
            .is_some_and(|line| line.ends_with(" waiting · Ctrl-C to stop (resumable)")),
        "{}",
        run.stderr
    );
    assert!(
        rest.contains("\n↻ agent launches resumed after "),
        "{}",
        run.stderr
    );
    assert!(
        rest.contains("\n↻ task C build attempt 2\n"),
        "{}",
        run.stderr
    );
    for task in ["A", "C"] {
        assert!(
            run.stderr.contains(&format!(
                "✔ task {task} merged ({})\n",
                short(&merged_as(&sandbox, task))
            )),
            "{}",
            run.stderr
        );
    }
    assert!(
        run.stderr.ends_with(&format!(
            "⛔ task B build blocked: attempts exhausted\n  \
             attempt 3 failed (exit_code): {b_message}\n  \
             inspect: surt inspect {b_execution}\n  \
             logs: surt logs {b_execution}\n  \
             resume: surt start {name}\n"
        )),
        "{}",
        run.stderr
    );
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
            "✖ manifest not approved yet (manifest rev 1 awaiting approval)\n  review manifest rev 1 and its Task Markdown files, then: surt approve {name}\n"
        )
    );
    assert_eq!(count(&sandbox, "execution"), executions);
}

#[test]
fn every_phase_command_finishes_a_pending_cleanup_of_the_feature() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            ship(),
        ],
    );
    ok(surt(&sandbox, &["start", &name]));
    let execution = text(&sandbox, "SELECT id FROM execution ORDER BY id LIMIT 1");
    let leftover = sandbox
        .dot_surt()
        .join("runtime/executions")
        .join(&execution)
        .join("inputs");

    for command in ["feature", "plan", "start"] {
        std::fs::create_dir_all(&leftover).expect("a leftover execution directory");
        sandbox.execute("UPDATE workflow_run SET cleaned_at = NULL");

        let run = surt(&sandbox, &[command, &name]);

        let finished = if command == "feature" {
            assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
            assert!(
                run.stderr.contains("--new-run"),
                "{command}: {}",
                run.stderr
            );
            run.stderr.split("✖").next().unwrap_or_default().to_owned()
        } else {
            ok(run)
        };
        assert!(
            finished.ends_with("✔ run succeeded\n↻ cleanup done\n"),
            "{command}: {finished}"
        );
        assert!(!leftover.exists(), "{command}");
        assert_eq!(
            count(&sandbox, "workflow_run WHERE cleaned_at IS NOT NULL"),
            "1"
        );
    }
    let refused = surt(&sandbox, &["start", &name]);
    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
}

#[test]
fn a_cleanup_keeps_an_edited_file_and_says_how_to_retry_what_it_could_not_remove() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            ship(),
        ],
    );
    ok(surt(&sandbox, &["start", &name]));
    let spec = text(
        &sandbox,
        "SELECT w.rel_path FROM working_copy_file w JOIN artifact a ON a.id = w.artifact_id \
         WHERE a.name = 'spec'",
    );
    let edited = format!(".surt/features/{name}/{spec}");
    std::fs::create_dir_all(sandbox.dot_surt().join("features").join(&name))
        .expect("the working copy");
    std::fs::write(sandbox.repo().join(&edited), "# Spec, edited later\n").expect("an edit");
    let execution = text(&sandbox, "SELECT id FROM execution ORDER BY id LIMIT 1");
    let blocker = sandbox
        .dot_surt()
        .join("runtime/executions")
        .join(&execution);
    std::fs::write(&blocker, "not a directory\n").expect("a file in the way");
    sandbox.execute("UPDATE workflow_run SET cleaned_at = NULL");

    let incomplete = ok(surt(&sandbox, &["start", &name]));

    assert!(
        incomplete.contains(&format!("⚠ kept {edited} — unrecorded edit\n")),
        "{incomplete}"
    );
    assert!(
        incomplete.contains(&format!(
            "⚠ cleanup incomplete — could not remove .surt/runtime/executions/{execution} ("
        )),
        "{incomplete}"
    );
    assert!(
        incomplete.ends_with(&format!("  retry: surt start {name}\n")),
        "{incomplete}"
    );
    assert_eq!(
        count(&sandbox, "workflow_run WHERE cleaned_at IS NOT NULL"),
        "0"
    );
    std::fs::remove_file(&blocker).expect("the blocker is removed");

    let finished = ok(surt(&sandbox, &["start", &name]));

    assert!(
        finished.ends_with(&format!(
            "↻ cleanup done\n⚠ kept {edited} — unrecorded edit\n"
        )),
        "{finished}"
    );
    assert_eq!(
        count(&sandbox, "workflow_run WHERE cleaned_at IS NOT NULL"),
        "1"
    );
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

    assert!(
        resumed
            .ends_with("✔ run succeeded\n↻ cleanup 2 worktrees · 2 task branches · working copy\n"),
        "{resumed}"
    );
}
