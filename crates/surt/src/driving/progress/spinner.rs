use crate::symbols::SPINNER;

pub(super) const FRAME_MS: i64 = 100;

#[allow(dead_code)]
pub(super) fn frame(elapsed_ms: i64) -> &'static str {
    let step = elapsed_ms.div_euclid(FRAME_MS);
    SPINNER[step.rem_euclid(SPINNER.len() as i64) as usize]
}

#[cfg(test)]
mod tests;
