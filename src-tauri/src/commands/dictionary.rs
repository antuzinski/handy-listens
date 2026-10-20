use crate::dictionary::{self, DictionaryFileKind, DictionaryStatus};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
#[specta::specta]
pub fn get_dictionary_status(app: AppHandle) -> Result<DictionaryStatus, String> {
    let path = dictionary::ensure_file(&app, DictionaryFileKind::Dictionary)?;
    Ok(dictionary::refresh(&app).status(&path))
}

/// Opens one of the two dictionary files in the default editor. Only these
/// fixed files in the app data dir can be opened; no path comes from the UI.
#[tauri::command]
#[specta::specta]
pub fn open_dictionary_file(app: AppHandle, kind: DictionaryFileKind) -> Result<(), String> {
    let path = dictionary::ensure_file(&app, kind)?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))
}

#[tauri::command]
#[specta::specta]
pub fn reset_dictionary_file(app: AppHandle, kind: DictionaryFileKind) -> Result<(), String> {
    dictionary::reset_file(&app, kind)?;
    dictionary::refresh(&app);
    Ok(())
}

/// Applies the dictionary replacements to sample text (no recognition, no LLM).
#[tauri::command]
#[specta::specta]
pub fn preview_dictionary(app: AppHandle, text: String) -> String {
    dictionary::refresh(&app).apply(&text)
}
