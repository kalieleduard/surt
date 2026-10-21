use anstyle::{AnsiColor, Style};
use surt_engine::project::{Choice, Question};

use super::Picker;
use crate::symbols::{ARROW, BAR, DOWN, ELLIPSIS, POINTER, QUESTION, SEPARATOR, SUCCESS, UP};

const BODY: &str = "      ";
const ENTRY: &str = "      ";
const POINTED: &str = "    ";
const DETAIL: &str = "        ";
const HINT: &str = "  ";
const SLACK: usize = 2;
const NARROWEST: usize = 20;
const OTHER: &str = "type another answer";
const RECOMMENDED: &str = "recommended";

pub(in crate::answer) struct Palette {
    accent: Style,
    asking: Style,
    done: Style,
    strong: Style,
    faded: Style,
}

#[derive(Clone, Copy)]
pub(in crate::answer) struct Position {
    pub(in crate::answer) number: usize,
    pub(in crate::answer) total: usize,
}

impl Palette {
    pub(in crate::answer) fn plain() -> Palette {
        Palette {
            accent: Style::new(),
            asking: Style::new(),
            done: Style::new(),
            strong: Style::new(),
            faded: Style::new(),
        }
    }

    pub(in crate::answer) fn colored() -> Palette {
        Palette {
            accent: AnsiColor::Cyan.on_default().bold(),
            asking: AnsiColor::Yellow.on_default().bold(),
            done: AnsiColor::Green.on_default().bold(),
            strong: Style::new().bold(),
            faded: Style::new().dimmed(),
        }
    }
}

fn paint(style: Style, text: &str) -> String {
    format!("{style}{text}{style:#}")
}

pub(in crate::answer) fn lines(
    picker: &Picker<'_>,
    at: Position,
    columns: usize,
    palette: &Palette,
) -> Vec<String> {
    let width = columns.saturating_sub(SLACK).max(NARROWEST);
    let mut lines = heading(picker.question(), at, width, palette);
    lines.push(String::new());
    match picker.typed() {
        Some(typed) => lines.extend(typing(picker, typed, width, palette)),
        None => lines.extend(choosing(picker, width, palette)),
    }
    lines.push(String::new());
    lines.extend(hint(picker, width, palette));
    lines
}

pub(in crate::answer) fn answered(
    question: &Question,
    answer: &str,
    at: Position,
    columns: usize,
    palette: &Palette,
) -> Vec<String> {
    let width = columns.saturating_sub(SLACK).max(NARROWEST);
    let counter = format!("{}/{}", at.number, at.total);
    let room = width.saturating_sub(BODY.len());
    let asked = shortened(question.question(), room);
    let mut lines = vec![format!(
        "{} {} {}",
        paint(palette.done, SUCCESS),
        paint(palette.strong, &counter),
        paint(palette.faded, &asked)
    )];
    for (index, line) in wrap(answer, room.saturating_sub(2)).iter().enumerate() {
        let lead = if index == 0 { ARROW } else { " " };
        lines.push(format!(
            "{BODY}{} {}",
            paint(palette.accent, lead),
            paint(palette.accent, line)
        ));
    }
    lines
}

fn heading(question: &Question, at: Position, width: usize, palette: &Palette) -> Vec<String> {
    let room = width.saturating_sub(BODY.len());
    let counter = format!("{}/{}", at.number, at.total);
    let mut lines: Vec<String> = wrap(question.question(), room)
        .into_iter()
        .enumerate()
        .map(|(index, line)| match index {
            0 => format!(
                "{} {} {}",
                paint(palette.asking, QUESTION),
                paint(palette.strong, &counter),
                paint(palette.strong, &line)
            ),
            _ => format!("{BODY}{}", paint(palette.strong, &line)),
        })
        .collect();
    let why = format!("why: {}", question.why());
    lines.extend(
        wrap(&why, room)
            .into_iter()
            .map(|line| format!("{BODY}{}", paint(palette.faded, &line))),
    );
    lines
}

