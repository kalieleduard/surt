use super::super::source::{Files, Format, render};
use super::super::stream::Stream;
use crate::symbols::Symbols;

macro_rules! captured {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../surt-engine/tests/fixtures/claude-code/",
            $name,
            ".stdout.jsonl"
        ))
    };
}

fn rendered(stream: &str) -> Vec<String> {
    let mut renderer = Stream::default();
    stream
        .lines()
        .filter_map(|line| renderer.render(line))
        .collect()
}

#[test]
fn a_successful_stream_shows_the_tool_call_relative_to_the_session_and_the_delivery() {
    assert_eq!(
        rendered(captured!("success")),
        ["Write delivery/note.md", "✔ delivered"]
    );
}

#[test]
fn a_failed_authentication_shows_the_assistant_message_and_no_delivery() {
    assert_eq!(
        rendered(captured!("auth-expired")),
        ["\"Failed to authenticate: OAuth session expired and could not be refreshed\""]
    );
}

#[test]
fn a_nonexistent_model_shows_the_whole_first_line_of_the_message() {
    assert_eq!(
        rendered(captured!("bad-model")),
        [
            "\"There's an issue with the selected model (claude-nonexistent-0-0). \
             It may not exist or you may not have access to it. \
             Run --model to pick a different model.\""
        ]
    );
}

#[test]
fn a_stream_cut_by_sigterm_shows_what_happened_before_it() {
    assert_eq!(rendered(captured!("sigterm")), ["Write delivery/a.md"]);
}

#[test]
fn a_multi_line_message_shows_its_first_line_and_an_ellipsis_skipping_tool_results() {
    assert_eq!(
        rendered(captured!("permission-denied")),
        [
            "Write delivery/note.md",
            "\"I couldn't create `delivery/note.md`. The Write call was denied because the \
             session is running in \"don't ask\" permission mode, so nothing has been written \
             and nothing has been delivered.…\"",
            "✔ delivered"
        ]
    );
}

#[test]
fn relative_paths_stay_as_the_agent_wrote_them() {
    assert_eq!(
        rendered(captured!("max-turns")),
        [
            "Write delivery/a.md",
            "Write delivery/b.md",
            "Write delivery/c.md",
            "Write delivery/d.md",
            "Write delivery/note.md"
        ]
    );
}

#[test]
fn every_captured_stream_prints_only_ascii_in_ascii_mode() {
    let streams = [
        captured!("success"),
        captured!("auth-expired"),
        captured!("bad-model"),
        captured!("sigterm"),
        captured!("permission-denied"),
        captured!("max-turns"),
    ];

    for stream in streams {
        let files = Files {
            stream: stream.to_owned(),
            stderr: String::new(),
            output: String::new(),
        };
        let text = render(&files, "B build#2 │ ", Format::Rendered);

        assert!(Symbols::Ascii.render(&text).is_ascii(), "{text}");
    }
}

#[test]
fn an_edit_and_a_cut_message_render_their_glyphs_in_ascii() {
    let mut renderer = Stream::default();
    renderer.render(r#"{"type":"system","subtype":"init","cwd":"/w"}"#);
    let edit = renderer.render(
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/w/a.rs","old_string":"a","new_string":"b\nc"}}]}}"#,
    );
    let message = renderer.render(
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"first\nsecond"}]}}"#,
    );

    assert_eq!(
        Symbols::Ascii.render(&edit.expect("an edit line")),
        "Edit a.rs (+2 -1)"
    );
    assert_eq!(
        Symbols::Ascii.render(&message.expect("a message line")),
        "\"first...\""
    );
}
