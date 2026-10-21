use std::collections::BTreeMap;
use std::io::{self, Write};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, queue, terminal};
use surt_engine::project::{Question, Questions, stderr_columns};

use super::picker::{Key, Palette, Picker, Position, Step, answered, lines};
use crate::message::stderr_colored;
use crate::symbols::printable;

pub(super) fn ask(questions: &Questions) -> io::Result<Option<BTreeMap<String, String>>> {
    let _raw = RawMode::enter()?;
    let palette = if stderr_colored() {
        Palette::colored()
    } else {
        Palette::plain()
    };
    let total = questions.questions().len();
    let mut canvas = Canvas::default();
    let mut answers = BTreeMap::new();
    for (index, question) in questions.questions().iter().enumerate() {
        let at = Position {
            number: index + 1,
            total,
        };
        let Some(answer) = pick(question, at, &palette, &mut canvas)? else {
            canvas.erase()?;
            return Ok(None);
        };
        canvas.draw(&answered(question, &answer, at, stderr_columns(), &palette))?;
        canvas.keep();
        answers.insert(question.id().to_owned(), answer);
    }
    Ok(Some(answers))
}

fn pick(
    question: &Question,
    at: Position,
    palette: &Palette,
    canvas: &mut Canvas,
) -> io::Result<Option<String>> {
    let mut picker = Picker::new(question);
    loop {
        canvas.draw(&lines(&picker, at, stderr_columns(), palette))?;
        let Some(key) = next_key()? else {
            continue;
        };
        match picker.press(key) {
            Step::Pending => {}
            Step::Answered(answer) => return Ok(Some(answer)),
            Step::Abandoned => return Ok(None),
        }
    }
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
        KeyCode::Up | KeyCode::BackTab => Some(Key::Up),
        KeyCode::Down | KeyCode::Tab => Some(Key::Down),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Back),
        KeyCode::Backspace => Some(Key::Erase),
        _ => None,
    }
}

struct RawMode;

impl RawMode {
    fn enter() -> io::Result<RawMode> {
        terminal::enable_raw_mode()?;
        let mut stderr = io::stderr();
        queue!(stderr, cursor::Hide)?;
        stderr.flush()?;
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

#[derive(Default)]
struct Canvas {
    drawn: u16,
}

impl Canvas {
    fn draw(&mut self, lines: &[String]) -> io::Result<()> {
        let mut stderr = io::stderr().lock();
        self.rewind(&mut stderr)?;
        for line in lines {
            write!(stderr, "{}\r\n", printable(line))?;
        }
        self.drawn = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        stderr.flush()
    }

    fn keep(&mut self) {
        self.drawn = 0;
    }

    fn erase(&mut self) -> io::Result<()> {
        let mut stderr = io::stderr().lock();
        self.rewind(&mut stderr)?;
        self.drawn = 0;
        stderr.flush()
    }

    fn rewind(&self, out: &mut impl Write) -> io::Result<()> {
        if self.drawn > 0 {
            queue!(out, cursor::MoveToPreviousLine(self.drawn))?;
        } else {
            queue!(out, cursor::MoveToColumn(0))?;
        }
        queue!(out, terminal::Clear(terminal::ClearType::FromCursorDown))
    }
}
