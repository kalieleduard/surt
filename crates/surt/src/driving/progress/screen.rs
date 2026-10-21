use std::io::{self, IsTerminal, Write};

use crossterm::{cursor, queue, terminal};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use surt_engine::project::{ActivitySnapshot, stderr_columns};

use anstyle::{AnsiColor, Style};

use super::activity::{Activity, Output, Terminal, on_tick};
use super::row::Row;
use crate::message::{Mark, Message, stderr_colored};
use crate::symbols::{Symbols, printable};

const MILLIS_PER_SECOND: i64 = 1000;
const CLOCK: &str = "%H:%M:%S";
const SPINNING: Style = AnsiColor::Cyan.on_default().bold();
const FADED: Style = Style::new().dimmed();

pub(super) struct Screen {
    activity: Activity,
    tty: bool,
    colored: bool,
    symbols: Symbols,
    footer: u16,
}

pub(super) fn now() -> i64 {
    Timestamp::now().as_millisecond()
}

impl Screen {
    pub(super) fn new(started_ms: i64) -> Screen {
        let started = started_ms.div_euclid(MILLIS_PER_SECOND);
        let tty = io::stderr().is_terminal();
        Screen {
            activity: Activity {
                snapshot: None,
                started,
                last_line: started,
            },
            tty,
            colored: tty && stderr_colored(),
            symbols: Symbols::of_console(),
            footer: 0,
        }
    }

    pub(super) fn snapshot(&mut self, snapshot: ActivitySnapshot, now_ms: i64) {
        self.activity.snapshot = Some(snapshot);
        self.tick(now_ms);
    }

    pub(super) fn line(&mut self, mark: Mark, message: &Message, now_ms: i64) {
        self.erase();
        match self.tty {
            true => message.print_stamped(mark, &clock(now_ms)),
            false => message.print(mark),
        }
        self.activity.last_line = now_ms.div_euclid(MILLIS_PER_SECOND);
        self.tick(now_ms);
    }

    pub(super) fn tick(&mut self, now_ms: i64) {
        match on_tick(&self.activity, now_ms, self.terminal()) {
            Some(Output::Footer(rows)) => {
                let text: String = rows
                    .iter()
                    .map(|row| match self.colored {
                        true => format!("{}\n", painted(row)),
                        false => format!("{}\n", row.plain()),
                    })
                    .collect();
                self.draw(&text, rows.len());
            }
            Some(Output::Line(text)) => {
                self.draw(&format!("{text}\n"), 0);
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
        if self.footer > 0 {
            self.draw("", 0);
        }
    }

    fn draw(&mut self, text: &str, rows: usize) {
        let mut stderr = io::stderr().lock();
        let _ = rewind(&mut stderr, self.footer);
        let _ = stderr.write_all(printable(text).as_bytes());
        let _ = stderr.flush();
        self.footer = u16::try_from(rows).unwrap_or(u16::MAX);
    }
}

fn rewind(out: &mut impl Write, drawn: u16) -> io::Result<()> {
    if drawn == 0 {
        return Ok(());
    }
    queue!(
        out,
        cursor::MoveToPreviousLine(drawn),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    )
}

fn clock(now_ms: i64) -> String {
    Timestamp::from_millisecond(now_ms)
        .map(|at| at.to_zoned(TimeZone::system()).strftime(CLOCK).to_string())
        .unwrap_or_default()
}

pub(super) fn painted(row: &Row) -> String {
    format!(
        "{}{}{}",
        styled(SPINNING, &row.lead),
        row.label,
        styled(FADED, &row.faded)
    )
}

fn styled(style: Style, text: &str) -> String {
    match text.is_empty() {
        true => String::new(),
        false => format!("{style}{text}{style:#}"),
    }
}
