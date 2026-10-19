const SHELL_SPECIAL: [char; 4] = ['\\', '"', '$', '`'];
const PLAIN_PUNCTUATION: &str = "-_./:@^~";

pub(crate) struct Retry {
    words: Vec<String>,
}

impl Retry {
    pub(crate) fn command(words: &[&str]) -> Retry {
        Retry {
            words: words.iter().map(|text| word(text)).collect(),
        }
    }

    pub(crate) fn option(mut self, flag: &str, value: Option<&str>) -> Retry {
        if let Some(value) = value {
            self.words.push(flag.to_owned());
            self.words.push(word(value));
        }
        self
    }

    pub(crate) fn word(mut self, word: &str) -> Retry {
        self.words.push(word.to_owned());
        self
    }

    pub(crate) fn render(&self) -> String {
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
