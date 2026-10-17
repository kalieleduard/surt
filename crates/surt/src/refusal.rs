use std::path::Path;

use surt_engine::project::Refusal;

use crate::message::Message;

const GIT_DOWNLOADS: &str = "https://git-scm.com/downloads";

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
    }
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
