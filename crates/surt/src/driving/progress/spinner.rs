use crate::symbols::Symbols;

pub(super) const FRAME_MS: i64 = 100;

pub(super) fn frame(elapsed_ms: i64, symbols: Symbols) -> &'static str {
    let frames = symbols.spinner();
    let step = elapsed_ms.div_euclid(FRAME_MS);
    frames[step.rem_euclid(frames.len() as i64) as usize]
}

#[cfg(test)]
mod tests;
