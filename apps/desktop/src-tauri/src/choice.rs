use std::path::{Path, PathBuf};

const CHOICE_FILE: &str = "prototype-choice.json";

#[tauri::command]
pub fn approve_choice(choice: serde_json::Value) -> Result<String, String> {
    let path = choice_path()?;
    let text = serde_json::to_string_pretty(&choice).map_err(|error| error.to_string())?;
    std::fs::write(&path, format!("{text}\n")).map_err(|error| error.to_string())?;
    Ok(path.display().to_string())
}

fn choice_path() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|app_dir| app_dir.join(CHOICE_FILE))
        .ok_or_else(|| "the desktop app directory is unknown".to_string())
}
