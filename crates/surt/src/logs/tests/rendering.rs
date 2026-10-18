use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use surt_engine::project::LogSource;

use super::super::source::{Files, Offsets, prefixes, render};
use super::super::stream::Stream;

const INIT: &str = r#"{"type":"system","subtype":"init","cwd":"/work/tree","model":"m"}"#;

fn one(line: &str) -> Option<String> {
    let mut stream = Stream::default();
    stream.render(INIT);
    stream.render(line)
}

fn tool(name: &str, input: &str) -> Option<String> {
    one(&format!(
        r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t","name":"{name}","input":{input}}}]}}}}"#
    ))
}

fn result(structured: &str) -> Option<String> {
    one(&format!(
        r#"{{"type":"result","subtype":"success","is_error":false,"structured_output":{structured}}}"#
    ))
}

fn source(item: Option<&str>, step: &str, attempt: u32) -> LogSource {
    LogSource {
        execution: "0192f3a8-7b1c-7000-8000-4a1f0c3e9b21".to_owned(),
        subject: "20261017-add-login".to_owned(),
        run: 1,
        item: item.map(str::to_owned),
        step: step.to_owned(),
        iteration: Some(1),
        attempt,
        state: "running".to_owned(),
        directory: PathBuf::from("logs"),
    }
}

#[test]
fn tool_calls_show_their_main_argument() {
    assert_eq!(
        tool("Read", r#"{"file_path":"/work/tree/src/auth.rs"}"#).as_deref(),
        Some("Read src/auth.rs")
    );
    assert_eq!(
        tool(
            "Bash",
            r#"{"command":"cargo test -p auth\necho done","timeout":5}"#
        )
        .as_deref(),
        Some("Bash cargo test -p auth…")
    );
    assert_eq!(
        tool("Grep", r#"{"pattern":"fn login","path":"/work/tree/src"}"#).as_deref(),
        Some("Grep fn login")
    );
    assert_eq!(
        tool("Read", r#"{"file_path":"/elsewhere/notes.md"}"#).as_deref(),
        Some("Read /elsewhere/notes.md")
    );
}

#[test]
fn an_edit_counts_the_lines_it_adds_and_removes() {
    assert_eq!(
        tool(
            "Edit",
            r#"{"file_path":"/work/tree/src/auth.rs","old_string":"a\nb\nc","new_string":"a\nB\nC\nD\ne"}"#
        )
        .as_deref(),
        Some("Edit src/auth.rs (+5 −3)")
    );
    assert_eq!(
        tool(
            "MultiEdit",
            r#"{"file_path":"src/a.rs","edits":[{"old_string":"x","new_string":"y\nz"},{"old_string":"p\nq","new_string":""}]}"#
        )
        .as_deref(),
        Some("MultiEdit src/a.rs (+2 −3)")
    );
}

#[test]
fn a_tool_without_a_known_argument_shows_its_name_and_the_delivery_call_shows_nothing() {
    assert_eq!(
        tool("TodoWrite", r#"{"todos":[]}"#).as_deref(),
        Some("TodoWrite")
    );
    assert_eq!(tool("StructuredOutput", r#"{"outcome":"done"}"#), None);
}

#[test]
fn an_accepted_delivery_names_a_non_default_outcome_and_the_commit() {
    assert_eq!(
        result(r#"{"outcome":"done","outputs":{"commit":{"sha":"9e04b7d1c2a3"}}}"#).as_deref(),
        Some("✔ delivered (commit 9e04b7d)")
    );
    assert_eq!(
        result(r#"{"outcome":"changes_requested","outputs":{"feedback":{}}}"#).as_deref(),
        Some("✔ delivered (changes_requested)")
    );
    assert_eq!(
        one(r#"{"type":"result","subtype":"error_max_turns","is_error":true}"#),
        None
    );
}

#[test]
fn unknown_types_fields_tool_results_and_malformed_lines_render_nothing() {
    for line in [
        r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
        r#"{"type":"rate_limit_event","rate_limit_info":{}}"#,
        r#"{"type":"fake","event":"launched"}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":""}]}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"  \n"}]}}"#,
        r#"{"type":"assistant","message":{"content":"plain"}}"#,
        r#"{"type":"assistant"}"#,
        r#"{"type":42}"#,
        r#"["not","an","event"]"#,
        "not json at all",
        "",
    ] {
        assert_eq!(one(line), None, "{line}");
    }
    assert_eq!(
        one(r#"{"type":"assistant","future":1,"message":{"content":[{"type":"text","text":"hi","extra":true}]}}"#)
            .as_deref(),
        Some("\"hi\"")
    );
}

#[test]
fn a_single_source_has_no_prefix_and_several_are_aligned() {
    let spec = source(None, "spec", 1);
    let build = source(Some("B"), "build", 2);

    assert_eq!(prefixes(&[&spec]), [""]);
    assert_eq!(prefixes(&[&spec, &build]), ["spec#1    │ ", "B build#2 │ "]);
}

#[test]
fn a_source_renders_its_stream_then_passes_stderr_and_check_output_through() {
    let files = Files {
        stream: format!("{INIT}\n{}\n", r#"{"type":"tool_result"}"#),
        stderr: "warning: slow\n".to_owned(),
        output: "=== test: cargo test\nok\n".to_owned(),
    };
    let edit = Files {
        stream: format!(
            "{INIT}\n{}\n",
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/work/tree/a"}}]}}"#
        ),
        stderr: String::new(),
        output: String::new(),
    };

    assert_eq!(
        render(&files, "B check#1 │ ", false),
        "B check#1 │ warning: slow\nB check#1 │ === test: cargo test\nB check#1 │ ok\n"
    );
    assert_eq!(render(&edit, "", false), "Read a\n");
}

#[test]
fn raw_prints_only_the_original_stream_lines() {
    let files = Files {
        stream: format!("{INIT}\nnot json\n"),
        stderr: "warning\n".to_owned(),
        output: "=== test\n".to_owned(),
    };

    assert_eq!(
        render(&files, "B build#1 │ ", true),
        format!("{INIT}\nnot json\n")
    );
}

#[test]
fn appended_files_hold_back_an_unfinished_line_until_the_last_read() {
    let directory = tempfile::tempdir().expect("a directory");
    let stream = directory.path().join("stream.jsonl");
    fs::write(&stream, "first\nsecond").expect("a stream");
    fs::write(directory.path().join("output.log"), "=== test\n").expect("an output");
    let mut offsets = Offsets::default();

    let started = Files::appended(directory.path(), &mut offsets, false).expect("read");
    let mut file = OpenOptions::new().append(true).open(&stream).expect("open");
    write!(file, " half\nthird").expect("append");
    let continued = Files::appended(directory.path(), &mut offsets, false).expect("read");
    let last = Files::appended(directory.path(), &mut offsets, true).expect("read");

    let read = |files: &Files| {
        (
            files.stream.clone(),
            files.stderr.clone(),
            files.output.clone(),
        )
    };
    assert_eq!(
        read(&started),
        ("first\n".into(), String::new(), "=== test\n".into())
    );
    assert_eq!(
        read(&continued),
        ("second half\n".into(), String::new(), String::new())
    );
    assert_eq!(read(&last), ("third".into(), String::new(), String::new()));
}
