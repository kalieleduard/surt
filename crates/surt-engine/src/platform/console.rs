#[cfg_attr(unix, path = "console/unix.rs")]
#[cfg_attr(windows, path = "console/windows.rs")]
mod native;

pub use native::outputs_utf8;

const FALLBACK_COLUMNS: usize = 80;

pub fn stderr_columns() -> usize {
    native::stderr_columns()
        .or_else(|| columns_variable(std::env::var("COLUMNS").ok()))
        .unwrap_or(FALLBACK_COLUMNS)
}

fn columns_variable(value: Option<String>) -> Option<usize> {
    value?.trim().parse().ok().filter(|columns| *columns > 0)
}

#[cfg(test)]
mod tests;
