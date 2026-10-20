use std::collections::HashSet;

const DEVICE_NAMES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
const NUMBERED_DEVICE_PREFIXES: [&str; 2] = ["COM", "LPT"];
const MAX_KEY_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyFault {
    NotPortable,
    Reserved,
    Duplicate,
}

pub(super) fn is_portable_path(path: &str) -> bool {
    !path.is_empty() && path.split('/').all(is_portable_component)
}

pub(super) fn first_key_fault<'a>(
    keys: impl IntoIterator<Item = &'a str>,
) -> Option<(&'a str, KeyFault)> {
    let mut seen = HashSet::new();
    keys.into_iter()
        .find_map(|key| key_fault(key, &mut seen).map(|fault| (key, fault)))
}

fn key_fault(key: &str, seen: &mut HashSet<String>) -> Option<KeyFault> {
    if !is_item_key(key) {
        return Some(KeyFault::NotPortable);
    }
    if is_device_name(key) {
        return Some(KeyFault::Reserved);
    }
    (!seen.insert(key.to_ascii_lowercase())).then_some(KeyFault::Duplicate)
}

fn is_item_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let starts_alphanumeric = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    starts_alphanumeric
        && key.len() <= MAX_KEY_LEN
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_device_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    DEVICE_NAMES.contains(&upper.as_str())
        || NUMBERED_DEVICE_PREFIXES.iter().any(|prefix| {
            upper
                .strip_prefix(prefix)
                .is_some_and(|number| number.len() == 1 && number.as_bytes()[0].is_ascii_digit())
        })
}

fn is_portable_component(component: &str) -> bool {
    let stem = component
        .split_once('.')
        .map_or(component, |(stem, _)| stem);
    !component.is_empty()
        && !component.ends_with('.')
        && component
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !is_device_name(stem)
}
