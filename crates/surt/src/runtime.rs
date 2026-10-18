#[cfg(feature = "test-support")]
use surt_engine::runtime::FakeRuntime;
use surt_engine::runtime::{
    AgentRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink, RuntimeRequest, StopSignal,
};

use crate::failure::Failure;
use crate::message::Message;

pub(crate) enum Selected {
    #[cfg(feature = "test-support")]
    Fake(FakeRuntime),
}

impl AgentRuntime for Selected {
    #[cfg_attr(
        not(feature = "test-support"),
        expect(
            unused_variables,
            reason = "a build without test-support has no agent runtime to hand the request to"
        )
    )]
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        match *self {
            #[cfg(feature = "test-support")]
            Selected::Fake(ref fake) => fake.execute(request, events, stop).await,
        }
    }
}

pub(crate) fn select() -> Result<Selected, Failure> {
    #[cfg(feature = "test-support")]
    if let Some(script) = std::env::var_os(fake::VARIABLE) {
        return fake::load(std::path::Path::new(&script)).map(Selected::Fake);
    }
    Err(Failure::Refused(
        Message::new("this surt has no agent runtime").way("fix", "install a release of surt"),
    ))
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
