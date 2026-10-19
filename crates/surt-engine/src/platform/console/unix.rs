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

pub(super) fn stderr_columns() -> Option<usize> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let status = unsafe { libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut size) };
    (status == 0 && size.ws_col > 0).then_some(usize::from(size.ws_col))
}

fn names_utf8(locale: &str) -> bool {
    let locale = locale.to_ascii_lowercase();
    UTF8_NAMES.iter().any(|name| locale.contains(name))
}
