const DEVICE_NAMES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
const NUMBERED_DEVICE_PREFIXES: [&str; 2] = ["COM", "LPT"];
const MAX_KEY_LEN: usize = 32;

pub(super) fn is_portable_path(path: &str) -> bool {
    !path.is_empty() && path.split('/').all(is_portable_component)
}

pub(super) fn is_item_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let starts_alphanumeric = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    starts_alphanumeric
        && key.len() <= MAX_KEY_LEN
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub(super) fn is_device_name(name: &str) -> bool {
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
