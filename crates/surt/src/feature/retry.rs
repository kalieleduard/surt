const SHELL_SPECIAL: [char; 4] = ['\\', '"', '$', '`'];
const PLAIN_PUNCTUATION: &str = "-_./:@^~";

pub(super) struct Retry {
    words: Vec<String>,
}

impl Retry {
    pub(super) fn of(description: &str) -> Retry {
        Retry {
            words: vec!["surt".to_owned(), "feature".to_owned(), quoted(description)],
        }
    }

    pub(super) fn option(mut self, flag: &str, value: Option<&str>) -> Retry {
        if let Some(value) = value {
            self.words.push(flag.to_owned());
            self.words.push(word(value));
        }
        self
    }

    pub(super) fn word(mut self, word: &str) -> Retry {
        self.words.push(word.to_owned());
        self
    }

    pub(super) fn render(&self) -> String {
        self.words.join(" ")
    }
}

pub(crate) fn quoted(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for character in text.chars() {
        if SHELL_SPECIAL.contains(&character) {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

pub(crate) fn word(text: &str) -> String {
    let plain = !text.is_empty()
        && text.chars().all(|character| {
            character.is_ascii_alphanumeric() || PLAIN_PUNCTUATION.contains(character)
        });
    if plain {
        return text.to_owned();
    }
    quoted(text)
}

#[cfg(test)]
mod tests;
