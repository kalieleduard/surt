const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_CTYPE", "LANG"];
const UTF8_NAMES: [&str; 2] = ["utf-8", "utf8"];

pub fn outputs_utf8() -> bool {
    locale_is_utf8(|name| std::env::var(name).ok())
}

pub(super) fn locale_is_utf8(read: impl Fn(&str) -> Option<String>) -> bool {
    LOCALE_VARIABLES
        .iter()
        .find_map(|name| read(name).filter(|value| !value.is_empty()))
        .is_none_or(|locale| names_utf8(&locale))
}

fn names_utf8(locale: &str) -> bool {
    let locale = locale.to_ascii_lowercase();
    UTF8_NAMES.iter().any(|name| locale.contains(name))
}
