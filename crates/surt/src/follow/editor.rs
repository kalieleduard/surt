#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Enter,
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Pending,
    Submitted(String),
    Quit,
}

pub(crate) struct LineEditor {
    text: Vec<char>,
    cursor: usize,
}

impl LineEditor {
    pub(crate) fn new(initial: &str) -> LineEditor {
        let text: Vec<char> = initial.chars().collect();
        let cursor = text.len();
        LineEditor { text, cursor }
    }

    pub(crate) fn text(&self) -> String {
        self.text.iter().collect()
    }

    pub(crate) fn chars(&self) -> &[char] {
        &self.text
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn press(&mut self, key: Key) -> Step {
        match key {
            Key::Char(character) => {
                self.text.insert(self.cursor, character);
                self.cursor += 1;
            }
            Key::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.cursor);
            }
            Key::Delete if self.cursor < self.text.len() => {
                self.text.remove(self.cursor);
            }
            Key::Backspace | Key::Delete => {}
            Key::Left => self.cursor = self.cursor.saturating_sub(1),
            Key::Right => self.cursor = (self.cursor + 1).min(self.text.len()),
            Key::Home => self.cursor = 0,
            Key::End => self.cursor = self.text.len(),
            Key::Enter => return Step::Submitted(self.text()),
            Key::Quit => return Step::Quit,
        }
        Step::Pending
    }
}

#[cfg(test)]
mod tests;
