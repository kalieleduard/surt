mod acceptance;
mod claude_code;
mod error;
#[cfg(feature = "test-support")]
mod fake;
mod launch;
mod log_files;
mod protocol;
mod report;
mod stop;

use std::future::Future;

pub(crate) use acceptance::{Rejection, Verdict, accept};
pub use claude_code::ClaudeCodeRuntime;
pub use error::{FailureClass, RuntimeError};
#[cfg(feature = "test-support")]
pub use fake::{Behavior, Delivery, FakeRuntime, InvalidScript, Script};
pub use launch::AgentLaunch;
pub use log_files::LogFiles;
pub(crate) use log_files::{OUTPUT_LOG, STDERR_LOG, STREAM_LOG};
pub use protocol::{
    Exit, FinalResult, Initialization, RuntimeCompletion, RuntimeEvent, RuntimeEventSink,
    RuntimeRequest, StepAttempt,
};
pub use report::RuntimeReport;
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
