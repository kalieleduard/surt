use std::path::Path;

use surt_engine::project::{FileState, InitFile, Initialized, Project};

use crate::failure::Failure;
use crate::message::Message;
use crate::symbols::SUCCESS;
use crate::template::{self, FEATURE_TEMPLATE, TEMPLATES};

const NEXT_COMMAND: &str = "surt feature \"<what you want to build>\"";
const DOT_SURT: &str = ".surt";
const STATE_GAP: usize = 2;

#[derive(clap::Args)]
pub(crate) struct InitArgs {
    #[arg(long, value_name = "name")]
    template: Option<String>,
}

pub(crate) async fn run(cwd: &Path, args: InitArgs) -> Result<(), Failure> {
    let files = match &args.template {
        Some(name) => template::named(name)
            .ok_or_else(|| Failure::Refused(unknown_template(name)))?
            .files(),
        None => Vec::new(),
    };
    let written: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_bytes()))
        .collect();
    let initialized = Project::init(cwd, &written).await?;
    report(&initialized).print_success();
    Ok(())
}

fn unknown_template(name: &str) -> Message {
    Message::new(format!("no template is named \"{name}\""))
        .context(format!("templates: {}", TEMPLATES.join(", ")))
        .way("fix", format!("surt init --template {FEATURE_TEMPLATE}"))
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
    let lines: Vec<(String, FileState)> = initialized
        .files
        .iter()
        .map(|(file, state)| (file_path(file), *state))
        .collect();
    let width = lines.iter().map(|(path, _)| path.len()).max().unwrap_or(0) + STATE_GAP;
    for (path, state) in lines {
        message = message.context(format!("{path:<width$}{}", state_name(state)));
    }
    message
        .context(format!(
            "git {} {SUCCESS}  identity {} {SUCCESS}",
            initialized.git, initialized.identity
        ))
        .way("next", NEXT_COMMAND)
}

fn file_path(file: &InitFile) -> String {
    match file {
        InitFile::Gitignore => format!("{DOT_SURT}/.gitignore"),
        InitFile::Config => format!("{DOT_SURT}/config.toml"),
        InitFile::Database => format!("{DOT_SURT}/surt.db"),
        InitFile::Template(path) => format!("{DOT_SURT}/{path}"),
    }
}

fn state_name(state: FileState) -> &'static str {
    match state {
        FileState::Created => "created",
        FileState::Kept => "kept",
    }
}
