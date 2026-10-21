use std::io::{IsTerminal, Write};

use surt_engine::project::{ActivitySnapshot, stderr_columns};

use anstyle::{AnsiColor, Style};

use super::activity::{Activity, Output, Terminal, on_tick};
use super::spinner;
use crate::message::{Mark, Message, stderr_colored};
use crate::symbols::{HORIZONTAL, SEPARATOR, Symbols, printable};

const ERASE_LINE: &str = "\r\x1b[2K";
const MILLIS_PER_SECOND: i64 = 1000;
const SPINNING: Style = AnsiColor::Cyan.on_default().bold();
const FADED: Style = Style::new().dimmed();

pub(super) struct Screen {
    activity: Activity,
    tty: bool,
    colored: bool,
    symbols: Symbols,
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
            colored: tty && stderr_colored(),
            symbols: Symbols::of_console(),
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
                let frame = spinner::frame(now_ms, self.symbols);
                let text = match self.colored {
                    true => painted(&text, frame, self.symbols),
                    false => text,
                };
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
                symbols: self.symbols,
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

pub(super) fn painted(footer: &str, frame: &str, symbols: Symbols) -> String {
    let Some(rest) = footer.strip_prefix(frame) else {
        return footer.to_owned();
    };
    let faded = |text: &str| format!("{FADED}{text}{FADED:#}");
    let rule = symbols.render(HORIZONTAL).repeat(2);
    let separator = symbols.render(SEPARATOR).into_owned();
    let rest = rest
        .replace(&rule, &faded(&rule))
        .replace(&separator, &faded(&separator));
    format!("{SPINNING}{frame}{SPINNING:#}{rest}")
}

fn write(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(printable(text).as_bytes());
    let _ = stderr.flush();
}
