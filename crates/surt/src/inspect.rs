mod details;
mod execution;
mod fields;
mod json;

use std::path::Path;

use jiff::{Timestamp, Zoned};
use surt_engine::project::{InspectRequest, Project, Reference, Timeline, TimelineEvent};

use crate::failure::Failure;
use crate::status::run_selector;
use crate::symbols::{SEPARATOR, printable};
use crate::table;

const SAME_DAY: &str = "%H:%M:%S";
const EARLIER_DAY: &str = "%Y-%m-%d %H:%M:%S";

#[derive(clap::Args)]
pub(crate) struct InspectArgs {
    feature: String,
    task: Option<String>,
    #[arg(long, value_name = "N")]
    run: Option<u32>,
    #[arg(long)]
    json: bool,
}

pub(crate) async fn run(cwd: &Path, args: InspectArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    if let (None, None) = (&args.task, args.run)
        && let Reference::Execution(id) = project.reference(&args.feature).await?
    {
        let detail = project.inspect_execution(&id).await?;
        if args.json {
            return crate::json::print(json::execution(&detail));
        }
        let now = Timestamp::now().as_second();
        anstream::print!("{}", printable(&execution::render(&detail, now)));
        return Ok(());
    }
    let request = InspectRequest {
        item: args.task.as_deref(),
        run: run_selector(args.run),
    };
    let timeline = project.inspect(&args.feature, request).await?;
    if args.json {
        return crate::json::print(json::timeline(&timeline));
    }
    anstream::print!("{}", printable(&render(&timeline, &Zoned::now())));
    Ok(())
}

pub(crate) fn render(timeline: &Timeline, now: &Zoned) -> String {
    let rows: Vec<[String; 3]> = timeline
        .events
        .iter()
        .map(|event| {
            [
                stamp(event.at, now),
                event.kind.clone(),
                describe(timeline, event),
            ]
        })
        .collect();
    table::render(&rows)
}

fn stamp(at: i64, now: &Zoned) -> String {
    let Ok(timestamp) = Timestamp::from_second(at) else {
        return at.to_string();
    };
    let zoned = timestamp.to_zoned(now.time_zone().clone());
    let format = if zoned.date() == now.date() {
        SAME_DAY
    } else {
        EARLIER_DAY
    };
    zoned.strftime(format).to_string()
}

fn describe(timeline: &Timeline, event: &TimelineEvent) -> String {
    let mut parts = describe_scope(timeline, event);
    parts.extend(details::of(event));
    parts.join(SEPARATOR)
}

fn describe_scope(timeline: &Timeline, event: &TimelineEvent) -> Vec<String> {
    let item = match &timeline.item {
        None => event.item.as_deref(),
        Some(_) => None,
    };
    scope(item, event.step.as_deref(), event.iteration, event.attempt)
}

pub(crate) fn scope(
    item: Option<&str>,
    step: Option<&str>,
    iteration: Option<u32>,
    attempt: Option<u32>,
) -> Vec<String> {
    let mut parts = Vec::new();
    parts.extend(item.map(|item| format!("task {item}")));
    parts.extend(step.map(|step| match iteration {
        Some(iteration) => format!("{step} iter {iteration}"),
        None => step.to_owned(),
    }));
    parts.extend(attempt.map(|attempt| format!("attempt {attempt}")));
    parts
}

#[cfg(test)]
mod tests;
