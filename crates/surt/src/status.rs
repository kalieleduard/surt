use std::path::Path;

use surt_engine::project::{Catalog, Project};

use crate::failure::Failure;

pub(crate) async fn run(cwd: &Path) -> Result<(), Failure> {
    Project::open(cwd, Catalog::new()).await?;
    Ok(())
}
