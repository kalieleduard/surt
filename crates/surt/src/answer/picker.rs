mod view;

use surt_engine::project::{Choice, Question};

pub(super) use view::{Palette, Position, answered, lines};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Key {
    Up,
    Down,
    Enter,
    Back,
    Erase,
    Quit,
    Char(char),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Step {
    Pending,
    Answered(String),
    Abandoned,
}

pub(super) struct Picker<'a> {
    question: &'a Question,
    cursor: usize,
    typed: Option<String>,
}

impl<'a> Picker<'a> {
    pub(super) fn new(question: &'a Question) -> Picker<'a> {
        let Some(choices) = question.options() else {
            return Picker {
                question,
                cursor: 0,
                typed: Some(String::new()),
            };
        };
        let recommended = choices
            .iter()
            .position(|choice| choice.value() == question.recommended())
            .unwrap_or(0);
        Picker {
            question,
            cursor: recommended,
            typed: None,
        }
    }

    pub(super) fn question(&self) -> &Question {
        self.question
    }

    pub(super) fn choices(&self) -> &[Choice] {
        self.question.options().unwrap_or_default()
    }

    pub(super) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(super) fn typed(&self) -> Option<&str> {
        self.typed.as_deref()
    }

    pub(super) fn press(&mut self, key: Key) -> Step {
        if key == Key::Quit {
            return Step::Abandoned;
        }
        match self.typed.take() {
            Some(typed) => self.type_key(typed, key),
            None => self.choose_key(key),
        }
    }

    fn entries(&self) -> usize {
        self.choices().len() + 1
    }

    fn choose_key(&mut self, key: Key) -> Step {
        let entries = self.entries();
        match key {
            Key::Up => self.cursor = (self.cursor + entries - 1) % entries,
            Key::Down => self.cursor = (self.cursor + 1) % entries,
            Key::Enter => return self.chosen(),
            Key::Back => return Step::Abandoned,
            Key::Char(digit) => return self.picked(digit),
            Key::Erase | Key::Quit => {}
        }
        Step::Pending
    }

    fn picked(&mut self, digit: char) -> Step {
        let Some(number) = digit.to_digit(10).and_then(|n| usize::try_from(n).ok()) else {
            return Step::Pending;
        };
        if number == 0 || number > self.choices().len() {
            return Step::Pending;
        }
        self.cursor = number - 1;
        self.chosen()
    }

    fn chosen(&mut self) -> Step {
        match self.choices().get(self.cursor) {
            Some(choice) => Step::Answered(choice.value().to_owned()),
            None => {
                self.typed = Some(String::new());
                Step::Pending
            }
        }
    }

    fn type_key(&mut self, mut typed: String, key: Key) -> Step {
        match key {
            Key::Char(character) => typed.push(character),
            Key::Erase => {
                typed.pop();
            }
            Key::Enter => return self.confirmed(typed),
            Key::Back if self.question.options().is_some() => return Step::Pending,
            Key::Back => return Step::Abandoned,
            Key::Up | Key::Down | Key::Quit => {}
        }
        self.typed = Some(typed);
        Step::Pending
    }

    fn confirmed(&mut self, typed: String) -> Step {
        let answer = typed.trim();
        if !answer.is_empty() {
            return Step::Answered(answer.to_owned());
        }
        if self.question.options().is_some() {
            return Step::Pending;
        }
        Step::Answered(self.question.recommended().to_owned())
    }
}

#[cfg(test)]
mod tests;
