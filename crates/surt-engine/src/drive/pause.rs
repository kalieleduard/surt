const BACKOFF_MINUTES: [i64; 5] = [1, 2, 4, 8, 15];
const SECONDS_PER_MINUTE: i64 = 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PauseSchedule {
    until: Option<i64>,
    backoff_step: Option<u32>,
    steps_taken: u32,
}

impl PauseSchedule {
    pub(crate) fn until(&self, now: i64) -> Option<i64> {
        self.until.filter(|until| now < *until)
    }

    pub(crate) fn failed(
        self,
        launched_at: i64,
        resets_at: Option<i64>,
        now: i64,
    ) -> PauseSchedule {
        let paused = self.until(now);
        match (resets_at.filter(|reset| now < *reset), paused) {
            (Some(reset), Some(until)) if reset <= until => self,
            (Some(reset), _) => PauseSchedule {
                until: Some(reset),
                backoff_step: None,
                ..self
            },
            (None, _) if !self.resumed(launched_at) => self,
            (None, _) => {
                let step = self.steps_taken + 1;
                PauseSchedule {
                    until: Some(now + backoff(step)),
                    backoff_step: Some(step),
                    steps_taken: step,
                }
            }
        }
    }

    fn resumed(&self, launched_at: i64) -> bool {
        self.until.is_none_or(|until| until <= launched_at)
    }

    pub(crate) fn ended(self) -> PauseSchedule {
        PauseSchedule {
            steps_taken: 0,
            ..self
        }
    }

    pub(crate) fn backoff_step(&self) -> Option<u32> {
        self.backoff_step
    }
}

pub(crate) fn backoff(step: u32) -> i64 {
    let last = BACKOFF_MINUTES.len() - 1;
    let index = usize::try_from(step.saturating_sub(1)).map_or(last, |index| index.min(last));
    BACKOFF_MINUTES[index] * SECONDS_PER_MINUTE
}

#[cfg(test)]
mod tests;
