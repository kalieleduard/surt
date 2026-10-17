use anstyle::{AnsiColor, Style};

const ERROR: Style = AnsiColor::Red.on_default().bold();
const SUCCESS: Style = AnsiColor::Green.on_default().bold();

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
        self.print("✖", ERROR);
    }

    pub(crate) fn print_success(&self) {
        self.print("✔", SUCCESS);
    }

    fn print(&self, symbol: &str, style: Style) {
        anstream::eprint!("{}", self.render(symbol, style));
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
        self.render("✖", Style::new())
    }
}
