use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::answer;
use crate::continue_run::continue_command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Moment {
    AtGates,
    Approved,
    Rejected,
    Blocked,
    Interrupted,
}

pub(crate) fn command(moment: Moment, subject: &str) -> String {
    match moment {
        Moment::AtGates => format!("surt approve {subject}"),
        Moment::Approved | Moment::Rejected | Moment::Blocked | Moment::Interrupted => {
            continue_command(subject)
        }
    }
}

pub(crate) struct Slot(Mutex<Option<String>>);

impl Slot {
    pub(crate) const fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub(crate) fn set(&self, command: String) {
        *self.lock() = Some(command);
    }

    pub(crate) fn take(&self) -> Option<String> {
        self.lock().take()
    }

    fn lock(&self) -> MutexGuard<'_, Option<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

static SLOT: Slot = Slot::new();

pub(crate) fn offer(moment: Moment, subject: &str) {
    offer_into(&SLOT, answer::interactive(), moment, subject);
}

fn offer_into(slot: &Slot, interactive: bool, moment: Moment, subject: &str) {
    if interactive {
        slot.set(command(moment, subject));
    }
}

pub(crate) fn take() -> Option<String> {
    SLOT.take()
}

#[cfg(test)]
mod tests;
