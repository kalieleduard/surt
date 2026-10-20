use std::collections::BTreeMap;

mod waiting;

pub use waiting::WaitingInput;
pub(super) use waiting::{input_step, next_round};

use super::{Error, Project, Refusal, RunId, open_store};
use crate::artifacts::{Round, Unanswered};
use crate::store::{AnswerOutcome, NewAnswer, Store};
use waiting::Waiting;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    pub step: String,
    pub artifact: String,
    pub revision: u32,
    pub answer_artifact: String,
    pub answer_revision: u32,
    pub round: u32,
    pub actor: String,
}

struct Answer<'a> {
    revision: u32,
    answers: &'a BTreeMap<String, String>,
    note: Option<&'a str>,
    actor: &'a str,
}

impl Project {
    pub async fn answer(
        &self,
        run: &RunId,
        revision: u32,
        answers: &BTreeMap<String, String>,
        note: Option<&str>,
        actor: &str,
    ) -> Result<Answered, Error> {
        let store = open_store(&self.layout).await?;
        let answer = Answer {
            revision,
            answers,
            note,
            actor,
        };
        let answered = self.record_answer(&store, run.as_str(), &answer).await;
        store.close().await;
        answered
    }

    async fn record_answer(
        &self,
        store: &Store,
        root: &str,
        answer: &Answer<'_>,
    ) -> Result<Answered, Error> {
        let waiting = self.waiting(store, root).await?;
        let round = checked(store, root, &waiting.input, answer).await?;
        let outcome = store
            .record_answer(NewAnswer {
                step: &waiting.gate,
                artifact: &waiting.input.artifact,
                revision: answer.revision,
                answer_artifact: &waiting.spec.answer_artifact,
                round: &round,
                actor: answer.actor,
            })
            .await?;
        self.answered(store, root, waiting, answer, outcome).await
    }

    async fn answered(
        &self,
        store: &Store,
        root: &str,
        waiting: Waiting,
        answer: &Answer<'_>,
        outcome: AnswerOutcome,
    ) -> Result<Answered, Error> {
        match outcome {
            AnswerOutcome::Recorded { revision, round } => Ok(Answered {
                step: waiting.gate.key,
                artifact: waiting.spec.artifact,
                revision: answer.revision,
                answer_artifact: waiting.spec.answer_artifact,
                answer_revision: revision,
                round,
                actor: answer.actor.to_owned(),
            }),
            AnswerOutcome::Stale { expected, latest } => {
                Err(stale(store, root, &waiting.spec.artifact, expected, latest).await?)
            }
            AnswerOutcome::NotWaiting => Err(self.no_input(store, root).await?),
        }
    }
}

async fn checked<'a>(
    store: &Store,
    root: &str,
    input: &'a WaitingInput,
    answer: &Answer<'a>,
) -> Result<Round<'a>, Error> {
    if input.revision != answer.revision {
        let latest = input.revision;
        return Err(stale(store, root, &input.artifact, answer.revision, latest).await?);
    }
    let round = Round {
        questions: input.questions.questions(),
        answers: answer.answers,
        note: answer.note,
    };
    match round.unanswered() {
        Some(unanswered) => Err(unanswered_questions(store, root, input, unanswered).await?),
        None => Ok(round),
    }
}

async fn stale(
    store: &Store,
    root: &str,
    artifact: &str,
    expected: u32,
    latest: u32,
) -> Result<Error, Error> {
    Ok(Refusal::StaleRevision {
        subject: store.subject_of_run(root).await?,
        artifact: artifact.to_owned(),
        expected,
        latest,
    }
    .into())
}

async fn unanswered_questions(
    store: &Store,
    root: &str,
    input: &WaitingInput,
    unanswered: Unanswered,
) -> Result<Error, Error> {
    Ok(Refusal::UnansweredQuestions {
        subject: store.subject_of_run(root).await?,
        artifact: input.artifact.clone(),
        revision: input.revision,
        missing: unanswered.missing,
        unknown: unanswered.unknown,
    }
    .into())
}
