use std::path::Path;

use surt_engine::project::{FileState, InitFile, Initialized, Project};

use crate::failure::Failure;
use crate::message::Message;

const NEXT_COMMAND: &str = "surt feature \"<what you want to build>\"";

pub(crate) async fn run(cwd: &Path) -> Result<(), Failure> {
    let initialized = Project::init(cwd).await?;
    report(&initialized).print_success();
    Ok(())
}

fn report(initialized: &Initialized) -> Message {
    let dot_surt = initialized.dot_surt.display();
    let headline = if initialized.was_already_initialized() {
        format!("surt is already initialized in {dot_surt}")
    } else {
        format!("initialized surt in {dot_surt}")
    };
    let mut message = Message::new(headline);
    if initialized.from_subdirectory {
        message = message.context(".surt/ lives at the toplevel of the repository's main worktree");
    }
    for (file, state) in initialized.files {
        message = message.context(file_line(file, state));
    }
    message
        .context(format!(
            "git {} ✔  identity {} ✔",
            initialized.git, initialized.identity
        ))
        .way("next", NEXT_COMMAND)
}

fn file_line(file: InitFile, state: FileState) -> String {
    let (name, created) = match file {
        InitFile::Gitignore => (
            ".surt/.gitignore",
            "(only config.toml and agents/ are versioned)",
        ),
        InitFile::Config => (".surt/config.toml", "(defaults, commented)"),
        InitFile::Database => (".surt/surt.db", ""),
    };
    let note = match state {
        FileState::Created => created,
        FileState::Kept => "(exists, kept)",
    };
    format!("{name:<20}{note}").trim_end().to_owned()
}
