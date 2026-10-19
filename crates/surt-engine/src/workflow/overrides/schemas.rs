use super::{InvalidOverride, LocalAgent};
use crate::workflow::error::Problem;
use crate::workflow::references::{Reference, references};
use crate::workflow::schemas::compile_schema;
use crate::workflow::sources::AgentSources;

pub(super) fn check(
    local: &LocalAgent,
    sources: &AgentSources,
) -> Result<(), Box<InvalidOverride>> {
    let refused = local.schemas.iter().find_map(|(path, file)| {
        reference_problem(path, &file.text, sources).map(|problem| (file, problem))
    });
    if let Some((file, problem)) = refused {
        return Err(InvalidOverride::new(&file.file, None, problem));
    }
    for (path, file) in &local.schemas {
        compile_schema(path, &file.text, &|path| sources.schema(path)).map_err(|detail| {
            InvalidOverride::new(&file.file, None, Problem::InvalidSchema { detail })
        })?;
    }
    Ok(())
}

fn reference_problem(path: &str, text: &str, sources: &AgentSources) -> Option<Problem> {
    references(path, text)
        .into_iter()
        .find_map(|reference| match reference {
            Reference::Outside(reference) => Some(Problem::NonLocalReference { reference }),
            Reference::Local(target) if sources.schema(&target).is_none() => {
                Some(Problem::MissingAsset { path: target })
            }
            Reference::Local(_) => None,
        })
}
