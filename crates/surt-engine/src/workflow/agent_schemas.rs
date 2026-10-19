use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::files::AgentFiles;
use super::references::{Reference, references};
use super::schemas::compile_schema;

pub(super) fn check(id: &str, files: &AgentFiles) -> Result<(), Box<InvalidDefinition>> {
    let invalid = |path: &str, problem| {
        InvalidDefinition::new(
            DefinitionFile::AgentFile {
                agent: id.to_owned(),
                path: path.to_owned(),
            },
            None,
            problem,
        )
    };
    let refused = files.schemas.iter().find_map(|(path, text)| {
        reference_problem(path, text, files).map(|problem| (path, problem))
    });
    if let Some((path, problem)) = refused {
        return Err(invalid(path, problem));
    }
    for (path, text) in &files.schemas {
        compile_schema(path, text, &|path| files.schema(path))
            .map_err(|detail| invalid(path, Problem::InvalidSchema { detail }))?;
    }
    Ok(())
}

fn reference_problem(path: &str, text: &str, files: &AgentFiles) -> Option<Problem> {
    references(path, text)
        .into_iter()
        .find_map(|reference| match reference {
            Reference::Outside(reference) => Some(Problem::NonLocalReference { reference }),
            Reference::Local(target) if files.schema(&target).is_none() => {
                Some(Problem::MissingAsset { path: target })
            }
            Reference::Local(_) => None,
        })
}
