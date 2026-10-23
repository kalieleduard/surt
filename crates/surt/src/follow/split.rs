use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SplitError {
    quote: char,
}

impl fmt::Display for SplitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unterminated {} quote", self.quote)
    }
}

impl std::error::Error for SplitError {}

pub(crate) fn split_line(line: &str) -> Result<Vec<String>, SplitError> {
    let mut arguments = Vec::new();
    let mut current: Option<String> = None;
    let mut quote: Option<char> = None;
    for character in line.chars() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => current.get_or_insert_default().push(character),
            None if character == '\'' || character == '"' => {
                quote = Some(character);
                current.get_or_insert_default();
            }
            None if character.is_whitespace() => arguments.extend(current.take()),
            None => current.get_or_insert_default().push(character),
        }
    }
    if let Some(quote) = quote {
        return Err(SplitError { quote });
    }
    arguments.extend(current);
    Ok(arguments)
}

#[cfg(test)]
mod tests;
