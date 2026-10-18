use std::path::Path;

use surt_engine::project::Project;

use crate::builtin;
use crate::failure::Failure;

pub(crate) async fn run(cwd: &Path) -> Result<(), Failure> {
    Project::open(cwd, builtin::catalog()).await?;
    Ok(())
}
