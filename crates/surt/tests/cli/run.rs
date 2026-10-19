use std::fs;

use jiff::Zoned;

use crate::scripts::spec;
use crate::support::{REFUSED_EXIT_CODE, Sandbox};

const BRANCH: &str = "trunk";

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
fn a_request_creates_a_dated_subject_and_drives_it_to_the_spec_gate() {
    let sandbox = project();
    let name = format!("{}-add-login", today());

    let run = sandbox.run_feature("Add Login", &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let short = &head(&sandbox)[..7];
    assert!(
        run.stderr.starts_with(&format!(
            "✔ created {name}   workflow: feature   base: {BRANCH} @ {short}\n"
        )),
        "{}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert_eq!(subjects(&sandbox), [name]);
    assert_eq!(
        sandbox.texts(
            "SELECT step_key || ' ' || state FROM step_run WHERE state <> 'pending' ORDER BY step_key"
        ),
        ["approve-spec running", "spec done", "spec-loop running"]
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
    sandbox.run_feature("add login", &[]);

    let second = sandbox.run_feature("add login", &["--new"]);
    let third = sandbox.run_feature("Add login!", &["--new"]);

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
fn a_request_whose_slug_has_an_active_run_refuses_pointing_at_continue() {
    let sandbox = project();
    sandbox.run_feature("add login", &[]);
    let name = format!("{}-add-login", today());

    let run = sandbox.run_feature("Add login!", &[]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        format!(
            "✖ {name} already has an active run\n  resume: surt continue {name}\n  or: surt run feature --input request=\"Add login!\" --new\n"
        )
    );
    assert_eq!(subjects(&sandbox), [name]);
}

#[test]
fn new_skips_the_active_slug_guard() {
    let sandbox = project();
    sandbox.run_feature("add login", &[]);

    let run = sandbox.run_feature("add login", &["--new"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(subjects(&sandbox).len(), 2);
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
    assert_eq!(
        subjects(&sandbox),
        [format!("{}-add-login-with-google", today())]
    );
    assert_eq!(
        initial_artifact(&sandbox, "request"),
        "Add login\nwith Google\n"
    );
}
