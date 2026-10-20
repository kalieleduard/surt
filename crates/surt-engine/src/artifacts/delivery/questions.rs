mod problem;

use std::collections::HashSet;

use serde::Deserialize;
use serde_json::Value;

use super::portable::{KeyFault, first_key_fault};
use super::{DeliveredFile, DeliveryRejection, ValidatedOutput};

pub(crate) use problem::{QuestionField, QuestionsProblem};

pub const MAX_QUESTIONS: usize = 5;

const MIN_OPTIONS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Questions(Vec<Question>);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    id: String,
    question: String,
    why: String,
    recommended: String,
    #[serde(default)]
    options: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    questions: Vec<Value>,
}

impl Questions {
    pub fn parse(bytes: &[u8]) -> Option<Questions> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        Questions::from_value(&value).ok()
    }

    pub fn questions(&self) -> &[Question] {
        &self.0
    }

    fn from_value(value: &Value) -> Result<Questions, QuestionsProblem> {
        let document = Document::deserialize(value).map_err(|_| QuestionsProblem::NotQuestions)?;
        check_count(document.questions.len())?;
        let questions = document
            .questions
            .iter()
            .enumerate()
            .map(|(index, element)| {
                Question::deserialize(element)
                    .map_err(|_| QuestionsProblem::MalformedQuestion { index })
            })
            .collect::<Result<Vec<_>, _>>()?;
        check_ids(&questions)?;
        questions.iter().try_for_each(Question::check)?;
        Ok(Questions(questions))
    }
}

impl Question {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn question(&self) -> &str {
        &self.question
    }

    pub fn why(&self) -> &str {
        &self.why
    }

    pub fn recommended(&self) -> &str {
        &self.recommended
    }

    pub fn options(&self) -> Option<&[String]> {
        self.options.as_deref()
    }

    fn check(&self) -> Result<(), QuestionsProblem> {
        if let Some(field) = self.empty_field() {
            return Err(QuestionsProblem::EmptyText {
                id: self.id.clone(),
                field,
            });
        }
        match &self.options {
            Some(options) => self.check_options(options),
            None => Ok(()),
        }
    }

    fn empty_field(&self) -> Option<QuestionField> {
        [
            (QuestionField::Question, &self.question),
            (QuestionField::Why, &self.why),
            (QuestionField::Recommended, &self.recommended),
        ]
        .into_iter()
        .find_map(|(field, text)| is_blank(text).then_some(field))
    }

    fn check_options(&self, options: &[String]) -> Result<(), QuestionsProblem> {
        let id = || self.id.clone();
        if options.len() < MIN_OPTIONS {
            return Err(QuestionsProblem::TooFewOptions { id: id() });
        }
        if options.iter().any(|option| is_blank(option)) {
            return Err(QuestionsProblem::EmptyOption { id: id() });
        }
        let mut seen = HashSet::new();
        if let Some(option) = options.iter().find(|option| !seen.insert(option.as_str())) {
            return Err(QuestionsProblem::DuplicateOption {
                id: id(),
                option: option.clone(),
            });
        }
        if !seen.contains(self.recommended.as_str()) {
            return Err(QuestionsProblem::RecommendationNotOffered { id: id() });
        }
        Ok(())
    }
}

pub(super) fn validate_questions(
    file: DeliveredFile,
    value: Value,
) -> Result<ValidatedOutput, DeliveryRejection> {
    match Questions::from_value(&value) {
        Ok(_) => Ok(ValidatedOutput {
            files: vec![file],
            json: Some(value),
        }),
        Err(problem) => Err(DeliveryRejection::InvalidQuestions {
            path: file.path,
            problem,
        }),
    }
}

fn check_count(count: usize) -> Result<(), QuestionsProblem> {
    match count {
        0 => Err(QuestionsProblem::NoQuestions),
        count if count > MAX_QUESTIONS => Err(QuestionsProblem::TooManyQuestions { count }),
        _ => Ok(()),
    }
}

fn check_ids(questions: &[Question]) -> Result<(), QuestionsProblem> {
    let ids = questions.iter().map(|question| question.id.as_str());
    let Some((id, fault)) = first_key_fault(ids) else {
        return Ok(());
    };
    let id = id.to_owned();
    Err(match fault {
        KeyFault::NotPortable => QuestionsProblem::InvalidId { id },
        KeyFault::Reserved => QuestionsProblem::ReservedId { id },
        KeyFault::Duplicate => QuestionsProblem::DuplicateId { id },
    })
}

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}