fn choosing(picker: &Picker<'_>, width: usize, palette: &Palette) -> Vec<String> {
    let recommended = picker.question().recommended();
    let mut lines = Vec::new();
    for (index, choice) in picker.choices().iter().enumerate() {
        let pointed = index == picker.cursor();
        let tag = (choice.value() == recommended).then_some(RECOMMENDED);
        lines.extend(entry(choice.value(), tag, pointed, width, palette));
        lines.extend(described(choice, width, palette));
    }
    let pointed = picker.cursor() == picker.choices().len();
    lines.extend(entry(OTHER, None, pointed, width, palette));
    lines
}

fn entry(
    value: &str,
    tag: Option<&str>,
    pointed: bool,
    width: usize,
    palette: &Palette,
) -> Vec<String> {
    let room = width.saturating_sub(ENTRY.len() + tag.map_or(0, |tag| tag.len() + 2));
    let style = if pointed {
        palette.accent
    } else {
        Style::new()
    };
    let mut lines: Vec<String> = wrap(value, room)
        .into_iter()
        .enumerate()
        .map(|(index, line)| match (index, pointed) {
            (0, true) => format!("{POINTED}{} {}", paint(style, POINTER), paint(style, &line)),
            _ => format!("{ENTRY}{}", paint(style, &line)),
        })
        .collect();
    if let (Some(tag), Some(first)) = (tag, lines.first_mut()) {
        first.push_str(&format!("  {}", paint(palette.done, tag)));
    }
    lines
}

fn described(choice: &Choice, width: usize, palette: &Palette) -> Vec<String> {
    let Some(description) = choice.description() else {
        return Vec::new();
    };
    wrap(description, width.saturating_sub(DETAIL.len()))
        .into_iter()
        .map(|line| format!("{DETAIL}{}", paint(palette.faded, &line)))
        .collect()
}

fn typing(picker: &Picker<'_>, typed: &str, width: usize, palette: &Palette) -> Vec<String> {
    let label = match picker.question().options() {
        Some(_) => format!("{OTHER}: "),
        None => String::new(),
    };
    let room = width.saturating_sub(POINTED.len() + 2 + label.len() + 1);
    let shown = if typed.is_empty() && picker.question().options().is_none() {
        format!(
            "{}{}",
            paint(palette.accent, BAR),
            paint(palette.faded, picker.question().recommended())
        )
    } else {
        format!(
            "{}{}",
            paint(palette.accent, &tail(typed, room)),
            paint(palette.accent, BAR)
        )
    };
    vec![format!(
        "{POINTED}{} {}{shown}",
        paint(palette.accent, POINTER),
        paint(palette.accent, &label)
    )]
}

fn hint(picker: &Picker<'_>, width: usize, palette: &Palette) -> Vec<String> {
    let keys = match (picker.typed(), picker.question().options()) {
        (None, Some(choices)) => vec![
            format!("{UP}/{DOWN} move"),
            "enter choose".to_owned(),
            format!("1-{} pick", choices.len()),
            "esc answer later".to_owned(),
        ],
        (Some(_), Some(_)) => vec![
            "enter confirm".to_owned(),
            "esc back to the options".to_owned(),
        ],
        (_, None) => vec![
            "enter confirm".to_owned(),
            "empty takes the recommendation".to_owned(),
            "esc answer later".to_owned(),
        ],
    };
    wrap(&keys.join(SEPARATOR), width.saturating_sub(HINT.len()))
        .into_iter()
        .map(|line| format!("{HINT}{}", paint(palette.faded, &line)))
        .collect()
}

fn tail(text: &str, room: usize) -> String {
    let count = text.chars().count();
    text.chars().skip(count.saturating_sub(room)).collect()
}

fn shortened(text: &str, room: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= room {
        return flat;
    }
    let kept: String = flat.chars().take(room.saturating_sub(1)).collect();
    let whole_words = kept
        .rfind(' ')
        .map_or(kept.as_str(), |space| &kept[..space]);
    format!("{}{ELLIPSIS}", whole_words.trim_end())
}

pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        for piece in pieces(word, width) {
            let used = line.chars().count();
            if used > 0 && used + 1 + piece.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&piece);
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn pieces(word: &str, width: usize) -> Vec<String> {
    let characters: Vec<char> = word.chars().collect();
    characters
        .chunks(width)
        .map(|chunk| chunk.iter().collect())
        .collect()
}
