use serde::{Deserialize, Serialize};

use super::agent::Format;
use super::error::Problem;

mod declared;

pub(crate) use declared::DeclaredInputs;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputFormat {
    Text,
    Markdown,
    Json,
    GitRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputSpec {
    pub(crate) format: InputFormat,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubjectSpec {
    pub(crate) from: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RunInputs {
    pub(crate) declared: DeclaredInputs,
    pub(crate) subject: Option<String>,
}

impl InputFormat {
    pub fn name(self) -> &'static str {
        match self {
            InputFormat::Text => "text",
            InputFormat::Markdown => "markdown",
            InputFormat::Json => "json",
            InputFormat::GitRef => "git_ref",
        }
    }

    pub(crate) fn artifact_format(self) -> Format {
        match self {
            InputFormat::Text => Format::Text,
            InputFormat::Markdown => Format::Markdown,
            InputFormat::Json | InputFormat::GitRef => Format::Json,
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            InputFormat::Text => "txt",
            InputFormat::Markdown => "md",
            InputFormat::Json | InputFormat::GitRef => "json",
        }
    }

    fn names_a_subject(self) -> bool {
        matches!(self, InputFormat::Text | InputFormat::Markdown)
    }
}

impl RunInputs {
    pub(crate) fn new(
        declared: DeclaredInputs,
        subject: Option<SubjectSpec>,
    ) -> Result<RunInputs, Problem> {
        if let Some((name, _)) = declared
            .iter()
            .find(|(_, spec)| spec.default.is_some() && spec.format != InputFormat::GitRef)
        {
            return Err(Problem::MisplacedInputDefault {
                input: name.to_owned(),
            });
        }
        let subject = subject.map(|subject| subject.from);
        if let Some(from) = &subject {
            check_subject(&declared, from)?;
        }
        Ok(RunInputs { declared, subject })
    }

    pub(crate) fn git_refs(&self) -> impl Iterator<Item = &str> {
        self.declared
            .iter()
            .filter(|(_, spec)| spec.format == InputFormat::GitRef)
            .map(|(name, _)| name)
    }

    pub(crate) fn subject_spec(&self) -> Option<SubjectSpec> {
        self.subject.clone().map(|from| SubjectSpec { from })
    }
}

fn check_subject(declared: &DeclaredInputs, from: &str) -> Result<(), Problem> {
    match declared.get(from) {
        None => Err(Problem::UnknownSubjectInput {
            input: from.to_owned(),
        }),
        Some(spec) if !spec.format.names_a_subject() => Err(Problem::SubjectInputNotText {
            input: from.to_owned(),
            format: spec.format,
        }),
        Some(_) => Ok(()),
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
