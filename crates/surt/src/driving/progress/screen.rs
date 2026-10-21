use std::io::{IsTerminal, Write};

use surt_engine::project::{ActivitySnapshot, stderr_columns};

use super::activity::{Activity, Output, Terminal, on_tick};
use crate::message::{Mark, Message};
use crate::symbols::printable;

const ERASE_LINE: &str = "\r\x1b[2K";
const MILLIS_PER_SECOND: i64 = 1000;

pub(super) struct Screen {
    activity: Activity,
    tty: bool,
    footer: bool,
}

pub(super) fn now() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

impl Screen {
    pub(super) fn new(started_ms: i64) -> Screen {
        let started = started_ms.div_euclid(MILLIS_PER_SECOND);
        let tty = std::io::stderr().is_terminal();
        Screen {
            activity: Activity {
                snapshot: None,
                started,
                last_line: started,
            },
            tty,
            footer: false,
        }
    }

    pub(super) fn snapshot(&mut self, snapshot: ActivitySnapshot, now_ms: i64) {
        self.activity.snapshot = Some(snapshot);
        self.tick(now_ms);
    }

    pub(super) fn line(&mut self, mark: Mark, message: &Message, now_ms: i64) {
        self.erase();
        message.print(mark);
        self.activity.last_line = now_ms.div_euclid(MILLIS_PER_SECOND);
        self.tick(now_ms);
    }

    pub(super) fn tick(&mut self, now_ms: i64) {
        match on_tick(&self.activity, now_ms, self.terminal()) {
            Some(Output::Footer(text)) => {
                write(&format!("{ERASE_LINE}{text}"));
                self.footer = true;
            }
            Some(Output::Line(text)) => {
                write(&format!("{text}\n"));
                self.activity.last_line = now_ms.div_euclid(MILLIS_PER_SECOND);
            }
            None => {}
        }
    }

    fn terminal(&self) -> Terminal {
        if self.tty {
            Terminal::Tty {
                width: stderr_columns(),
            }
        } else {
            Terminal::Plain
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
