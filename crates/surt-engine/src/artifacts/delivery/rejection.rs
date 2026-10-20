use super::item_list::ItemListProblem;
use super::questions::QuestionsProblem;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DeliveryRejection {
    #[error("no result envelope was delivered")]
    MissingEnvelope,
    #[error("the result envelope is not valid JSON with an outcome and outputs")]
    InvalidEnvelope,
    #[error("outcome {outcome} is not declared")]
    UnknownOutcome { outcome: String },
    #[error("output {name} is missing")]
    MissingOutput { name: String },
    #[error("output {name} is not declared")]
    UndeclaredOutput { name: String },
    #[error("{path} is not a safe relative path")]
    UnsafePath { path: String },
    #[error("{path} is a link")]
    Link { path: String },
    #[error("{path} is missing")]
    MissingFile { path: String },
    #[error("{path} is not a regular file")]
    NotRegularFile { path: String },
    #[error("{path} is empty")]
    EmptyFile { path: String },
    #[error("{path} is not UTF-8")]
    NotUtf8 { path: String },
    #[error("{path} is not valid JSON: {detail}")]
    InvalidJson { path: String, detail: String },
    #[error("{path} does not match its schema: {detail}")]
    SchemaViolation { path: String, detail: String },
    #[error("{path}: {problem}")]
    InvalidItemList {
        path: String,
        problem: ItemListProblem,
    },
    #[error("{path}: {problem}")]
    InvalidQuestions {
        path: String,
        problem: QuestionsProblem,
    },
}
