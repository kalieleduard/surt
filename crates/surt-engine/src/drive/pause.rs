//! The pause on Transient Failure: when agent launches may start again.
//!
//! A pure schedule of `now` (B. Clock): the Driver feeds it every agent
//! Execution that ends and asks it, before each tick, until when launches
//! wait.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pause {
    until: Option<i64>,
    backoff_step: Option<u32>,
    steps_taken: u32,
}

impl Pause {
    pub(crate) fn until(&self, now: i64) -> Option<i64> {
        self.until.filter(|until| now < *until)
    }

    /// An agent Execution failed `transient` at `now`, reporting `resets_at`
    /// when the stream knew it.
    ///
    /// A reset time later than `now` ends the pause then, unless the pause
    /// already ends later (the latest reset wins). Without one, a failure
    /// while launches are paused comes from an agent that was already
    /// running, so it changes nothing; a failure once launches resumed takes
    /// the next backoff step.
    pub(crate) fn failed(self, resets_at: Option<i64>, now: i64) -> Pause {
        let paused = self.until(now);
        match (resets_at.filter(|reset| now < *reset), paused) {
            (Some(reset), Some(until)) if reset <= until => self,
            (Some(reset), _) => Pause {
                until: Some(reset),
                backoff_step: None,
                ..self
            },
            (None, Some(_)) => self,
            (None, None) => {
                let step = self.steps_taken + 1;
                Pause {
                    until: Some(now + backoff(step)),
                    backoff_step: Some(step),
                    steps_taken: step,
                }
            }
        }
    }

    /// An agent Execution ended any other way: the backoff starts over.
    pub(crate) fn ended(self) -> Pause {
        Pause {
            steps_taken: 0,
            ..self
        }
    }

    pub(crate) fn backoff_step(&self) -> Option<u32> {
        self.backoff_step
    }
}

/// Seconds the pause lasts at backoff `step` (1-based): 1, 2, 4, 8, then 15
/// minutes, with no ceiling on the number of steps.
pub(crate) fn backoff(step: u32) -> i64 {
    const MINUTES: [i64; 5] = [1, 2, 4, 8, 15];
    let index = usize::try_from(step.saturating_sub(1)).unwrap_or(usize::MAX);
    MINUTES.get(index).copied().unwrap_or(15) * 60
}

#[cfg(test)]
mod tests;
