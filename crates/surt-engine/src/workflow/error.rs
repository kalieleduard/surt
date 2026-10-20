use super::inputs::InputFormat;

pub(crate) const WORKFLOWS_DIR: &str = ".surt/workflows";
pub(crate) const AGENTS_DIR: &str = ".surt/agents";
pub(crate) const AGENT_FILE: &str = "agent.toml";
pub(crate) const INSTRUCTIONS_FILE: &str = "instructions.md";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidDefinition {
    pub file: DefinitionFile,
    pub field: Option<String>,
    pub step: Option<String>,
    pub problem: Problem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinitionFile {
    Workflow(String),
    Agent(String),
    AgentFile { agent: String, path: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepField {
    Needs,
    Body,
    When,
    Until,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Syntax {
        line: Option<usize>,
        detail: String,
    },
    UnsupportedSchema {
        found: i64,
    },
    NameMismatch {
        found: String,
    },
    MissingKind,
    UnknownKind {
        kind: String,
    },
    InvalidStep {
        detail: String,
    },
    UnknownWorkflow {
        name: String,
    },
    RecursiveWorkflow {
        name: String,
    },
    UnknownAgent {
        name: String,
    },
    UnknownOperation {
        name: String,
    },
    UnknownStep {
        field: StepField,
        name: String,
    },
    OutOfScope {
        field: StepField,
        name: String,
    },
    SharedBodyStep {
        name: String,
    },
    NestedLoop {
        name: String,
    },
    Cycle {
        path: Vec<String>,
    },
    UntilOutsideBody {
        name: String,
    },
    UnknownOutcome {
        field: StepField,
        step: String,
        outcome: String,
        declared: Vec<String>,
    },
    UnknownAgentInput {
        input: String,
    },
    MissingInput {
        input: String,
    },
    UnproducedArtifact {
        artifact: String,
    },
    NotAnItemList {
        artifact: String,
    },
    DuplicateRejectionArtifact {
        artifact: String,
    },
    MissingAsset {
        path: String,
    },
    NonLocalReference {
        reference: String,
    },
    InvalidSchema {
        detail: String,
    },
    MisplacedJsonSchema {
        name: String,
    },
    MisplacedItemFiles {
        name: String,
    },
    UnsupportedItemFileFormat {
        name: String,
    },
    MissingClaudeProfile,
    MisplacedApiKeyEnv,
    InvalidBudget,
    MissingFile,
    StrayAgentFile,
    TestOnlyRuntime,
    MissingCheckedOutput {
        output: String,
    },
    MissingCheckedOutcome {
        outcome: String,
    },
    MisplacedInputDefault {
        input: String,
    },
    UnknownSubjectInput {
        input: String,
    },
    SubjectInputNotText {
        input: String,
        format: InputFormat,
    },
    InputsInChildWorkflow,
    GitInputNotGitRef {
        input: String,
    },
    ConversationOutsideLoop {
        agent: String,
    },
}

impl InvalidDefinition {
    pub(crate) fn new(file: DefinitionFile, step: Option<&str>, problem: Problem) -> Box<Self> {
        Box::new(InvalidDefinition {
            file,
            field: None,
            step: step.map(str::to_owned),
            problem,
        })
    }

    pub(crate) fn with_field(mut self: Box<Self>, field: Option<String>) -> Box<Self> {
        self.field = field;
        self
    }
}

impl DefinitionFile {
    pub fn path(&self) -> String {
        match self {
            DefinitionFile::Workflow(name) => format!("{WORKFLOWS_DIR}/{name}.toml"),
            DefinitionFile::Agent(id) => format!("{AGENTS_DIR}/{id}/{AGENT_FILE}"),
            DefinitionFile::AgentFile { agent, path } => format!("{AGENTS_DIR}/{agent}/{path}"),
        }
    }
}

impl Problem {
    pub(crate) fn syntax(text: &str, error: &toml::de::Error) -> Problem {
        Problem::Syntax {
            line: error
                .span()
                .and_then(|span| text.get(..span.start))
                .map(|before| before.matches('\n').count() + 1),
            detail: error.message().to_owned(),
        }
    }
}
