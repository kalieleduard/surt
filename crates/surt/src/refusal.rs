mod definition;
mod edits;
mod reading;

use std::path::Path;

use surt_engine::project::{Candidate, DefinitionFile, InvalidDefinition, Refusal, RunState};

use crate::feature::quoted;
use crate::message::Message;
use crate::status::{STATE_SEPARATOR, state_name};

const GIT_DOWNLOADS: &str = "https://git-scm.com/downloads";
const SHORT_SHA_CHARS: usize = 7;

pub(crate) fn message(refusal: &Refusal) -> Message {
    match refusal {
        Refusal::NotAProject { root } => Message::new(format!(
            "not a surt project: no .surt/ in {}",
            root.display()
        ))
        .way("run", "surt init"),
        Refusal::InsideManagedWorktree { path, project } => {
            managed_worktree(path, project.as_deref())
        }
        Refusal::NotAGitRepository { path } => {
            Message::new(format!("{} is not inside a git repository", path.display()))
                .way("fix", "git init")
        }
        Refusal::BareRepository { path } => Message::new(format!(
            "{} is a bare repository; surt needs a worktree",
            path.display()
        ))
        .way("fix", format!("git clone {} <directory>", path.display())),
        Refusal::GitNotFound => Message::new("git is not on PATH; surt needs git 2.38 or later")
            .way("fix", format!("install git from {GIT_DOWNLOADS}")),
        Refusal::GitTooOld { found, min } => Message::new(format!(
            "git {found} is too old; surt needs git {}.{} or later",
            min.major, min.minor
        ))
        .way("fix", format!("upgrade git from {GIT_DOWNLOADS}")),
        Refusal::NoGitIdentity => {
            Message::new("git has no user identity; surt's merge commits need one")
                .way("fix", "git config --global user.name \"Your Name\"")
                .way("fix", "git config --global user.email \"you@example.com\"")
        }
        Refusal::SchemaTooNew { db, supported } => Message::new(format!(
            ".surt/surt.db is at schema {db}; this surt supports up to {supported}"
        ))
        .way("fix", "upgrade surt"),
        Refusal::InvalidConfig { path, line, reason } => invalid_config(path, *line, reason),
        Refusal::MergeIntoBaseUnsupported { path } => {
            Message::new("merge_into_base = \"ff-only\" is not supported yet")
                .context("this surt never merges into the base branch")
                .way(
                    "fix",
                    format!("set merge_into_base = \"never\" in {}", path.display()),
                )
        }
        Refusal::ActiveRunExists { subject } => {
            Message::new(format!("{subject} already has an active run"))
                .way("resume", format!("surt feature {subject}"))
        }
        Refusal::UnrecordedEditsBlockNewRun {
            subject,
            run,
            files,
        } => edits::blocking_new_run(subject, *run, files),
        Refusal::InvalidDefinition(invalid) => invalid_definition(invalid),
        Refusal::AgentOverrides { path } => Message::new(format!(
            "local agent overrides in {} are not supported yet",
            path.display()
        ))
        .context("this surt runs only its built-in agents")
        .way("fix", format!("move {} out of the project", path.display())),
        Refusal::UnknownSubject { reference } => {
            Message::new(format!("no feature matches {}", quoted(reference)))
                .way("inspect", "surt status")
        }
        Refusal::AmbiguousSubject {
            reference,
            candidates,
        } => ambiguous(reference, candidates),
        Refusal::DriverLive { subject, pid, .. } => driver_live(subject, *pid),
        Refusal::NoGateWaiting { subject } => {
            Message::new(format!("{subject} has no approval gate waiting"))
                .way("run", format!("surt status {subject}"))
        }
        Refusal::StaleRevision {
            subject,
            artifact,
            expected,
            latest,
        } => Message::new(format!(
            "{artifact} rev {latest} is the latest, not rev {expected}"
        ))
        .context(format!("review rev {latest} before deciding"))
        .way(
            "run",
            format!("surt show {subject} {artifact} --rev {latest}"),
        ),
        Refusal::UnapprovedRevision {
            subject,
            artifact,
            revision,
        } => edits::unapproved(subject, artifact, *revision),
        Refusal::PinnedEdit {
            subject,
            artifact,
            revision,
            files,
        } => edits::pinned(subject, artifact, *revision, files),
        Refusal::InvalidEdit {
            subject,
            artifact,
            problem,
            ..
        } => edits::invalid(subject, artifact, problem),
        Refusal::NotRestorable {
            subject,
            artifact,
            restorable,
        } => edits::not_restorable(subject, artifact, restorable),
        Refusal::TerminalRun {
            subject,
            run,
            files,
        } => edits::terminal_run(subject, *run, files),
        Refusal::IntegrationBranchMoved {
            subject,
            branch,
            expected,
            found,
        } => Message::new(format!(
            "{branch} moved outside surt (expected {}, found {})",
            short(expected),
            found.as_deref().map_or("no branch", short)
        ))
        .way("fix", format!("git branch -f {branch} {expected}"))
        .way("resume", format!("surt start {subject}")),
        Refusal::IntegrationBranchCheckedOut {
            subject,
            branch,
            path,
        } => Message::new(format!("{branch} is checked out in {}", path.display()))
            .way("fix", format!("git -C {} switch --detach", path.display()))
            .way("resume", format!("surt start {subject}")),
        Refusal::UnknownArtifact {
            subject,
            artifact,
            known,
        } => reading::unknown_artifact(subject, artifact, known),
        Refusal::UnknownRevision {
            subject,
            artifact,
            revision,
            latest,
        } => reading::unknown_revision(subject, artifact, *revision, *latest),
        Refusal::UnknownItem {
            subject,
            item,
            known,
        } => reading::unknown_task(subject, item, known),
        Refusal::UnknownRun { subject, run, runs } => reading::unknown_run(subject, *run, runs),
        Refusal::NoRevisionInIteration {
            subject,
            item,
            artifact,
            iteration,
            iterations,
        } => reading::no_revision_in_iteration(
            subject,
            item.as_deref(),
            artifact,
            *iteration,
            iterations,
        ),
        Refusal::UnknownFile {
            artifact,
            revision,
            path,
            files,
            ..
        } => reading::unknown_file(artifact, *revision, path, files),
    }
}

