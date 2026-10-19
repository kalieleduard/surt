#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidDefinition {
    pub file: DefinitionFile,
    pub step: Option<String>,
    pub problem: Problem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinitionFile {
    Workflow(String),
    Agent(String),
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
    UnknownAgentDirectory {
        known: Vec<String>,
    },
    StrayAgentFile,
    TestOnlyRuntime,
    MissingCheckedOutput {
        output: String,
    },
    MissingCheckedOutcome {
        outcome: String,
    },
}

impl InvalidDefinition {
    pub(crate) fn new(file: DefinitionFile, step: Option<&str>, problem: Problem) -> Box<Self> {
        Box::new(InvalidDefinition {
            file,
            step: step.map(str::to_owned),
            problem,
        })
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
