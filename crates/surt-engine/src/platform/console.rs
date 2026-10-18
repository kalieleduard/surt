#[cfg_attr(unix, path = "console/unix.rs")]
#[cfg_attr(windows, path = "console/windows.rs")]
mod native;

pub use native::outputs_utf8;

#[cfg(test)]
mod tests;
