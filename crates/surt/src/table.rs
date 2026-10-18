const GAP: usize = 2;

pub(crate) fn render<const COLUMNS: usize>(rows: &[[String; COLUMNS]]) -> String {
    let widths: Vec<usize> = (0..COLUMNS)
        .map(|column| {
            rows.iter()
                .map(|row| row[column].chars().count())
                .max()
                .unwrap_or_default()
        })
        .collect();
    rows.iter()
        .map(|row| {
            let mut text = String::new();
            for (cell, width) in row.iter().zip(&widths) {
                text.push_str(&format!("{cell:<padded$}", padded = width + GAP));
            }
            format!("{}\n", text.trim_end())
        })
        .collect()
}
