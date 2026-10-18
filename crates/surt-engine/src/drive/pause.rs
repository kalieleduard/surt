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

    pub(crate) fn failed(self, launched_at: i64, resets_at: Option<i64>, now: i64) -> Pause {
        let paused = self.until(now);
        match (resets_at.filter(|reset| now < *reset), paused) {
            (Some(reset), Some(until)) if reset <= until => self,
            (Some(reset), _) => Pause {
                until: Some(reset),
                backoff_step: None,
                ..self
            },
            (None, _) if !self.resumed(launched_at) => self,
            (None, _) => {
                let step = self.steps_taken + 1;
                Pause {
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

pub(crate) fn backoff(step: u32) -> i64 {
    const MINUTES: [i64; 5] = [1, 2, 4, 8, 15];
    let index = usize::try_from(step.saturating_sub(1)).unwrap_or(usize::MAX);
    MINUTES.get(index).copied().unwrap_or(15) * 60
}

#[cfg(test)]
mod tests;