fn short(sha: &str) -> &str {
    sha.get(..SHORT_SHA_CHARS).unwrap_or(sha)
}

fn ambiguous(reference: &str, candidates: &[Candidate]) -> Message {
    let headline = format!(
        "{} matches {} features:",
        quoted(reference),
        candidates.len()
    );
    let width = candidates
        .iter()
        .map(|candidate| candidate.name.chars().count())
        .max()
        .unwrap_or_default();
    candidates
        .iter()
        .fold(Message::new(headline), |message, candidate| {
            let line = format!("  {:<width$}  {}", candidate.name, derived_state(candidate));
            message.context(line.trim_end())
        })
        .context("use the full name")
}

fn derived_state(candidate: &Candidate) -> String {
    candidate
        .states
        .iter()
        .map(|state| {
            let name = state_name(*state);
            match (state, &candidate.awaiting) {
                (RunState::AwaitingApproval, Some(awaited)) => {
                    format!("{name} ({} rev {})", awaited.artifact, awaited.revision)
                }
                _ => name.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(STATE_SEPARATOR)
}

fn driver_live(subject: &str, pid: Option<u32>) -> Message {
    let driver = match pid {
        Some(pid) => format!("pid {pid}"),
        None => "another surt process".to_owned(),
    };
    Message::new(format!("{subject} is being driven by {driver}"))
        .way("fix", "stop it with Ctrl-C in its terminal, then rerun")
}

fn invalid_definition(invalid: &InvalidDefinition) -> Message {
    let file = match &invalid.file {
        DefinitionFile::Workflow(name) => format!("workflow {name}"),
        DefinitionFile::Agent(name) => format!("agent {name}"),
    };
    let location = match &invalid.step {
        Some(step) => format!("{file} [steps.{step}]"),
        None => file,
    };
    Message::new(format!("invalid built-in definition: {location}"))
        .context(definition::describe(&invalid.problem))
        .way("fix", "reinstall surt")
}

fn managed_worktree(path: &Path, project: Option<&Path>) -> Message {
    let message = Message::new(format!(
        "{} is inside a surt-managed worktree",
        path.display()
    ))
    .context("surt commands run from the project, not from a task's worktree");
    match project {
        Some(project) => message.way("fix", format!("cd {}", project.display())),
        None => message.way("fix", "cd <your repository>"),
    }
}

fn invalid_config(path: &Path, line: Option<usize>, reason: &str) -> Message {
    let location = match line {
        Some(line) => format!("{}:{line}", path.display()),
        None => path.display().to_string(),
    };
    Message::new(format!("invalid configuration in {location}"))
        .context(reason)
        .way("fix", format!("edit {location}"))
}

#[cfg(test)]
mod tests;
