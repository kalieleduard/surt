use std::io::{self, Write};

use anstyle::Style;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, queue, terminal};
use surt_engine::project::stderr_columns;

use super::editor::{Key, LineEditor, Step};
use crate::message::stderr_colored;
use crate::symbols::{DOT, POINTER, printable};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Edited {
    Submitted(String),
    Quit,
}

pub(crate) fn edit(initial: &str) -> io::Result<Edited> {
    let _raw = RawMode::enter()?;
    let prefix = printable(&format!("{POINTER} ")).into_owned();
    let hint = hint();
    let mut editor = LineEditor::new(initial);
    loop {
        draw(&editor, &prefix, &hint)?;
        let Some(key) = next_key()? else {
            continue;
        };
        match editor.press(key) {
            Step::Pending => {}
            Step::Submitted(text) => {
                let mut stderr = io::stderr().lock();
                rewind(&mut stderr)?;
                write!(stderr, "{prefix}{}\r\n", printable(&text))?;
                stderr.flush()?;
                return Ok(Edited::Submitted(text));
            }
            Step::Quit => {
                let mut stderr = io::stderr().lock();
                rewind(&mut stderr)?;
                stderr.flush()?;
                return Ok(Edited::Quit);
            }
        }
    }
}

fn hint() -> String {
    let text = printable(&format!("Enter executa {DOT} Esc sai")).into_owned();
    if stderr_colored() {
        let style = Style::new().dimmed();
        format!("{style}{text}{style:#}")
    } else {
        text
    }
}

fn draw(editor: &LineEditor, prefix: &str, hint: &str) -> io::Result<()> {
    let (shown, column) = visible(
        editor.chars(),
        editor.cursor(),
        stderr_columns(),
        prefix.chars().count(),
    );
    let mut stderr = io::stderr().lock();
    rewind(&mut stderr)?;
    write!(stderr, "{prefix}{}\r\n{hint}", printable(&shown))?;
    queue!(
        stderr,
        cursor::MoveToPreviousLine(1),
        cursor::MoveToColumn(column)
    )?;
    stderr.flush()
}

fn rewind(out: &mut impl Write) -> io::Result<()> {
    queue!(
        out,
        cursor::MoveToColumn(0),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    )
}

/// The part of `text` that fits after the prefix in `columns`, and the column
/// of the cursor. The last column stays free for the cursor so the line never
/// wraps.
fn visible(text: &[char], cursor: usize, columns: usize, prefix_width: usize) -> (String, u16) {
    let width = columns.saturating_sub(prefix_width + 1).max(1);
    let cursor = cursor.min(text.len());
    let start = cursor.saturating_sub(width);
    let end = (start + width).min(text.len());
    let shown = text[start..end].iter().collect();
    let column = u16::try_from(prefix_width + cursor - start).unwrap_or(u16::MAX);
    (shown, column)
}

fn next_key() -> io::Result<Option<Key>> {
    match event::read()? {
        Event::Key(key) if key.kind != KeyEventKind::Release => Ok(translated(key)),
        _ => Ok(None),
    }
}

fn translated(key: KeyEvent) -> Option<Key> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c' | 'd') if control => Some(Key::Quit),
        KeyCode::Char(character) if !control => Some(Key::Char(character)),
        KeyCode::Esc => Some(Key::Quit),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Delete => Some(Key::Delete),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        _ => None,
    }
}

struct RawMode;

impl RawMode {
    fn enter() -> io::Result<RawMode> {
        terminal::enable_raw_mode()?;
        Ok(RawMode)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let mut stderr = io::stderr();
        let _ = queue!(stderr, cursor::Show);
        let _ = stderr.flush();
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(test)]
mod tests;
