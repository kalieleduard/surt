use serde_json::{Value, json};
use uuid::Uuid;

use crate::runtime::Exit;

macro_rules! captured {
    ($name:literal, $kind:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/claude-code/",
            $name,
            ".",
            $kind
        ))
    };
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Fixture {
    Success,
    AuthExpired,
    BadModel,
    Sigterm,
    SchemaMiss,
    PermissionDenied,
    MaxTurns,
    ResumeWithoutTranscript,
}

impl Fixture {
    fn stdout(self) -> &'static str {
        match self {
            Fixture::Success => captured!("success", "stdout.jsonl"),
            Fixture::AuthExpired => captured!("auth-expired", "stdout.jsonl"),
            Fixture::BadModel => captured!("bad-model", "stdout.jsonl"),
            Fixture::Sigterm => captured!("sigterm", "stdout.jsonl"),
            Fixture::SchemaMiss => captured!("schema-miss", "stdout.jsonl"),
            Fixture::PermissionDenied => captured!("permission-denied", "stdout.jsonl"),
            Fixture::MaxTurns => captured!("max-turns", "stdout.jsonl"),
            Fixture::ResumeWithoutTranscript => {
                captured!("resume-without-transcript", "stdout.jsonl")
            }
        }
    }

    fn meta(self) -> &'static str {
        match self {
            Fixture::Success => captured!("success", "meta.txt"),
            Fixture::AuthExpired => captured!("auth-expired", "meta.txt"),
            Fixture::BadModel => captured!("bad-model", "meta.txt"),
            Fixture::Sigterm => captured!("sigterm", "meta.txt"),
            Fixture::SchemaMiss => captured!("schema-miss", "meta.txt"),
            Fixture::PermissionDenied => captured!("permission-denied", "meta.txt"),
            Fixture::MaxTurns => captured!("max-turns", "meta.txt"),
            Fixture::ResumeWithoutTranscript => captured!("resume-without-transcript", "meta.txt"),
        }
    }

    pub(super) fn exit(self) -> Exit {
        let code = self
            .meta()
            .lines()
            .find_map(|line| line.strip_prefix("exit_code="))
            .expect("every capture records its exit code");
        Exit::Code(code.parse().expect("a numeric exit code"))
    }
}

#[derive(Clone, Debug)]
pub(super) struct Stream {
    events: Vec<Value>,
    session_id: Uuid,
}

impl Stream {
    pub(super) fn captured(fixture: Fixture) -> Stream {
        let events = fixture
            .stdout()
            .lines()
            .map(|line| serde_json::from_str(line).expect("captured lines are JSON"))
            .collect();
        let mut stream = Stream {
            events,
            session_id: Uuid::nil(),
        };
        let session = stream
            .events
            .iter()
            .find_map(|event| event["session_id"].as_str())
            .expect("an event carries the session id");
        stream.session_id = Uuid::parse_str(session).expect("a UUID session id");
        stream
    }

    pub(super) fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub(super) fn with_init(mut self, field: &str, value: Value) -> Stream {
        self.init_mut()[field] = value;
        self
    }

    pub(super) fn without_init(mut self) -> Stream {
        self.events
            .retain(|event| !is(event, "system", Some("init")));
        self
    }

    pub(super) fn without_result(mut self) -> Stream {
        self.events.retain(|event| !is(event, "result", None));
        self
    }

    pub(super) fn with_envelope(mut self, envelope: Value) -> Stream {
        self.result_mut()["structured_output"] = envelope;
        self
    }

    pub(super) fn with_result(mut self, field: &str, value: Value) -> Stream {
        self.result_mut()[field] = value;
        self
    }

    pub(super) fn with_assistant_error(mut self, category: &str) -> Stream {
        self.events
            .iter_mut()
            .filter(|event| event.get("error").is_some())
            .for_each(|event| event["error"] = json!(category));
        self
    }

    pub(super) fn before_result(mut self, event: Value) -> Stream {
        let at = self
            .events
            .iter()
            .position(|event| is(event, "result", None))
            .expect("a result event");
        self.events.insert(at, event);
        self
    }

    pub(super) fn first(mut self, event: Value) -> Stream {
        self.events.insert(0, event);
        self
    }

    pub(super) fn last(mut self, event: Value) -> Stream {
        self.events.push(event);
        self
    }

    pub(super) fn lines(&self) -> Vec<String> {
        self.events.iter().map(Value::to_string).collect()
    }

    fn init_mut(&mut self) -> &mut Value {
        self.events
            .iter_mut()
            .find(|event| is(event, "system", Some("init")))
            .expect("an init event")
    }

    fn result_mut(&mut self) -> &mut Value {
        self.events
            .iter_mut()
            .find(|event| is(event, "result", None))
            .expect("a result event")
    }
}

pub(super) fn delivered_note() -> Stream {
    Stream::captured(Fixture::Success)
        .with_envelope(json!({ "outcome": "done", "outputs": { "note": "note.md" } }))
}

fn is(event: &Value, kind: &str, subtype: Option<&str>) -> bool {
    event["type"] == kind && subtype.is_none_or(|subtype| event["subtype"] == subtype)
}
