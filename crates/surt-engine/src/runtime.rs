#[cfg_attr(
    not(all(test, feature = "test-support")),
    expect(
        dead_code,
        reason = "drive accepts or rejects every Execution from ticket 08 on"
    )
)]
mod acceptance;
mod claude_code;
mod error;
#[cfg(feature = "test-support")]
mod fake;
mod launch;
mod protocol;
mod stop;

use std::future::Future;

#[cfg_attr(
    not(all(test, feature = "test-support")),
    expect(
        unused_imports,
        reason = "drive accepts or rejects every Execution from ticket 08 on"
    )
)]
pub(crate) use acceptance::{Rejection, Verdict, accept};
pub use claude_code::ClaudeCodeRuntime;
pub use error::{FailureClass, RuntimeError};
#[cfg(feature = "test-support")]
pub use fake::{Behavior, Delivery, FakeRuntime, Script};
pub use launch::AgentLaunch;
pub use protocol::{
    Exit, FinalResult, Initialization, RuntimeCompletion, RuntimeEvent, RuntimeEventSink,
    RuntimeRequest, StepAttempt,
};
pub use stop::{StopLevel, StopSignal};

pub trait AgentRuntime: Send + Sync {
    fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> impl Future<Output = Result<RuntimeCompletion, RuntimeError>> + Send;
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
