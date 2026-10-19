use super::{Retry, quoted};

#[test]
fn a_description_is_always_double_quoted_with_shell_specials_escaped() {
    assert_eq!(quoted("add login"), "\"add login\"");
    assert_eq!(
        quoted(r#"say "hi" to $USER `now` \o/"#),
        r#""say \"hi\" to \$USER \`now\` \\o/""#
    );
}

#[test]
fn option_values_are_quoted_only_when_the_shell_needs_it() {
    let retry = Retry::command(&["surt", "run", "feature"])
        .option("--name", Some("google sso"))
        .option("--base", Some("origin/main~2"))
        .option("--skipped", None)
        .word("--new");

    assert_eq!(
        retry.render(),
        "surt run feature --name \"google sso\" --base origin/main~2 --new"
    );
}
