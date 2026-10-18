use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

pub(super) struct Flights<F> {
    slots: usize,
    flying: Vec<(Option<usize>, Pin<Box<F>>)>,
}

impl<F: Future> Flights<F> {
    pub(super) fn new(slots: usize) -> Flights<F> {
        Flights {
            slots,
            flying: Vec::new(),
        }
    }

    pub(super) fn free_slots(&self) -> Vec<usize> {
        (0..self.slots)
            .filter(|slot| self.flying.iter().all(|(taken, _)| *taken != Some(*slot)))
            .collect()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.flying.is_empty()
    }

    pub(super) fn push(&mut self, slot: Option<usize>, flight: F) {
        self.flying.push((slot, Box::pin(flight)));
    }

    pub(super) async fn landed(&mut self) -> Vec<F::Output> {
        poll_fn(|context| {
            let landed = self.poll_all(context);
            if landed.is_empty() {
                Poll::Pending
            } else {
                Poll::Ready(landed)
            }
        })
        .await
    }

    pub(super) async fn alongside<T>(
        &mut self,
        work: impl Future<Output = T>,
    ) -> (T, Vec<F::Output>) {
        let mut work = pin!(work);
        let mut landed = Vec::new();
        let value = poll_fn(|context| {
            landed.extend(self.poll_all(context));
            work.as_mut().poll(context)
        })
        .await;
        (value, landed)
    }

    fn poll_all(&mut self, context: &mut Context<'_>) -> Vec<F::Output> {
        let mut landed = Vec::new();
        self.flying
            .retain_mut(|(_, flight)| match flight.as_mut().poll(context) {
                Poll::Ready(output) => {
                    landed.push(output);
                    false
                }
                Poll::Pending => true,
            });
        landed
    }
}
