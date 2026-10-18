use std::io::{IsTerminal, Write};

use surt_engine::project::ActivitySnapshot;

use super::activity::{Activity, Output, Terminal, on_tick};
use crate::message::{Mark, Message};
use crate::symbols::printable;

const ERASE_LINE: &str = "\r\x1b[2K";

pub(super) struct Screen {
    activity: Activity,
    terminal: Terminal,
    footer: bool,
}

pub(super) fn now() -> i64 {
    jiff::Timestamp::now().as_second()
}

impl Screen {
    pub(super) fn new(started: i64) -> Screen {
        let terminal = if std::io::stderr().is_terminal() {
            Terminal::Tty
        } else {
            Terminal::Plain
        };
        Screen {
            activity: Activity {
                snapshot: None,
                started,
                last_line: started,
            },
            terminal,
            footer: false,
        }
    }

    pub(super) fn snapshot(&mut self, snapshot: ActivitySnapshot, now: i64) {
        self.activity.snapshot = Some(snapshot);
        self.tick(now);
    }

    pub(super) fn line(&mut self, mark: Mark, message: &Message, now: i64) {
        self.erase();
        message.print(mark);
        self.activity.last_line = now;
        self.tick(now);
    }

    pub(super) fn tick(&mut self, now: i64) {
        match on_tick(&self.activity, now, self.terminal) {
            Some(Output::Footer(text)) => {
                write(&format!("{ERASE_LINE}{text}"));
                self.footer = true;
            }
            Some(Output::Line(text)) => {
                write(&format!("{text}\n"));
                self.activity.last_line = now;
            }
            None => {}
        }
    }

    pub(super) fn finish(&mut self) {
        self.erase();
    }

    fn erase(&mut self) {
        if self.footer {
            write(ERASE_LINE);
            self.footer = false;
        }
    }
}

fn write(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(printable(text).as_bytes());
    let _ = stderr.flush();
}
