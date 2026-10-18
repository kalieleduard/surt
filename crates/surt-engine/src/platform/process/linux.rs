use std::fs;

pub(super) use super::signals::signal;

const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";
const STATE_FIELD: usize = 0;
const START_TIME_FIELD: usize = 19;
const ENDED_STATES: [&str; 2] = ["Z", "X"];

pub(super) fn start_time(pid: u32) -> Option<i64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, after_name) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    if ENDED_STATES.contains(fields.get(STATE_FIELD)?) {
        return None;
    }
    fields.get(START_TIME_FIELD)?.parse().ok()
}

pub(super) fn boot_id() -> Option<String> {
    let id = fs::read_to_string(BOOT_ID).ok()?;
    Some(id.trim().to_owned())
}
