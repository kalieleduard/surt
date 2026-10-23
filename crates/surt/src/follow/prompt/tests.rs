use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Key, translated, visible};

fn chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

#[test]
fn a_short_line_is_shown_whole_with_the_cursor_after_the_prefix() {
    let text = chars("surt approve s");
    assert_eq!(
        visible(&text, text.len(), 80, 2),
        ("surt approve s".to_owned(), 16)
    );
    assert_eq!(visible(&text, 0, 80, 2), ("surt approve s".to_owned(), 2));
}

#[test]
fn a_line_that_exactly_fits_is_not_cut() {
    let text = chars("abcdefg");
    assert_eq!(visible(&text, 7, 10, 2), ("abcdefg".to_owned(), 9));
}

#[test]
fn a_long_line_keeps_the_cursor_at_the_end_visible() {
    let text = chars("abcdefghij");
    assert_eq!(visible(&text, 10, 8, 2), ("fghij".to_owned(), 7));
}

#[test]
fn a_long_line_with_the_cursor_at_the_start_shows_the_beginning() {
    let text = chars("abcdefghij");
    assert_eq!(visible(&text, 0, 8, 2), ("abcde".to_owned(), 2));
}

#[test]
fn a_long_line_with_the_cursor_in_the_middle_keeps_it_visible() {
    let text = chars("abcdefghij");
    assert_eq!(visible(&text, 7, 8, 2), ("cdefg".to_owned(), 7));
}

#[test]
fn multi_byte_characters_take_one_column() {
    let text = chars("açãoé");
    assert_eq!(visible(&text, 5, 6, 2), ("ãoé".to_owned(), 5));
}

#[test]
fn a_tiny_terminal_still_shows_one_character() {
    let text = chars("abc");
    assert_eq!(visible(&text, 3, 0, 2), ("c".to_owned(), 3));
    assert_eq!(visible(&[], 0, 0, 2), (String::new(), 2));
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
    translated(KeyEvent::new(code, modifiers))
}

#[test]
fn escape_and_control_c_or_d_quit() {
    assert_eq!(key(KeyCode::Esc, KeyModifiers::NONE), Some(Key::Quit));
    assert_eq!(
        key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Some(Key::Quit)
    );
    assert_eq!(
        key(KeyCode::Char('d'), KeyModifiers::CONTROL),
        Some(Key::Quit)
    );
}

#[test]
fn editing_keys_are_translated() {
    assert_eq!(
        key(KeyCode::Char('ç'), KeyModifiers::NONE),
        Some(Key::Char('ç'))
    );
    assert_eq!(
        key(KeyCode::Char('R'), KeyModifiers::SHIFT),
        Some(Key::Char('R'))
    );
    assert_eq!(key(KeyCode::Enter, KeyModifiers::NONE), Some(Key::Enter));
    assert_eq!(
        key(KeyCode::Backspace, KeyModifiers::NONE),
        Some(Key::Backspace)
    );
    assert_eq!(key(KeyCode::Delete, KeyModifiers::NONE), Some(Key::Delete));
    assert_eq!(key(KeyCode::Left, KeyModifiers::NONE), Some(Key::Left));
    assert_eq!(key(KeyCode::Right, KeyModifiers::NONE), Some(Key::Right));
    assert_eq!(key(KeyCode::Home, KeyModifiers::NONE), Some(Key::Home));
    assert_eq!(key(KeyCode::End, KeyModifiers::NONE), Some(Key::End));
}

#[test]
fn other_keys_are_ignored() {
    assert_eq!(key(KeyCode::Up, KeyModifiers::NONE), None);
    assert_eq!(key(KeyCode::Tab, KeyModifiers::NONE), None);
    assert_eq!(key(KeyCode::Char('w'), KeyModifiers::CONTROL), None);
}
