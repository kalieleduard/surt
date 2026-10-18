use jiff::Zoned;

use crate::support::{REFUSED_EXIT_CODE, Sandbox};

const BRANCH: &str = "trunk";

fn project() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.git(&sandbox.repo(), &["checkout", "--quiet", "-b", BRANCH]);
    sandbox.commit_something();
    sandbox.init();
    sandbox
}

fn today() -> String {
    Zoned::now().date().strftime("%Y%m%d").to_string()
}

fn head(sandbox: &Sandbox) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["rev-parse", "HEAD"])
        .trim()
        .to_owned()
}

fn subjects(sandbox: &Sandbox) -> Vec<String> {
    sandbox.texts("SELECT name FROM subject ORDER BY name")
}

fn initial_artifact(sandbox: &Sandbox, name: &str) -> String {
    let sql = format!(
        "SELECT CAST(b.bytes AS TEXT) FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
         JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 \
         WHERE a.name = '{name}'"
    );
    sandbox.texts(&sql).concat()
}

#[test]
fn a_description_creates_a_dated_feature_with_a_pending_run_and_stops() {
    let sandbox = project();
    let name = format!("{}-add-login", today());

    let run = sandbox.feature(&["Add Login"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let short = &head(&sandbox)[..7];
    assert!(
        run.stderr
            .starts_with(&format!("✔ created {name}   base: {BRANCH} @ {short}\n")),
        "{}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert_eq!(subjects(&sandbox), [name]);
    assert_eq!(
        sandbox.texts("SELECT state FROM workflow_run WHERE parent_step_run_id IS NULL"),
        ["active"]
    );
    assert_eq!(
        sandbox.texts("SELECT DISTINCT state FROM step_run"),
        ["pending"]
    );
    assert_eq!(initial_artifact(&sandbox, "request"), "Add Login");
    let base: serde_json::Value =
        serde_json::from_str(&initial_artifact(&sandbox, "base")).expect("base is JSON");
    assert_eq!(
        base,
        serde_json::json!({ "ref": BRANCH, "sha": head(&sandbox) })
    );
}

#[test]
fn a_taken_name_gets_the_next_numeric_suffix() {
    let sandbox = project();
    sandbox.feature(&["add login"]);

    let second = sandbox.feature(&["add login", "--new"]);
    let third = sandbox.feature(&["Add login!", "--new"]);

    let first = format!("{}-add-login", today());
    assert_eq!(second.code, Some(0), "{}", second.stderr);
    assert!(
        second
            .stderr
            .starts_with(&format!("✔ created {first}-2   base: ")),
        "{}",
        second.stderr
    );
    assert!(
        second
            .stderr
            .contains(&format!("\n  {first} already exists\n")),
        "{}",
        second.stderr
    );
    assert_eq!(third.code, Some(0), "{}", third.stderr);
    assert_eq!(
        subjects(&sandbox),
        [first.clone(), format!("{first}-2"), format!("{first}-3")]
    );
}

#[test]
fn name_overrides_the_slug_but_not_the_request_or_the_date() {
    let sandbox = project();

    let run = sandbox.feature(&[
        "Let people sign in with their Google account",
        "--name",
        "Google-SSO",
    ]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(subjects(&sandbox), [format!("{}-google-sso", today())]);
    assert_eq!(
        initial_artifact(&sandbox, "request"),
        "Let people sign in with their Google account"
    );
}

#[test]
fn rerunning_a_description_whose_slug_has_an_active_run_refuses_and_changes_nothing() {
    let sandbox = project();
    sandbox.feature(&["add login"]);
    let name = format!("{}-add-login", today());

    let run = sandbox.feature(&["Add login!"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        format!(
            "✖ {name} already has an active run for \"Add login!\"\n  resume: surt feature {name}\n  or: surt feature \"Add login!\" --new\n"
        )
    );
    assert_eq!(subjects(&sandbox), [name]);
}

#[test]
fn new_skips_the_active_slug_guard() {
    let sandbox = project();
    sandbox.feature(&["add login"]);

    let run = sandbox.feature(&["add login", "--new"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(subjects(&sandbox).len(), 2);
}

#[test]
fn base_fixes_the_commit_the_feature_starts_from() {
    let sandbox = project();
    let first = head(&sandbox);
    sandbox.git(&sandbox.repo(), &["tag", "v1"]);
    sandbox.git(
        &sandbox.repo(),
        &["commit", "--quiet", "--allow-empty", "-m", "second"],
    );

    let run = sandbox.feature(&["add login", "--base", "v1"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stderr.contains(&format!("   base: {}\n", &first[..7])),
        "{}",
        run.stderr
    );
    let base: serde_json::Value =
        serde_json::from_str(&initial_artifact(&sandbox, "base")).expect("base is JSON");
    assert_eq!(base, serde_json::json!({ "ref": null, "sha": first }));
}

#[test]
fn a_base_that_names_no_commit_is_refused() {
    let sandbox = project();

    let run = sandbox.feature(&["add login", "--base", "no-such-branch"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "✖ --base no-such-branch does not name a commit\n  fix: surt feature \"add login\" --base <branch, tag or commit>\n"
    );
    assert!(subjects(&sandbox).is_empty());
}

#[test]
fn a_repository_without_commits_has_no_base_to_start_from() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();

    let run = sandbox.feature(&["add login"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("✖ HEAD does not name a commit; the repository has no commits yet\n"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_description_with_no_portable_characters_asks_for_a_name() {
    let sandbox = project();

    let run = sandbox.feature(&["ログイン"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "✖ cannot derive a name from the description; pass --name <slug>\n  fix: surt feature \"ログイン\" --name <slug>\n"
    );
    assert!(subjects(&sandbox).is_empty());
}
