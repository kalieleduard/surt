use std::path::PathBuf;

use crate::workflow::{AgentContract, Profile};

#[derive(Clone, Debug)]
pub struct AgentLaunch {
    pub(crate) profile: Profile,
    pub(crate) outcomes: Vec<String>,
    pub(crate) outputs: Vec<String>,
    pub(crate) instructions: String,
    pub(crate) context: String,
    pub(crate) working_dir: PathBuf,
    pub(crate) launch_dir: PathBuf,
}

impl AgentLaunch {
    #[cfg_attr(
        not(all(test, feature = "test-support")),
        expect(dead_code, reason = "drive launches every agent from ticket 08 on")
    )]
    pub(crate) fn new(
        agent: &AgentContract,
        instructions: &str,
        context: &str,
        working_dir: PathBuf,
        launch_dir: PathBuf,
    ) -> AgentLaunch {
        AgentLaunch {
            profile: agent.profile.clone(),
            outcomes: agent.outcomes(),
            outputs: agent.outputs.keys().cloned().collect(),
            instructions: instructions.to_owned(),
            context: context.to_owned(),
            working_dir,
            launch_dir,
        }
    }
}
