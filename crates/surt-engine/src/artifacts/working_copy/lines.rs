const NEWLINE: u8 = b'\n';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LineCounts {
    pub(super) added: usize,
    pub(super) removed: usize,
}

pub(super) fn counts(old: &[u8], new: &[u8]) -> LineCounts {
    let old = lines(old);
    let new = lines(new);
    let common = longest_common(&old, &new);
    LineCounts {
        added: new.len() - common,
        removed: old.len() - common,
    }
}

fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    let trimmed = bytes.strip_suffix(&[NEWLINE]).unwrap_or(bytes);
    if bytes.is_empty() {
        return Vec::new();
    }
    trimmed.split(|byte| *byte == NEWLINE).collect()
}

fn longest_common(old: &[&[u8]], new: &[&[u8]]) -> usize {
    let mut previous = vec![0; new.len() + 1];
    for line in old {
        let mut current = vec![0; new.len() + 1];
        for (index, other) in new.iter().enumerate() {
            current[index + 1] = if line == other {
                previous[index] + 1
            } else {
                current[index].max(previous[index + 1])
            };
        }
        previous = current;
    }
    previous[new.len()]
}

#[cfg(test)]
mod tests;
