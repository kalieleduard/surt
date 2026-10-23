use super::{Key, LineEditor, Step};

fn pressed(initial: &str, keys: &[Key]) -> LineEditor {
    let mut editor = LineEditor::new(initial);
    for key in keys {
        assert_eq!(editor.press(*key), Step::Pending);
    }
    editor
}

fn typed(text: &str) -> Vec<Key> {
    text.chars().map(Key::Char).collect()
}

#[test]
fn a_new_editor_puts_the_cursor_at_the_end() {
    let editor = LineEditor::new("surt approve s");
    assert_eq!(editor.text(), "surt approve s");
    assert_eq!(editor.cursor(), 14);
}

#[test]
fn an_empty_editor_starts_at_zero() {
    let editor = LineEditor::new("");
    assert_eq!(editor.text(), "");
    assert_eq!(editor.cursor(), 0);
}

#[test]
fn characters_are_appended_at_the_end() {
    let editor = pressed("surt", &typed(" status"));
    assert_eq!(editor.text(), "surt status");
    assert_eq!(editor.cursor(), 11);
}

#[test]
fn characters_are_inserted_in_the_middle() {
    let editor = pressed("sut", &[Key::Left, Key::Char('r')]);
    assert_eq!(editor.text(), "surt");
    assert_eq!(editor.cursor(), 3);
}

#[test]
fn backspace_removes_the_character_before_the_cursor() {
    let editor = pressed("surtx", &[Key::Backspace]);
    assert_eq!(editor.text(), "surt");
    assert_eq!(editor.cursor(), 4);
}

#[test]
fn backspace_at_the_start_does_nothing() {
    let editor = pressed("surt", &[Key::Home, Key::Backspace]);
    assert_eq!(editor.text(), "surt");
    assert_eq!(editor.cursor(), 0);
}

#[test]
fn delete_removes_the_character_under_the_cursor() {
    let editor = pressed("xsurt", &[Key::Home, Key::Delete]);
    assert_eq!(editor.text(), "surt");
    assert_eq!(editor.cursor(), 0);
}

#[test]
fn delete_at_the_end_does_nothing() {
    let editor = pressed("surt", &[Key::Delete]);
    assert_eq!(editor.text(), "surt");
    assert_eq!(editor.cursor(), 4);
}

#[test]
fn left_and_right_move_one_character() {
    let editor = pressed("surt", &[Key::Left, Key::Left, Key::Right]);
    assert_eq!(editor.cursor(), 3);
}

#[test]
fn left_at_the_start_and_right_at_the_end_do_nothing() {
    let editor = pressed("ab", &[Key::Right]);
    assert_eq!(editor.cursor(), 2);
    let editor = pressed("ab", &[Key::Home, Key::Left]);
    assert_eq!(editor.cursor(), 0);
}

#[test]
fn home_and_end_jump_to_the_ends() {
    let editor = pressed("surt", &[Key::Home]);
    assert_eq!(editor.cursor(), 0);
    let editor = pressed("surt", &[Key::Home, Key::End]);
    assert_eq!(editor.cursor(), 4);
}

#[test]
fn keys_on_an_empty_line_never_panic() {
    let editor = pressed(
        "",
        &[
            Key::Backspace,
            Key::Delete,
            Key::Left,
            Key::Right,
            Key::Home,
            Key::End,
        ],
    );
    assert_eq!(editor.text(), "");
    assert_eq!(editor.cursor(), 0);
}

#[test]
fn multi_byte_characters_count_as_one() {
    let editor = pressed("ação", &[Key::Left, Key::Backspace, Key::Char('a')]);
    assert_eq!(editor.text(), "açao");
    assert_eq!(editor.cursor(), 3);
    let editor = pressed("ç", &[Key::Left, Key::Delete, Key::Char('é')]);
    assert_eq!(editor.text(), "é");
    assert_eq!(editor.cursor(), 1);
}

#[test]
fn enter_submits_the_current_text() {
    let mut editor = pressed("surt", &typed(" status"));
    assert_eq!(
        editor.press(Key::Enter),
        Step::Submitted("surt status".to_owned())
    );
}

#[test]
fn quit_ends_without_text() {
    let mut editor = LineEditor::new("surt approve s");
    assert_eq!(editor.press(Key::Quit), Step::Quit);
}

#[test]
fn an_approve_line_becomes_a_reject_with_a_message() {
    let mut keys = vec![Key::Home];
    keys.extend([Key::Right; 5]);
    keys.extend([Key::Delete; 7]);
    keys.extend(typed("reject"));
    keys.push(Key::End);
    keys.extend(typed(" -m \"x\""));
    let mut editor = pressed("surt approve s", &keys);
    assert_eq!(
        editor.press(Key::Enter),
        Step::Submitted("surt reject s -m \"x\"".to_owned())
    );
}
