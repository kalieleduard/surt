use anstyle::{AnsiColor, Style};

use crate::symbols;

#[derive(Clone, Copy)]
pub(crate) enum Mark {
    Success,
    Error,
    Paused,
    Blocked,
    Stopped,
    Started,
    Retry,
    Warning,
    Waiting,
}

impl Mark {
    fn symbol(self) -> &'static str {
        match self {
            Mark::Success => symbols::SUCCESS,
            Mark::Error => symbols::ERROR,
            Mark::Paused => symbols::PAUSED,
            Mark::Blocked => symbols::BLOCKED,
            Mark::Stopped => symbols::STOPPED,
            Mark::Started => symbols::STARTED,
            Mark::Retry => symbols::RETRY,
            Mark::Warning => symbols::WARNING,
            Mark::Waiting => symbols::WAITING,
        }
    }

    fn style(self) -> Style {
        match self {
            Mark::Success => AnsiColor::Green.on_default().bold(),
            Mark::Error | Mark::Blocked => AnsiColor::Red.on_default().bold(),
            Mark::Paused | Mark::Stopped | Mark::Retry | Mark::Warning | Mark::Waiting => {
                AnsiColor::Yellow.on_default().bold()
            }
            Mark::Started => AnsiColor::Cyan.on_default().bold(),
        }
    }
}

pub(crate) struct Message {
    headline: String,
    lines: Vec<String>,
}

impl Message {
    pub(crate) fn new(headline: impl Into<String>) -> Message {
        Message {
            headline: headline.into(),
            lines: Vec::new(),
        }
    }

    pub(crate) fn context(mut self, line: impl Into<String>) -> Message {
        self.lines.push(line.into());
        self
    }

    pub(crate) fn way(mut self, label: &str, command: impl AsRef<str>) -> Message {
        self.lines.push(format!("{label}: {}", command.as_ref()));
        self
    }

    pub(crate) fn print_error(&self) {
        self.print(Mark::Error);
    }

    pub(crate) fn print_success(&self) {
        self.print(Mark::Success);
    }

    pub(crate) fn print(&self, mark: Mark) {
        anstream::eprint!(
            "{}",
            symbols::printable(&self.render(mark.symbol(), mark.style()))
        );
    }

    fn render(&self, symbol: &str, style: Style) -> String {
        let mut text = format!("{style}{symbol}{style:#} {}\n", self.headline);
        for line in &self.lines {
            text.push_str("  ");
            text.push_str(line);
            text.push('\n');
        }
        text
    }

    #[cfg(test)]
    pub(crate) fn plain_error(&self) -> String {
        self.render(symbols::ERROR, Style::new())
    }

    #[cfg(test)]
    pub(crate) fn plain(&self, mark: Mark) -> String {
        self.render(mark.symbol(), Style::new())
    }
}
