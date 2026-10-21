use crate::symbols::{ELLIPSIS, Symbols};

const LAST_COLUMN: usize = 1;
const ELLIPSIS_ROOM: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) lead: String,
    pub(super) label: String,
    pub(super) faded: String,
}

impl Row {
    pub(super) fn plain(&self) -> String {
        format!("{}{}{}", self.lead, self.label, self.faded)
    }

    pub(super) fn rendered(self, symbols: Symbols) -> Row {
        Row {
            lead: symbols.render(&self.lead).into_owned(),
            label: symbols.render(&self.label).into_owned(),
            faded: symbols.render(&self.faded).into_owned(),
        }
    }

    pub(super) fn fit(self, width: usize) -> Row {
        let plain = self.plain();
        let fitted = fit(&plain, width);
        if fitted == plain {
            return self;
        }
        let lead_end = self.lead.chars().count();
        let label_end = lead_end + self.label.chars().count();
        Row {
            lead: slice(&fitted, 0, lead_end),
            label: slice(&fitted, lead_end, label_end),
            faded: slice(&fitted, label_end, usize::MAX),
        }
    }
}

fn slice(text: &str, from: usize, to: usize) -> String {
    text.chars().skip(from).take(to - from).collect()
}

fn fit(text: &str, width: usize) -> String {
    let limit = width.saturating_sub(LAST_COLUMN);
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    if limit < ELLIPSIS_ROOM {
        return text.chars().take(limit).collect();
    }
    let kept: String = text
        .chars()
        .take(limit.saturating_sub(ELLIPSIS_ROOM))
        .collect();
    format!("{}{ELLIPSIS}", kept.trim_end())
}
