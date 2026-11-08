use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Stdio;

use process_wrap::tokio::{ChildWrapper, CommandWrap};
use tokio::process::{ChildStderr, ChildStdout};

use super::{Project, Script};
use crate::platform::{contain, exit_code, find_executable, interrupt};

pub struct ScriptProcess {
    child: Box<dyn ChildWrapper>,
}

impl Project {
    pub fn script(&self, name: &str) -> Option<&Script> {
        self.config()
            .scripts
            .iter()
            .find(|script| script.name == name)
    }

    pub fn spawn_script(&self, script: &Script) -> io::Result<ScriptProcess> {
        spawn(script, self.root())
    }
}

fn spawn(script: &Script, root: &Path) -> io::Result<ScriptProcess> {
    let environment: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    let program = find_executable(Path::new(&script.run[0]), &environment);
    let mut command = CommandWrap::with_new(program, |command| {
        command
            .args(&script.run[1..])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    contain(&mut command);
    Ok(ScriptProcess {
        child: command.spawn()?,
    })
}

impl ScriptProcess {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn take_output(&mut self) -> (Option<ChildStdout>, Option<ChildStderr>) {
        (self.child.stdout().take(), self.child.stderr().take())
    }

    pub async fn wait(&mut self) -> io::Result<i32> {
        Ok(exit_code(self.child.wait().await?))
    }

    pub fn interrupt(&mut self) {
        interrupt(self.child.as_mut());
    }

    pub fn kill(&mut self) -> io::Result<()> {
        self.child.start_kill()
    }
}

#[cfg(test)]
mod tests;
