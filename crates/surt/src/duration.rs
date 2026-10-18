const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;

pub(crate) fn rounded(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < SECONDS_PER_MINUTE {
        format!("{seconds}s")
    } else if seconds < SECONDS_PER_HOUR {
        format!("{}m", seconds / SECONDS_PER_MINUTE)
    } else {
        format!(
            "{}h{:02}m",
            seconds / SECONDS_PER_HOUR,
            seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE
        )
    }
}

pub(crate) fn exact(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < SECONDS_PER_MINUTE {
        return rounded(seconds);
    }
    format!("{}{:02}s", rounded(seconds), seconds % SECONDS_PER_MINUTE)
}
