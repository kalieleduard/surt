use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use super::slots::Slot;

pub(super) struct Flights<F> {
    flying: Vec<(Option<Slot>, Pin<Box<F>>)>,
}

impl<F: Future> Flights<F> {
    pub(super) fn new() -> Flights<F> {
        Flights { flying: Vec::new() }
    }

    pub(super) fn held(&self) -> Vec<usize> {
        self.flying
            .iter()
            .filter_map(|(slot, _)| slot.as_ref().map(Slot::index))
            .collect()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.flying.is_empty()
    }

    pub(super) fn push(&mut self, slot: Option<Slot>, flight: F) {
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
