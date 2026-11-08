use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde::Serialize;
use surt_engine::project::{Script, ScriptProcess};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::oneshot;

use crate::workspace::{Failure, Workspace};

const OUTPUT_EVENT: &str = "surt://script-output";
const ENDED_EVENT: &str = "surt://script-ended";
const STOP_GRACE: Duration = Duration::from_secs(4);

#[derive(Clone, Default)]
pub struct Scripts {
    running: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptView {
    name: String,
    description: Option<String>,
    command: String,
    running: bool,
}

#[derive(Clone, Serialize)]
struct OutputLine {
    script: String,
    stream: &'static str,
    line: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ended {
    script: String,
    code: Option<i32>,
    stopped: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn project_scripts(
    workspace: State<'_, Workspace>,
    scripts: State<'_, Scripts>,
) -> Result<Vec<ScriptView>, Failure> {
    let project = workspace.project().await?;
    Ok(project
        .config()
        .scripts
        .iter()
        .map(|script| scripts.view(script))
        .collect())
}

#[tauri::command]
pub async fn start_script(
    name: String,
    app: AppHandle,
    workspace: State<'_, Workspace>,
    scripts: State<'_, Scripts>,
) -> Result<(), Failure> {
    let project = workspace.project().await?;
    let script = project
        .script(&name)
        .cloned()
        .ok_or_else(|| Failure::UnknownScript(name.clone()))?;
    let stop = scripts.claim(&name)?;
    let mut process = match project.spawn_script(&script) {
        Ok(process) => process,
        Err(error) => {
            scripts.release(&name);
            return Err(Failure::Io(script.run[0].clone(), error.to_string()));
        }
    };
    let (stdout, stderr) = process.take_output();
    forward(&app, &name, "stdout", stdout);
    forward(&app, &name, "stderr", stderr);
    let registry = scripts.inner().clone();
    tokio::spawn(async move {
        let ended = supervise(&name, process, stop).await;
        registry.release(&name);
        if let Err(error) = app.emit(ENDED_EVENT, ended) {
            eprintln!("cannot announce that {name} ended: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
pub fn stop_script(name: String, scripts: State<'_, Scripts>) {
    if let Some(stop) = scripts.lock().remove(&name) {
        let _ = stop.send(());
    }
}

impl Scripts {
    fn view(&self, script: &Script) -> ScriptView {
        ScriptView {
            name: script.name.clone(),
            description: script.description.clone(),
            command: script.run.join(" "),
            running: self.lock().contains_key(&script.name),
        }
    }

    fn claim(&self, name: &str) -> Result<oneshot::Receiver<()>, Failure> {
        let mut running = self.lock();
        if running.contains_key(name) {
            return Err(Failure::ScriptRunning(name.to_owned()));
        }
        let (stop, stopped) = oneshot::channel();
        running.insert(name.to_owned(), stop);
        Ok(stopped)
    }

    fn release(&self, name: &str) {
        self.lock().remove(name);
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, oneshot::Sender<()>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

async fn supervise(name: &str, mut process: ScriptProcess, stop: oneshot::Receiver<()>) -> Ended {
    let ended = |code: Option<i32>, stopped: bool, error: Option<String>| Ended {
        script: name.to_owned(),
        code,
        stopped,
        error,
    };
    tokio::select! {
        status = process.wait() => match status {
            Ok(code) => ended(Some(code), false, None),
            Err(error) => ended(None, false, Some(error.to_string())),
        },
        _ = stop => {
            process.interrupt();
            let code = match tokio::time::timeout(STOP_GRACE, process.wait()).await {
                Ok(status) => status.ok(),
                Err(_) => {
                    let _ = process.kill();
                    process.wait().await.ok()
                }
            };
            ended(code, true, None)
        }
    }
}

fn forward<R: AsyncRead + Unpin + Send + 'static>(
    app: &AppHandle,
    script: &str,
    stream: &'static str,
    output: Option<R>,
) {
    let Some(output) = output else {
        return;
    };
    let app = app.clone();
    let script = script.to_owned();
    tokio::spawn(async move {
        let mut reader = BufReader::new(output);
        let mut bytes = Vec::new();
        while matches!(reader.read_until(b'\n', &mut bytes).await, Ok(read) if read > 0) {
            let line = String::from_utf8_lossy(&bytes)
                .trim_end_matches(['\r', '\n'])
                .to_owned();
            bytes.clear();
            let event = OutputLine {
                script: script.clone(),
                stream,
                line,
            };
            if app.emit(OUTPUT_EVENT, event).is_err() {
                return;
            }
        }
    });
}
