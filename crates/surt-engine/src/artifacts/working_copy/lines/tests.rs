use super::{LineCounts, counts};

fn counted(old: &str, new: &str) -> (usize, usize) {
    let LineCounts { added, removed } = counts(old.as_bytes(), new.as_bytes());
    (added, removed)
}

#[test]
fn a_rewritten_line_counts_once_each_way() {
    assert_eq!(counted("a\nb\nc\n", "a\nB\nc\n"), (1, 1));
}

#[test]
fn appended_and_dropped_lines_count_one_way() {
    assert_eq!(counted("a\n", "a\nb\nc\n"), (2, 0));
    assert_eq!(counted("a\nb\nc\n", "c\n"), (0, 2));
}

#[test]
fn a_missing_final_newline_is_not_a_line_and_empty_content_has_none() {
    assert_eq!(counted("a\nb", "a\nb\n"), (0, 0));
    assert_eq!(counted("", "a\n"), (1, 0));
}

#[test]
fn moved_lines_count_as_removed_and_added() {
    assert_eq!(counted("a\nb\nc\n", "c\na\nb\n"), (1, 1));
}
