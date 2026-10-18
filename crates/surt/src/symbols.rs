use std::borrow::Cow;
use std::sync::OnceLock;

use surt_engine::project::console_outputs_utf8;

pub(crate) const SUCCESS: &str = "✔";
pub(crate) const ERROR: &str = "✖";
pub(crate) const WARNING: &str = "⚠";
pub(crate) const RETRY: &str = "↻";
pub(crate) const PAUSED: &str = "⏸";
pub(crate) const BLOCKED: &str = "⛔";
pub(crate) const STOPPED: &str = "⏹";
pub(crate) const STARTED: &str = "●";
pub(crate) const WAITING: &str = "⏳";
pub(crate) const EDITED: &str = "✎";
pub(crate) const INTEGRATING: &str = "⟳";
pub(crate) const VERTICAL: &str = "│";
pub(crate) const HORIZONTAL: &str = "─";
pub(crate) const ELLIPSIS: &str = "…";
pub(crate) const MINUS: &str = "−";
pub(crate) const DASH: &str = "—";
pub(crate) const DOT: &str = "·";
pub(crate) const ARROW: &str = "→";
pub(crate) const SEPARATOR: &str = " · ";

const ASCII: [(&str, &str); 18] = [
    (SUCCESS, "[ok]"),
    (ERROR, "[error]"),
    (WARNING, "[warn]"),
    (RETRY, "[retry]"),
    (PAUSED, "[paused]"),
    (BLOCKED, "[blocked]"),
    (STOPPED, "[stopped]"),
    (STARTED, "*"),
    (WAITING, "[waiting]"),
    (EDITED, "[edited]"),
    (INTEGRATING, "[integrating]"),
    (VERTICAL, "|"),
    (HORIZONTAL, "-"),
    (ELLIPSIS, "..."),
    (MINUS, "-"),
    (DASH, "-"),
    (DOT, "|"),
    (ARROW, "->"),
];

#[derive(Clone, Copy)]
pub(crate) enum Symbols {
    Unicode,
    Ascii,
}

impl Symbols {
    fn of_console() -> Symbols {
        static CONSOLE: OnceLock<Symbols> = OnceLock::new();
        *CONSOLE.get_or_init(|| {
            if console_outputs_utf8() {
                Symbols::Unicode
            } else {
                Symbols::Ascii
            }
        })
    }

    pub(crate) fn render(self, text: &str) -> Cow<'_, str> {
        match self {
            Symbols::Ascii if !text.is_ascii() => Cow::Owned(to_ascii(text)),
            Symbols::Ascii | Symbols::Unicode => Cow::Borrowed(text),
        }
    }
}

pub(crate) fn printable(text: &str) -> Cow<'_, str> {
    Symbols::of_console().render(text)
}

fn to_ascii(text: &str) -> String {
    let mut rendered = String::with_capacity(text.len());
    let mut buffer = [0; 4];
    for character in text.chars() {
        let glyph: &str = character.encode_utf8(&mut buffer);
        let ascii = ASCII
            .iter()
            .find_map(|(unicode, ascii)| (*unicode == glyph).then_some(*ascii));
        rendered.push_str(ascii.unwrap_or(glyph));
    }
    rendered
}

#[cfg(test)]
mod tests;
