pub(crate) fn number(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

pub(crate) fn count(value: i64) -> usize {
    usize::try_from(value.max(0)).unwrap_or(usize::MAX)
}
