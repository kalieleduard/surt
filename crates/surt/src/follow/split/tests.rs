use super::split_line;

fn split(line: &str) -> Vec<String> {
    split_line(line).expect("the line splits")
}

#[test]
fn plain_words_are_separated_by_whitespace() {
    assert_eq!(split("surt continue s"), ["surt", "continue", "s"]);
}

#[test]
fn repeated_spaces_count_as_one_separator() {
    assert_eq!(split("surt   approve \t s"), ["surt", "approve", "s"]);
}

#[test]
fn leading_and_trailing_spaces_are_ignored() {
    assert_eq!(split("  surt status  "), ["surt", "status"]);
}

#[test]
fn single_quotes_group_text_with_spaces() {
    assert_eq!(split("-m 'a b  c'"), ["-m", "a b  c"]);
}

#[test]
fn double_quotes_group_text_with_spaces() {
    assert_eq!(split("-m \"a b  c\""), ["-m", "a b  c"]);
}

#[test]
fn one_quote_kind_is_literal_inside_the_other() {
    assert_eq!(split("\"it's\" 'say \"hi\"'"), ["it's", "say \"hi\""]);
}

#[test]
fn adjacent_quoted_and_unquoted_text_form_one_argument() {
    assert_eq!(split("a\"b c\"d 'e'f"), ["ab cd", "ef"]);
}

#[test]
fn an_empty_quoted_pair_is_an_empty_argument() {
    assert_eq!(split("surt \"\" ''"), ["surt", "", ""]);
}

#[test]
fn a_reject_reason_with_spaces_stays_whole() {
    assert_eq!(
        split("surt reject s -m \"motivo com espaços\""),
        ["surt", "reject", "s", "-m", "motivo com espaços"]
    );
}

#[test]
fn backslashes_are_ordinary_characters() {
    assert_eq!(
        split(r#"surt run C:\dev\surt "C:\Program Files\x""#),
        ["surt", "run", r"C:\dev\surt", r"C:\Program Files\x"]
    );
}

#[test]
fn an_unterminated_single_quote_is_an_error() {
    let error = split_line("surt reject s -m 'motivo").expect_err("the quote is open");
    assert_eq!(error.to_string(), "unterminated ' quote");
}

#[test]
fn an_unterminated_double_quote_is_an_error() {
    let error = split_line("surt reject s -m \"motivo").expect_err("the quote is open");
    assert_eq!(error.to_string(), "unterminated \" quote");
}

#[test]
fn empty_or_blank_input_has_no_arguments() {
    assert!(split("").is_empty());
    assert!(split("  \t ").is_empty());
}
