use super::super::{ArtifactFile, InitialArtifact};
use super::{InputProblem, Supplied, Value};
use crate::project::{Commit, Error, Project};
use crate::workflow::InputFormat;

#[derive(Default)]
pub(in crate::project::create_run) struct BuiltInputs {
    pub(in crate::project::create_run) artifacts: Vec<InitialArtifact>,
    pub(in crate::project::create_run) carried: Vec<String>,
    pub(in crate::project::create_run) git_refs: Vec<(String, Commit)>,
}

pub(in crate::project::create_run) async fn build(
    project: &Project,
    supplied: Vec<Supplied<'_>>,
    refuse: impl Fn(InputProblem) -> Error,
) -> Result<BuiltInputs, Error> {
    let mut built = BuiltInputs::default();
    for input in supplied {
        match input.value {
            Value::Carried => built.carried.push(input.name.to_owned()),
            Value::Given(text) | Value::Default(text) => {
                let bytes = built
                    .bytes(project, &input, text)
                    .await?
                    .ok_or_else(|| refuse(unresolved(&input, text)))?;
                built
                    .artifacts
                    .push(single_file(input.name, input.format, bytes));
            }
        }
    }
    Ok(built)
}

impl BuiltInputs {
    async fn bytes(
        &mut self,
        project: &Project,
        input: &Supplied<'_>,
        text: &str,
    ) -> Result<Option<Vec<u8>>, Error> {
        if input.format != InputFormat::GitRef {
            return Ok(Some(text.as_bytes().to_vec()));
        }
        let Some(commit) = project.resolve_commit(text).await? else {
            return Ok(None);
        };
        let bytes = git_ref_json(&commit);
        self.git_refs.push((input.name.to_owned(), commit));
        Ok(Some(bytes))
    }
}

fn unresolved(input: &Supplied<'_>, reference: &str) -> InputProblem {
    InputProblem::Unresolved {
        name: input.name.to_owned(),
        reference: reference.to_owned(),
    }
}

fn git_ref_json(commit: &Commit) -> Vec<u8> {
    serde_json::json!({ "ref": commit.branch, "sha": commit.sha })
        .to_string()
        .into_bytes()
}

fn single_file(name: &str, format: InputFormat, bytes: Vec<u8>) -> InitialArtifact {
    InitialArtifact {
        name: name.to_owned(),
        files: vec![ArtifactFile {
            rel_path: format!("{name}.{}", format.extension()),
            bytes,
        }],
    }
}
