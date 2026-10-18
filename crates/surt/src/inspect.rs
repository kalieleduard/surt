mod details;
mod fields;

use std::path::Path;

use jiff::{Timestamp, Zoned};
use surt_engine::project::{InspectRequest, Project, Timeline, TimelineEvent};

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
}

pub(crate) async fn run(cwd: &Path, args: InspectArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    let request = InspectRequest {
        item: args.task.as_deref(),
        run: run_selector(args.run),
    };
    let timeline = project.inspect(&args.feature, request).await?;
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
    let mut parts = scope(timeline, event);
    parts.extend(details::of(event));
    parts.join(SEPARATOR)
}

fn scope(timeline: &Timeline, event: &TimelineEvent) -> Vec<String> {
    let mut parts = Vec::new();
    if let (None, Some(item)) = (&timeline.item, &event.item) {
        parts.push(format!("task {item}"));
    }
    if let Some(step) = &event.step {
        parts.push(match event.iteration {
            Some(iteration) => format!("{step} iter {iteration}"),
            None => step.clone(),
        });
    }
    if let Some(attempt) = event.attempt {
        parts.push(format!("attempt {attempt}"));
    }
    parts
}

#[cfg(test)]
mod tests;
