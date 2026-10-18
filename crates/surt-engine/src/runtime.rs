mod acceptance;
mod error;
#[cfg(feature = "test-support")]
mod fake;
mod protocol;
mod stop;

use std::future::Future;

pub(crate) use acceptance::{Rejection, Verdict, accept};
pub use error::{FailureClass, RuntimeError};
#[cfg(feature = "test-support")]
pub use fake::{Behavior, Delivery, FakeRuntime, InvalidScript, Script};
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
