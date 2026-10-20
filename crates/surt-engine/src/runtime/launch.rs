use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::workflow::{AgentContract, Profile};

#[derive(Clone, Debug)]
pub struct AgentLaunch {
    pub(crate) profile: Profile,
    pub(crate) outcomes: Vec<String>,
    pub(crate) outputs: BTreeMap<String, Vec<String>>,
    pub(crate) instructions: String,
    pub(crate) working_dir: PathBuf,
    pub(crate) launch_dir: PathBuf,
}

impl AgentLaunch {
    pub(crate) fn new(
        agent: &AgentContract,
        instructions: &str,
        working_dir: PathBuf,
        launch_dir: PathBuf,
    ) -> AgentLaunch {
        AgentLaunch {
            profile: agent.profile.clone(),
            outcomes: agent.outcomes(),
            outputs: agent.delivering(),
            instructions: instructions.to_owned(),
            working_dir,
            launch_dir,
        }
    }
}
