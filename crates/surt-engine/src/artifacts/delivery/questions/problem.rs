use std::fmt;

use super::{MAX_QUESTIONS, MIN_OPTIONS};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum QuestionsProblem {
    #[error("the file is not an object holding a list of questions")]
    NotQuestions,
    #[error("there are no questions")]
    NoQuestions,
    #[error("{count} questions are more than {}", MAX_QUESTIONS)]
    TooManyQuestions { count: usize },
    #[error("question {} has no id, question, why or recommended of the right shape", index + 1)]
    MalformedQuestion { index: usize },
    #[error("id {id} is not a portable item key")]
    InvalidId { id: String },
    #[error("id {id} is a reserved device name")]
    ReservedId { id: String },
    #[error("id {id} is used by more than one question")]
    DuplicateId { id: String },
    #[error("question {id} has an empty {field}")]
    EmptyText { id: String, field: QuestionField },
    #[error("question {id} offers fewer than {} options", MIN_OPTIONS)]
    TooFewOptions { id: String },
    #[error("question {id} offers an empty option")]
    EmptyOption { id: String },
    #[error("question {id} offers {option} more than once")]
    DuplicateOption { id: String, option: String },
    #[error("question {id} recommends an answer that is not one of its options")]
    RecommendationNotOffered { id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuestionField {
    Question,
    Why,
    Recommended,
}

impl fmt::Display for QuestionField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            QuestionField::Question => "question",
            QuestionField::Why => "why",
            QuestionField::Recommended => "recommended",
        })
    }
}
