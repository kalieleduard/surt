mod create_run;
mod error;

pub use self::create_run::{ArtifactFile, InitialArtifact, NewRun, RunId};
pub use self::error::{Error, InternalError, Refusal};
pub use crate::workflow::{
    AgentAssets, Catalog, DefinitionFile, InvalidDefinition, Problem, StepField,
};

use crate::store::Store;
use crate::workflow::RunConfig;

pub struct Project {
    store: Store,
    catalog: Catalog,
    config: RunConfig,
}

#[cfg(test)]
mod tests;
