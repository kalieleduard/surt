use serde_json::{Value, json};

pub const HEAD: &str = "$HEAD";

pub fn spec(iteration: u32, text: &str) -> String {
    format!(
        "[[on]]\nstep = \"spec\"\niteration = {iteration}\n\
         deliver = {{ outcome = \"done\", outputs = {{ spec = \"spec.md\" }}, files = {{ \"spec.md\" = {text:?} }} }}\n\n"
    )
}

pub fn questions(iteration: u32, asked: &[Value]) -> String {
    let document = json!({ "questions": asked }).to_string();
    format!(
        "[[on]]\nstep = \"spec\"\niteration = {iteration}\n\
         deliver = {{ outcome = \"questions\", outputs = {{ spec-questions = \"questions.json\" }}, files = {{ \"questions.json\" = {document:?} }} }}\n\n"
    )
}

pub fn question(id: &str, recommended: &str) -> Value {
    json!({
        "id": id,
        "question": format!("Which {id}?"),
        "why": format!("{id} changes the acceptance criteria"),
        "recommended": recommended,
    })
}

pub fn spec_restarted(iteration: u32, text: &str) -> String {
    format!(
        "[[on]]\nstep = \"spec\"\niteration = {iteration}\n\
         transcript_missing = {{ outcome = \"done\", outputs = {{ spec = \"spec.md\" }}, files = {{ \"spec.md\" = {text:?} }} }}\n\n"
    )
}

pub fn tasks(iteration: u32, keys: &[&str]) -> String {
    let independent: Vec<(&str, &[&str])> = keys.iter().map(|key| (*key, &[][..])).collect();
    tasks_needing(iteration, &independent)
}

pub fn tasks_needing(iteration: u32, needing: &[(&str, &[&str])]) -> String {
    let keys: Vec<&str> = needing.iter().map(|(key, _)| *key).collect();
    let items: Vec<String> = needing
        .iter()
        .map(|(key, needs)| item(key, needs))
        .collect();
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

pub fn crash_in(step: &str, iteration: u32, attempt: u32) -> String {
    format!(
        "[[on]]\nstep = \"{step}\"\niteration = {iteration}\nattempt = {attempt}\ncrash = {{ exit_code = 1 }}\n\n"
    )
}

pub fn build(item: &str, iteration: u32, attempt: u32, path: &str, contents: &str) -> String {
    let commit = json!({ "sha": HEAD }).to_string();
    format!(
        "[[on]]\nstep = \"{item}/build\"\niteration = {iteration}\nattempt = {attempt}\n\
         deliver = {{ outcome = \"done\", outputs = {{ commit = \"commit.json\" }}, files = {{ \"commit.json\" = {commit:?} }}, \
         commit = {{ message = \"Build {item}\", files = {{ {path:?} = {contents:?} }} }} }}\n\n"
    )
}

pub fn review(item: &str, iteration: u32, outcome: &str) -> String {
    let findings: Vec<Value> = match outcome {
        "approved" => Vec::new(),
        _ => vec![json!({ "blocking": true, "path": "README.md", "message": "fix this" })],
    };
    let feedback = json!({
        "reviewed_commit": HEAD,
        "summary": format!("{outcome} after reading the code"),
        "findings": findings,
    })
    .to_string();
    format!(
        "[[on]]\nstep = \"{item}/review\"\niteration = {iteration}\n\
         deliver = {{ outcome = \"{outcome}\", outputs = {{ feedback = \"feedback.json\" }}, files = {{ \"feedback.json\" = {feedback:?} }} }}\n\n"
    )
}

pub fn ship() -> String {
    let delivery = json!({ "title": "Add login", "summary": "Adds the login." }).to_string();
    format!(
        "[[on]]\nstep = \"ship\"\n\
         deliver = {{ outcome = \"done\", outputs = {{ delivery = \"delivery.json\" }}, files = {{ \"delivery.json\" = {delivery:?} }} }}\n\n"
    )
}

pub fn wait_until_stopped(step: &str) -> String {
    format!("[[on]]\nstep = \"{step}\"\nwait_until_stopped = {{}}\n\n")
}

pub fn limit_failure_in(step: &str, seconds: i64) -> String {
    format!("[[on]]\nstep = \"{step}\"\nlimit_failure = {{ resets_in = {seconds} }}\n\n")
}

fn item(key: &str, needs: &[&str]) -> String {
    json!({
        "key": key,
        "needs": needs,
        "title": format!("Task {key}"),
        "description": format!("Do {key}."),
        "acceptance_criteria": [format!("{key} works")],
        "files": { "task": format!("tasks/{key}.md") },
    })
    .to_string()
}
