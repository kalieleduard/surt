use std::iter;

use jiff::civil::Date;

const MAX_SLUG_CHARS: usize = 40;
const DATE_FORMAT: &str = "%Y%m%d";
const DATE_CHARS: usize = 8;
const FIRST_SUFFIX: u32 = 2;

const LATIN_FOLDS: [(&str, &str); 26] = [
    ("àáâãäåāăą", "a"),
    ("çćĉċč", "c"),
    ("ďđð", "d"),
    ("èéêëēĕėęě", "e"),
    ("ĝğġģ", "g"),
    ("ĥħ", "h"),
    ("ìíîïĩīĭįı", "i"),
    ("ĵ", "j"),
    ("ķ", "k"),
    ("ĺļľŀł", "l"),
    ("ñńņňŉ", "n"),
    ("òóôõöøōŏő", "o"),
    ("ŕŗř", "r"),
    ("śŝşšș", "s"),
    ("ţťŧț", "t"),
    ("ùúûüũūŭůűų", "u"),
    ("ŵ", "w"),
    ("ýÿŷ", "y"),
    ("źżž", "z"),
    ("æ", "ae"),
    ("œ", "oe"),
    ("ß", "ss"),
    ("þ", "th"),
    ("ĳ", "ij"),
    ("ŋ", "ng"),
    ("ſ", "s"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub fn derive(text: &str) -> Option<Slug> {
        let slug = slug(text, MAX_SLUG_CHARS);
        if slug.is_empty() {
            return None;
        }
        Some(Slug(slug))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn dated_name(slug: &Slug, date: Date) -> String {
    format!("{}-{}", date.strftime(DATE_FORMAT), slug.as_str())
}

pub(crate) fn first_free(stem: &str, taken: &[String]) -> String {
    let is_free = |candidate: &String| taken.iter().all(|name| name != candidate);
    iter::once(stem.to_owned())
        .chain((FIRST_SUFFIX..).map(|suffix| format!("{stem}-{suffix}")))
        .find(is_free)
        .unwrap_or_else(|| stem.to_owned())
}

pub(crate) fn is_named_after(name: &str, slug: &Slug) -> bool {
    let Some(undated) = without_date(name) else {
        return false;
    };
    let Some(rest) = undated.strip_prefix(slug.as_str()) else {
        return false;
    };
    rest.is_empty()
        || rest.strip_prefix('-').is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
}

pub(crate) fn without_date(name: &str) -> Option<&str> {
    let (date, rest) = name.split_at_checked(DATE_CHARS)?;
    if !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    rest.strip_prefix('-')
}

pub(crate) fn slug(text: &str, max_chars: usize) -> String {
    let ascii = transliterate(text);
    let words = ascii
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty());
    let mut slug = String::new();
    for word in words {
        let separator = usize::from(!slug.is_empty());
        if slug.len() + separator + word.len() > max_chars {
            if slug.is_empty() {
                slug.push_str(&word[..max_chars]);
            }
            break;
        }
        if separator == 1 {
            slug.push('-');
        }
        slug.push_str(word);
    }
    slug
}

fn transliterate(text: &str) -> String {
    let mut ascii = String::with_capacity(text.len());
    for character in text.chars().flat_map(char::to_lowercase) {
        match fold(character) {
            Some(folded) => ascii.push_str(folded),
            None => ascii.push(character),
        }
    }
    ascii
}

fn fold(character: char) -> Option<&'static str> {
    LATIN_FOLDS
        .iter()
        .find(|(accented, _)| accented.contains(character))
        .map(|(_, plain)| *plain)
}

#[cfg(test)]
mod tests;
