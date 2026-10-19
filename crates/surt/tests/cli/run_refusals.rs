use crate::run::{project, subjects};
use crate::scripts::spec;
use crate::support::{REFUSED_EXIT_CODE, Sandbox};

const USAGE_EXIT_CODE: i32 = 2;
const FEATURE_INPUTS: &str = "  inputs of feature:\n    \
     request  text     required  what to build\n    \
     base     git_ref  optional  branch, tag or commit to start from (default HEAD)\n";

#[test]
fn a_base_that_names_no_commit_is_refused_listing_the_inputs() {
    let sandbox = project();

    let run = sandbox.run_feature("add login", &["--input", "base=no-such-branch"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        format!(
            "✖ input \"base\": \"no-such-branch\" does not name a commit\n{FEATURE_INPUTS}  \
             fix: pass --input base=<branch, tag or commit>\n"
        )
    );
    assert!(subjects(&sandbox).is_empty());
}

#[test]
fn a_repository_without_commits_has_no_base_to_start_from() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    sandbox.script(&[spec(
        1, "# Spec
",
    )]);

    let run = sandbox.run_feature("add login", &[]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.starts_with(
            "✖ input \"base\": HEAD does not name a commit; the repository has no commits yet\n"
        ),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .ends_with("  fix: git commit --allow-empty -m \"first commit\"\n"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_request_with_no_portable_characters_asks_for_a_name() {
    let sandbox = project();

    let run = sandbox.run_feature("ログイン", &[]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "✖ cannot derive a name from the subject input; pass --name <slug>\n  fix: surt run feature --input request=\"ログイン\" --name <slug>\n"
    );
    assert!(subjects(&sandbox).is_empty());
}

#[test]
fn an_input_file_that_cannot_be_read_is_refused() {
    let sandbox = project();

    let run = sandbox.run_feature("@missing.txt", &[]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("✖ cannot read --input request=@missing.txt: "),
        "{}",
        run.stderr
    );
    assert!(subjects(&sandbox).is_empty());
}

#[test]
fn a_missing_required_input_is_refused_listing_every_declared_input() {
    let sandbox = project();

    let run = sandbox.surt(&sandbox.repo(), &["run", "feature"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        format!(
            "✖ input \"request\" is required\n{FEATURE_INPUTS}  fix: pass --input request=<value>\n"
        )
    );
    assert!(subjects(&sandbox).is_empty());
}

#[test]
fn an_unknown_input_is_refused_listing_every_declared_input() {
    let sandbox = project();

    let run = sandbox.run_feature("add login", &["--input", "colour=blue"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        format!(
            "✖ workflow feature has no input \"colour\"\n{FEATURE_INPUTS}  \
             fix: drop --input colour, or declare it in .surt/workflows/feature.toml\n"
        )
    );
}

#[test]
fn an_input_without_a_name_is_a_usage_error() {
    let sandbox = project();

    let run = sandbox.surt(&sandbox.repo(), &["run", "feature", "--input", "add login"]);

    assert_eq!(run.code, Some(USAGE_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr
            .contains("expected <name>=<value> or <name>=@<path>"),
        "{}",
        run.stderr
    );
}

#[test]
fn feature_no_longer_creates_a_subject_from_a_description() {
    let sandbox = project();

    let run = sandbox.feature(&["add login"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(subjects(&sandbox).is_empty());
}
