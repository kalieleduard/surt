use std::sync::Arc;

use tokio::sync::watch;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum StopLevel {
    #[default]
    None,
    Graceful,
    Force,
}

#[derive(Clone, Debug)]
pub struct StopSignal(Arc<watch::Sender<StopLevel>>);

impl Default for StopSignal {
    fn default() -> StopSignal {
        StopSignal::new()
    }
}

impl StopSignal {
    pub fn new() -> StopSignal {
        StopSignal(Arc::new(watch::Sender::new(StopLevel::None)))
    }

    pub fn graceful(&self) {
        self.raise(StopLevel::Graceful);
    }

    pub fn force(&self) {
        self.raise(StopLevel::Force);
    }

    pub fn level(&self) -> StopLevel {
        *self.0.borrow()
    }

    pub async fn requested(&self) -> StopLevel {
        let mut receiver = self.0.subscribe();
        receiver
            .wait_for(|level| *level != StopLevel::None)
            .await
            .map_or(StopLevel::Force, |level| *level)
    }

    fn raise(&self, level: StopLevel) {
        self.0.send_if_modified(|current| {
            let raised = level > *current;
            if raised {
                *current = level;
            }
            raised
        });
    }
}
