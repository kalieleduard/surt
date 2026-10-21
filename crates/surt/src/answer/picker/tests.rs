use surt_engine::project::{Question, Questions};

use super::view::wrap;
use super::{Key, Palette, Picker, Position, Step, answered, lines};

const AT: Position = Position {
    number: 1,
    total: 2,
};
const WIDE: usize = 100;

fn questions() -> Questions {
    let document = serde_json::json!({ "questions": [
        {
            "id": "login-id",
            "question": "Log in by e-mail only, or by username too?",
            "why": "changes the users table",
            "recommended": "e-mail or username",
            "options": [
                { "value": "e-mail only", "description": "one login field" },
                { "value": "e-mail or username", "description": "adds a unique username column" },
                "username only",
            ],
        },
        {
            "id": "expiry",
            "question": "Does a session expire, and after how long?",
            "why": "decides whether the Feature needs a refresh flow",
            "recommended": "7 days, no refresh",
        },
    ]});
    Questions::parse(document.to_string().as_bytes()).expect("valid questions")
}

fn closed(questions: &Questions) -> &Question {
    &questions.questions()[0]
}

fn open(questions: &Questions) -> &Question {
    &questions.questions()[1]
}

fn pressed(picker: &mut Picker<'_>, keys: &[Key]) -> Step {
    let mut step = Step::Pending;
    for key in keys {
        step = picker.press(*key);
    }
    step
}

fn typed(text: &str) -> Vec<Key> {
    text.chars().map(Key::Char).collect()
}

fn answer(text: &str) -> Step {
    Step::Answered(text.to_owned())
}

fn shown(picker: &Picker<'_>, columns: usize) -> String {
    lines(picker, AT, columns, &Palette::plain()).join("\n")
}

#[test]
fn the_cursor_starts_on_the_recommendation_so_enter_accepts_it() {
    let questions = questions();
    let mut picker = Picker::new(closed(&questions));

    assert_eq!(picker.cursor(), 1);
    assert_eq!(picker.press(Key::Enter), answer("e-mail or username"));
}

#[test]
fn the_arrows_move_through_the_options_and_wrap_around_the_other_answer_entry() {
    let questions = questions();
    let mut picker = Picker::new(closed(&questions));

    assert_eq!(
        pressed(&mut picker, &[Key::Up, Key::Enter]),
        answer("e-mail only")
    );

    let mut picker = Picker::new(closed(&questions));
    pressed(&mut picker, &[Key::Down, Key::Down]);
    assert_eq!(picker.cursor(), 3);
    pressed(&mut picker, &[Key::Down]);
    assert_eq!(picker.cursor(), 0);
    pressed(&mut picker, &[Key::Up]);
    assert_eq!(picker.cursor(), 3);
}

#[test]
fn a_digit_picks_its_option_at_once_and_other_characters_are_ignored() {
    let questions = questions();

    assert_eq!(
        Picker::new(closed(&questions)).press(Key::Char('3')),
        answer("username only")
    );
    for ignored in ['0', '4', 'x'] {
        let mut picker = Picker::new(closed(&questions));
        assert_eq!(picker.press(Key::Char(ignored)), Step::Pending, "{ignored}");
        assert_eq!(picker.cursor(), 1);
    }
}

#[test]
fn the_other_answer_entry_takes_typed_text_with_backspace_and_trims_it() {
    let questions = questions();
    let mut picker = Picker::new(closed(&questions));
    pressed(&mut picker, &[Key::Down, Key::Down, Key::Enter]);

    assert_eq!(picker.typed(), Some(""));
    let mut keys = typed(" SSO onlyy");
    keys.extend([Key::Erase, Key::Char(' '), Key::Enter]);
    assert_eq!(pressed(&mut picker, &keys), answer("SSO only"));
}

