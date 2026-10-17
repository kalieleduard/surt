use std::path::PathBuf;

use super::Layout;

fn project_name_of(directory: &str) -> String {
    Layout::new(PathBuf::from("/work").join(directory)).project_name()
}

#[test]
fn the_project_name_is_the_directory_reduced_to_a_portable_slug() {
    let cases = [
        ("app", "app"),
        ("My App", "my-app"),
        ("surt_engine.v2", "surt-engine-v2"),
        ("--Weird__Name--", "weird-name"),
    ];

    for (directory, expected) in cases {
        assert_eq!(project_name_of(directory), expected, "{directory}");
    }
}

#[test]
fn the_project_name_stops_at_a_word_boundary_within_24_characters() {
    assert_eq!(
        project_name_of("the quick brown fox jumps over"),
        "the-quick-brown-fox"
    );
    assert_eq!(
        project_name_of("abcdefghijklmnopqrstuvwxyz-tail"),
        "abcdefghijklmnopqrstuvwx"
    );
}

#[test]
fn a_directory_without_ascii_letters_or_digits_is_named_project() {
    assert_eq!(project_name_of("日本語"), "project");
    assert_eq!(Layout::new(PathBuf::from("/")).project_name(), "project");
}
