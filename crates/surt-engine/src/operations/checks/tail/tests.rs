use super::last_lines;

#[test]
fn only_the_last_lines_are_kept() {
    assert_eq!(last_lines("one\ntwo\nthree\n", 2), "two\nthree\n");
}

#[test]
fn a_last_line_without_a_newline_counts_as_a_line() {
    assert_eq!(last_lines("one\ntwo\nthree", 2), "two\nthree");
}

#[test]
fn fewer_lines_than_the_limit_are_kept_whole() {
    assert_eq!(last_lines("one\n", 200), "one\n");
}
