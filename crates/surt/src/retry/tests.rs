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
fn command_words_are_quoted_only_when_the_shell_needs_it() {
    let retry =
        Retry::command(&["surt", "run", "feature", "google sso", "origin/main~2"]).word("--merge");

    assert_eq!(
        retry.render(),
        "surt run feature \"google sso\" origin/main~2 --merge"
    );
}
