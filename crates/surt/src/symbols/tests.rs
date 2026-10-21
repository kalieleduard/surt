use std::fs;
use std::path::{Path, PathBuf};

use super::{ASCII_SPINNER, SPINNER, Symbols};

const SOURCE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
const TABLE_FILE: &str = "symbols.rs";
const TEST_FILE: &str = "tests.rs";
const TEST_DIRECTORY: &str = "tests";

#[test]
fn ascii_mode_maps_the_six_status_symbols_to_their_bracketed_words() {
    assert_eq!(
        Symbols::Ascii.render("✔ a\n✖ b\n⚠ c\n↻ d\n⏸ e\n⛔ f\n"),
        "[ok] a\n[error] b\n[warn] c\n[retry] d\n[paused] e\n[blocked] f\n"
    );
}

#[test]
fn ascii_mode_maps_every_other_glyph_to_an_ascii_equivalent() {
    assert_eq!(
        Symbols::Ascii.render("● ⏳ ⏹ ✎ ⟳ │ ─ … − — · →"),
        "* [waiting] [stopped] [edited] [integrating] | - ... - - | ->"
    );
}

#[test]
fn ascii_mode_maps_the_picker_glyphs_to_words_and_marks() {
    assert_eq!(Symbols::Ascii.render("❯ a ↑/↓ b▌"), "> a up/down b_");
}

#[test]
fn each_mode_has_its_own_spinner_and_the_ascii_one_needs_no_conversion() {
    assert_eq!(Symbols::Unicode.spinner(), SPINNER);
    assert_eq!(Symbols::Ascii.spinner(), ASCII_SPINNER);
    for frame in ASCII_SPINNER {
        assert_eq!(Symbols::Ascii.render(frame), frame);
    }
}

#[test]
fn ascii_mode_keeps_plain_text_and_unknown_characters() {
    assert_eq!(
        Symbols::Ascii.render("approved spec rev 2 by José"),
        "approved spec rev 2 by José"
    );
}

#[test]
fn unicode_mode_prints_the_glyphs_unchanged() {
    let text = "✔ spec rev 1 — approved · 3 files → next";

    assert_eq!(Symbols::Unicode.render(text), text);
}

#[test]
fn no_cli_source_prints_a_non_ascii_character_outside_the_symbol_table() {
    let offenders: Vec<String> = sources(Path::new(SOURCE_ROOT))
        .into_iter()
        .filter(|path| !path.ends_with(TABLE_FILE))
        .flat_map(|path| non_ascii_lines(&path))
        .collect();

    assert!(offenders.is_empty(), "{offenders:#?}");
}

fn sources(directory: &Path) -> Vec<PathBuf> {
    let entries = fs::read_dir(directory).expect("the source directory is readable");
    let mut found = Vec::new();
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|name| name.to_str());
        if matches!(name, Some(TEST_FILE | TEST_DIRECTORY)) {
            continue;
        }
        if path.is_dir() {
            found.extend(sources(&path));
        } else {
            found.push(path);
        }
    }
    found
}

fn non_ascii_lines(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).expect("a source file is UTF-8");
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.is_ascii())
        .map(|(number, line)| format!("{}:{}: {line}", path.display(), number + 1))
        .collect()
}
