pub fn spec(iteration: u32, text: &str) -> String {
    format!(
        "[[on]]\nstep = \"spec\"\niteration = {iteration}\n\
         deliver = {{ outcome = \"done\", outputs = {{ spec = \"spec.md\" }}, files = {{ \"spec.md\" = {text:?} }} }}\n\n"
    )
}

pub fn tasks(iteration: u32, keys: &[&str]) -> String {
    let items: Vec<String> = keys.iter().map(|key| item(key)).collect();
    let manifest = format!("[{}]", items.join(", "));
    let task_files: String = keys
        .iter()
        .map(|key| format!(", \"tasks/{key}.md\" = \"# Task {key}\\n\""))
        .collect();
    format!(
        "[[on]]\nstep = \"plan-tasks\"\niteration = {iteration}\n\
         deliver = {{ outcome = \"done\", outputs = {{ manifest = \"manifest.json\" }}, files = {{ \"manifest.json\" = {manifest:?}{task_files} }} }}\n\n"
    )
}

pub fn crash(step: &str, attempt: u32) -> String {
    format!("[[on]]\nstep = \"{step}\"\nattempt = {attempt}\ncrash = {{ exit_code = 1 }}\n\n")
}

fn item(key: &str) -> String {
    serde_json::json!({
        "key": key,
        "needs": [],
        "title": format!("Task {key}"),
        "description": format!("Do {key}."),
        "acceptance_criteria": [format!("{key} works")],
        "files": { "task": format!("tasks/{key}.md") },
    })
    .to_string()
}