#[test]
fn an_empty_other_answer_or_escape_returns_to_the_options() {
    let questions = questions();
    let mut picker = Picker::new(closed(&questions));
    pressed(&mut picker, &[Key::Up, Key::Up, Key::Enter]);

    assert_eq!(
        pressed(&mut picker, &[Key::Char(' '), Key::Enter]),
        Step::Pending
    );
    assert_eq!(picker.typed(), None);

    pressed(&mut picker, &[Key::Enter, Key::Char('x'), Key::Back]);
    assert_eq!(picker.typed(), None);
    assert_eq!(picker.cursor(), 3);
}

#[test]
fn an_open_question_is_typed_and_an_empty_answer_takes_the_recommendation() {
    let questions = questions();

    let mut picker = Picker::new(open(&questions));
    assert_eq!(picker.typed(), Some(""));
    assert_eq!(picker.press(Key::Enter), answer("7 days, no refresh"));

    let mut picker = Picker::new(open(&questions));
    let mut keys = typed("30 days");
    keys.push(Key::Enter);
    assert_eq!(pressed(&mut picker, &keys), answer("30 days"));
}

#[test]
fn escape_on_the_options_or_an_open_question_and_ctrl_c_anywhere_abandon() {
    let questions = questions();

    assert_eq!(
        Picker::new(closed(&questions)).press(Key::Back),
        Step::Abandoned
    );
    assert_eq!(
        Picker::new(open(&questions)).press(Key::Back),
        Step::Abandoned
    );
    let mut typing = Picker::new(closed(&questions));
    pressed(&mut typing, &[Key::Up, Key::Up, Key::Enter, Key::Char('x')]);
    assert_eq!(typing.press(Key::Quit), Step::Abandoned);
}

#[test]
fn the_options_show_their_descriptions_the_pointer_and_the_recommendation() {
    let questions = questions();
    let picker = Picker::new(closed(&questions));

    assert_eq!(
        shown(&picker, WIDE),
        "? 1/2 Log in by e-mail only, or by username too?\n      \
         why: changes the users table\n\
         \n      \
         e-mail only\n        \
         one login field\n    \
         \u{276f} e-mail or username  recommended\n        \
         adds a unique username column\n      \
         username only\n      \
         type another answer\n\
         \n  \
         \u{2191}/\u{2193} move \u{b7} enter choose \u{b7} 1-3 pick \u{b7} esc answer later"
    );
}

#[test]
fn typing_shows_the_text_with_a_bar_and_an_empty_open_answer_shows_the_recommendation() {
    let questions = questions();
    let mut other = Picker::new(closed(&questions));
    let mut keys = vec![Key::Up, Key::Up, Key::Enter];
    keys.extend(typed("SSO"));
    pressed(&mut other, &keys);

    let lines = shown(&other, WIDE);
    assert!(
        lines.contains("    \u{276f} type another answer: SSO\u{258c}\n"),
        "{lines}"
    );
    assert!(
        lines.ends_with("enter confirm \u{b7} esc back to the options"),
        "{lines}"
    );

    let lines = shown(&Picker::new(open(&questions)), WIDE);
    assert!(
        lines.contains("    \u{276f} \u{258c}7 days, no refresh\n"),
        "{lines}"
    );
    assert!(lines.contains("empty takes the recommendation"), "{lines}");
}

#[test]
fn no_line_is_wider_than_the_terminal_so_the_redraw_can_count_them() {
    let questions = questions();
    for columns in [24, 30, 40, 60] {
        let picker = Picker::new(closed(&questions));
        for line in lines(&picker, AT, columns, &Palette::plain()) {
            assert!(line.chars().count() < columns, "{columns}: {line:?}");
        }
    }
}

#[test]
fn an_answered_question_shrinks_to_its_first_line_and_the_answer() {
    let questions = questions();

    let summary = answered(closed(&questions), "SSO only", AT, 40, &Palette::plain());

    assert_eq!(
        summary,
        [
            "\u{2714} 1/2 Log in by e-mail only, or by\u{2026}",
            "      \u{2192} SSO only"
        ]
    );
}

#[test]
fn wrapping_breaks_between_words_and_splits_a_word_longer_than_the_line() {
    assert_eq!(wrap("one two three", 7), ["one two", "three"]);
    assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
    assert_eq!(wrap("", 4), [""]);
}
