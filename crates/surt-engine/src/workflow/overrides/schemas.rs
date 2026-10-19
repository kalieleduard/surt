use super::{InvalidOverride, LocalAgent};
use crate::workflow::error::Problem;
use crate::workflow::references::{Reference, references};
use crate::workflow::sources::AgentSources;

pub(super) fn check_references(
    local: &LocalAgent,
    sources: &AgentSources,
) -> Result<(), Box<InvalidOverride>> {
    for (path, file) in &local.schemas {
        let refused =
            references(path, &file.text)
                .into_iter()
                .find_map(|reference| match reference {
                    Reference::Outside(reference) => Some(Problem::NonLocalReference { reference }),
                    Reference::Local(target) if sources.schema(&target).is_none() => {
                        Some(Problem::MissingAsset { path: target })
                    }
                    Reference::Local(_) => None,
                });
        if let Some(problem) = refused {
            return Err(InvalidOverride::new(&file.file, None, problem));
        }
    }
    Ok(())
}
