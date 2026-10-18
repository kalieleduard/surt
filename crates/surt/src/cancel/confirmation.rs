use surt_engine::project::CancelImpact;

use crate::symbols::SEPARATOR;

pub(super) fn question(subject: &str, impact: &CancelImpact) -> String {
    let mut stopped = vec![running(impact.executions)];
    if let Some(branch) = impact
        .integration_branch
        .as_ref()
        .filter(|_| impact.items > 0)
    {
        stopped.push(format!(
            "{}/{} tasks merged stay on {branch}",
            impact.merged, impact.items
        ));
    }
    format!(
        "cancel {subject}?\n  {}\n  worktrees are removed; task branches and salvage refs stay\n  \
         starting over later needs a new run (surt feature {subject} --new-run)\n",
        stopped.join(SEPARATOR)
    )
}

fn running(executions: usize) -> String {
    match executions {
        0 => "nothing is running".to_owned(),
        1 => "1 running execution will be stopped".to_owned(),
        many => format!("{many} running executions will be stopped"),
    }
}
