use uuid::Uuid;

use super::{Exit, FailureClass, RuntimeCompletion, RuntimeRequest};
use crate::artifacts::{AcceptedDelivery, DeliveryContract, DeliveryRejection};

#[derive(Debug)]
pub(crate) enum Verdict {
    Accepted(AcceptedDelivery),
    Interrupted,
    Rejected(Rejection),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Rejection {
    #[error("the agent exited with code {0}")]
    ExitCode(i32),
    #[error("the agent never reported its initialization")]
    MissingInitialization,
    #[error("the agent reported session {reported}, not {expected}")]
    SessionMismatch { expected: Uuid, reported: Uuid },
    #[error("the agent ran model {reported}, not {requested}")]
    ModelMismatch { requested: String, reported: String },
    #[error("the agent ended without a final result")]
    MissingResult,
    #[error("the agent's final result is not a success")]
    UnsuccessfulResult,
    #[error(transparent)]
    Delivery(DeliveryRejection),
}

impl Rejection {
    pub(crate) fn class(&self) -> FailureClass {
        match self {
            Rejection::ModelMismatch { .. } => FailureClass::NotRetryable,
            _ => FailureClass::Retryable,
        }
    }
}

pub(crate) fn accept(
    request: &RuntimeRequest,
    completion: &RuntimeCompletion,
    contract: &DeliveryContract,
) -> Verdict {
    let code = match completion.exit {
        Exit::Stopped => return Verdict::Interrupted,
        Exit::Code(code) => code,
    };
    let checked = check_evidence(request, completion, code).and_then(|envelope| {
        contract
            .validate(&request.delivery_dir, envelope)
            .map_err(Rejection::Delivery)
    });
    match checked {
        Ok(delivery) => Verdict::Accepted(delivery),
        Err(rejection) => Verdict::Rejected(rejection),
    }
}

fn check_evidence<'a>(
    request: &RuntimeRequest,
    completion: &'a RuntimeCompletion,
    code: i32,
) -> Result<Option<&'a str>, Rejection> {
    if code != 0 {
        return Err(Rejection::ExitCode(code));
    }
    let initialization = completion
        .initialization
        .as_ref()
        .ok_or(Rejection::MissingInitialization)?;
    if initialization.session_id != request.session_id {
        return Err(Rejection::SessionMismatch {
            expected: request.session_id,
            reported: initialization.session_id,
        });
    }
    if initialization.model != request.model {
        return Err(Rejection::ModelMismatch {
            requested: request.model.clone(),
            reported: initialization.model.clone(),
        });
    }
    let result = completion.result.as_ref().ok_or(Rejection::MissingResult)?;
    if !result.succeeded {
        return Err(Rejection::UnsuccessfulResult);
    }
    Ok(result.structured_output.as_deref())
}
