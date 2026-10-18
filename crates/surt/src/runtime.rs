use surt_engine::project::Project;
#[cfg(feature = "test-support")]
use surt_engine::runtime::FakeRuntime;
use surt_engine::runtime::{
    AgentRuntime, ClaudeCodeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, StopSignal,
};

use crate::failure::Failure;

const CLAUDE_CODE: &str = "claude";

pub(crate) enum Selected {
    Claude(ClaudeCodeRuntime),
    #[cfg(feature = "test-support")]
    Fake(FakeRuntime),
}

impl AgentRuntime for Selected {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        match *self {
            Selected::Claude(ref claude) => claude.execute(request, events, stop).await,
            #[cfg(feature = "test-support")]
            Selected::Fake(ref fake) => fake.execute(request, events, stop).await,
        }
    }
}

pub(crate) fn select(project: &Project) -> Result<Selected, Failure> {
    #[cfg(feature = "test-support")]
    if let Some(script) = std::env::var_os(fake::VARIABLE) {
        return fake::load(std::path::Path::new(&script)).map(Selected::Fake);
    }
    Ok(Selected::Claude(ClaudeCodeRuntime::new(
        CLAUDE_CODE,
        std::env::vars_os(),
        project.stop_grace_period(),
    )))
}

#[cfg(feature = "test-support")]
mod fake {
    use std::path::Path;

    use surt_engine::runtime::{FakeRuntime, Script};

    use crate::failure::Failure;

    pub(super) const VARIABLE: &str = "SURT_RUNTIME";

    pub(super) fn load(path: &Path) -> Result<FakeRuntime, Failure> {
        let text = std::fs::read_to_string(path).map_err(|error| {
            Failure::Internal(format!("cannot read {VARIABLE}={}: {error}", path.display()).into())
        })?;
        let script =
            Script::from_toml(&text).map_err(|error| Failure::Internal(Box::new(error)))?;
        Ok(FakeRuntime::new(script))
    }
}
