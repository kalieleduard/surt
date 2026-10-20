use serde_json::json;
use sqlx::SqliteConnection;

use super::columns::number;
use super::gates::WaitingGate;
use super::revisions::{Origin, artifact_id, publish};
use super::{About, Store, StoreError, record_event_data, unix_now};
use crate::artifacts::{Round, accumulate};

const ANSWER: &str = "answer";
const ANSWERED: &str = "answered";
const RECORDED: &str = "answer.recorded";
const ANSWERS_EXTENSION: &str = "json";

pub(crate) struct NewAnswer<'a> {
    pub(crate) step: &'a WaitingGate,
    pub(crate) artifact: &'a str,
    pub(crate) revision: u32,
    pub(crate) answer_artifact: &'a str,
    pub(crate) round: &'a Round<'a>,
    pub(crate) actor: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnswerOutcome {
    Recorded { revision: u32, round: u32 },
    NotWaiting,
    Stale { expected: u32, latest: u32 },
}

impl Store {
    pub(crate) async fn record_answer(
        &self,
        new: NewAnswer<'_>,
    ) -> Result<AnswerOutcome, StoreError> {
        let now = unix_now();
        let step = new.step.id.as_str();
        let mut transaction = self.begin_write().await?;
        match latest_number(&mut transaction, &new.step.run, new.artifact).await? {
            None => return Ok(AnswerOutcome::NotWaiting),
            Some(latest) if latest != new.revision => {
                return Ok(AnswerOutcome::Stale {
                    expected: new.revision,
                    latest,
                });
            }
            Some(_) => {}
        }
        let closed = sqlx::query(
            "UPDATE step_run SET state = 'done', outcome = ?, finished_at = ? WHERE id = ? AND state = 'running'",
        )
        .bind(ANSWERED)
        .bind(now)
        .bind(step)
        .execute(&mut *transaction)
        .await?;
        if closed.rows_affected() == 0 {
            return Ok(AnswerOutcome::NotWaiting);
        }
        let artifact = artifact_id(&mut transaction, &new.step.run, new.answer_artifact).await?;
        let file = format!("{}.{ANSWERS_EXTENSION}", new.answer_artifact);
        let previous = previous_answers(&mut transaction, &artifact, &file).await?;
        let accumulated =
            accumulate(previous.as_deref(), new.round).map_err(StoreError::CorruptAnswers)?;
        let origin = Origin::Human {
            action: ANSWER,
            actor: new.actor,
        };
        let files = [(file, accumulated.bytes)];
        let published = publish(&mut transaction, &artifact, &files, origin, now).await?;
        let data = json!({
            "artifact": new.artifact,
            "revision": new.revision,
            "answer_artifact": new.answer_artifact,
            "answer_revision": published.number,
            "round": accumulated.round,
            "actor": new.actor,
        });
        record_event_data(&mut transaction, About::Step(step), RECORDED, now, &data).await?;
        let outcome = json!({ "outcome": ANSWERED });
        record_event_data(
            &mut transaction,
            About::Step(step),
            "step.done",
            now,
            &outcome,
        )
        .await?;
        transaction.commit().await?;
        Ok(AnswerOutcome::Recorded {
            revision: published.number,
            round: accumulated.round,
        })
    }
}

async fn latest_number(
    connection: &mut SqliteConnection,
    run: &str,
    artifact: &str,
) -> Result<Option<u32>, sqlx::Error> {
    let latest: Option<i64> = sqlx::query_scalar(
        "SELECT max(r.number) FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
         WHERE a.workflow_run_id = ? AND a.name = ?",
    )
    .bind(run)
    .bind(artifact)
    .fetch_one(connection)
    .await?;
    Ok(latest.map(number))
}

async fn previous_answers(
    connection: &mut SqliteConnection,
    artifact: &str,
    file: &str,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT b.bytes FROM artifact_revision r JOIN revision_file f ON f.revision_id = r.id \
         JOIN blob b ON b.sha256 = f.blob_sha256 \
         WHERE r.artifact_id = ? AND f.rel_path = ? ORDER BY r.number DESC LIMIT 1",
    )
    .bind(artifact)
    .bind(file)
    .fetch_optional(connection)
    .await
}
