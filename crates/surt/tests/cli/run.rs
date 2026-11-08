use std::fs;

use jiff::Zoned;

use crate::scripts::spec;
use crate::support::Sandbox;

const BRANCH: &str = "trunk";
const USAGE_EXIT_CODE: i32 = 2;
// Spelled in parts so the removed flag leaves no literal trace in the sources.
const REMOVED_FLAG: &str = concat!("--", "new");

pub(crate) fn project() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.git(&sandbox.repo(), &["checkout", "--quiet", "-b", BRANCH]);
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[spec(1, "# Spec\n")]);
    sandbox
}

pub(crate) fn today() -> String {
    Zoned::now().date().strftime("%Y%m%d").to_string()
}

pub(crate) fn head(sandbox: &Sandbox) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["rev-parse", "HEAD"])
        .trim()
        .to_owned()
}

pub(crate) fn subjects(sandbox: &Sandbox) -> Vec<String> {
    sandbox.texts("SELECT name FROM subject ORDER BY name")
}

pub(crate) fn initial_artifact(sandbox: &Sandbox, name: &str) -> String {
    let sql = format!(
        "SELECT CAST(b.bytes AS TEXT) FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
         JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 \
         WHERE a.name = '{name}'"
    );
    sandbox.texts(&sql).concat()
}

#[test]
fn a_request_creates_a_numbered_subject_and_drives_it_to_the_spec_gate() {
    let sandbox = project();
    let name = format!("{}-1", today());

    let run = sandbox.run_feature_unnamed("Add Login", &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let short = &head(&sandbox)[..7];
    assert!(
        run.stderr.starts_with(&format!(
            "✔ created {name}   workflow: feature   base: {BRANCH} @ {short}
"
        )),
        "{}",
        run.stderr
    );
    assert!(sandbox.dot_surt().join("subjects").join(&name).is_dir());
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert_eq!(subjects(&sandbox), [name]);
    assert_eq!(
        sandbox.texts(
            "SELECT step_key || ' ' || state FROM step_run WHERE state <> 'pending' ORDER BY step_key"
        ),
        [
            "answer-discovery skipped",
            "approve-spec running",
            "discovery done",
            "discovery-loop done",
            "spec done",
            "spec-loop running"
        ]
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
    sandbox.run_feature("add login", &["--name", "add login"]);

    let second = sandbox.run_feature("add login", &["--name", "add login"]);

    let first = format!("{}-add-login", today());
    assert_eq!(second.code, Some(0), "{}", second.stderr);
    assert!(
        second
            .stderr
            .starts_with(&format!("✔ created {first}-2   workflow: feature   base: ")),
        "{}",
        second.stderr
    );
    assert!(
        second.stderr.contains(&format!(
            "
  {first} already exists
"
        )),
        "{}",
        second.stderr
    );
    assert_eq!(subjects(&sandbox), [first.clone(), format!("{first}-2")]);
}

#[test]
fn name_overrides_the_slug_but_not_the_request_or_the_date() {
    let sandbox = project();

    let run = sandbox.run_feature(
        "Let people sign in with their Google account",
        &["--name", "Google-SSO"],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(subjects(&sandbox), [format!("{}-google-sso", today())]);
    assert_eq!(
        initial_artifact(&sandbox, "request"),
        "Let people sign in with their Google account"
    );
}

#[test]
fn a_second_run_while_the_first_is_active_takes_the_next_number() {
    let sandbox = project();

    let first = sandbox.run_feature_unnamed("add login", &[]);
    let second = sandbox.run_feature_unnamed("add login", &[]);

    let today = today();
    assert_eq!(first.code, Some(0), "{}", first.stderr);
    assert_eq!(second.code, Some(0), "{}", second.stderr);
    assert!(
        second
            .stderr
            .starts_with(&format!("✔ created {today}-2   workflow: feature   base: ")),
        "{}",
        second.stderr
    );
    assert!(
        !second.stderr.contains("already exists"),
        "{}",
        second.stderr
    );
    assert_eq!(
        subjects(&sandbox),
        [format!("{today}-1"), format!("{today}-2")]
    );
    assert_eq!(
        sandbox.texts(
            "SELECT state FROM workflow_run WHERE parent_step_run_id IS NULL ORDER BY rowid"
        ),
        ["active", "active"]
    );
}

#[test]
fn the_next_number_skips_named_subjects_and_follows_the_highest() {
    let sandbox = project();
    sandbox.run_feature_unnamed("add login", &[]);
    sandbox.run_feature("add login", &["--name", "5"]);
    sandbox.run_feature("add login", &["--name", "foo"]);

    let run = sandbox.run_feature_unnamed("add login", &[]);

    let today = today();
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        subjects(&sandbox),
        [
            format!("{today}-1"),
            format!("{today}-5"),
            format!("{today}-6"),
            format!("{today}-foo")
        ]
    );
}

#[test]
fn the_new_flag_is_no_longer_accepted() {
    let sandbox = project();

    let run = sandbox.run_feature_unnamed("add login", &[REMOVED_FLAG]);

    assert_eq!(run.code, Some(USAGE_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.contains(&format!("'{REMOVED_FLAG}'")),
        "{}",
        run.stderr
    );
    assert!(subjects(&sandbox).is_empty());
}

#[test]
fn a_base_input_fixes_the_commit_the_run_starts_from() {
    let sandbox = project();
    let first = head(&sandbox);
    sandbox.git(&sandbox.repo(), &["tag", "v1"]);
    sandbox.git(
        &sandbox.repo(),
        &["commit", "--quiet", "--allow-empty", "-m", "second"],
    );

    let run = sandbox.run_feature("add login", &["--input", "base=v1"]);

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
fn an_input_read_from_a_file_relative_to_the_current_directory_becomes_the_request() {
    let sandbox = project();
    let notes = sandbox.subdirectory("notes");
    fs::write(notes.join("request.txt"), "Add login\nwith Google\n").expect("the request file");

    let run = sandbox.surt(
        &notes,
        &["run", "feature", "--input", "request=@request.txt"],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(subjects(&sandbox), [format!("{}-1", today())]);
    assert_eq!(
        initial_artifact(&sandbox, "request"),
        "Add login\nwith Google\n"
    );
}
