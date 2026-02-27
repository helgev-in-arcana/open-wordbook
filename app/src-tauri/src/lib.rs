use tauri::{AppHandle, Manager};
use std::path::PathBuf;

mod db;
use db::{Word, Definition, RelatedWord};

fn resolve_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    // This is for resource bundling
    // "resources/words.sqlite3" in tauri.conf.json
    let resource_path = app.path().resolve("resources/words.sqlite3", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve resource path: {}", e))?;

    if resource_path.exists() {
        Ok(resource_path)
    } else {
        // Fallback for dev environment (running from src-tauri)
        let candidates = vec![
            PathBuf::from("resources/words.sqlite3"),
            PathBuf::from("src-tauri/resources/words.sqlite3"),
            PathBuf::from("../src-tauri/resources/words.sqlite3"),
            PathBuf::from("../../resources/words.sqlite3"),
            // During testing or different cwd
            PathBuf::from("words.sqlite3"),
        ];

        candidates.into_iter().find(|p| p.exists())
            .ok_or_else(|| format!("DB not found. Looked at {:?} and candidates", resource_path))
    }
}

#[tauri::command]
fn search_words(app: AppHandle, query: String) -> Result<Vec<Word>, String> {
    let db_path = resolve_db_path(&app)?;
    // println!("Opening DB at {:?}", db_path);
    db::search_words_in_db(&db_path, &query)
}

#[tauri::command]
fn get_word_definitions(app: AppHandle, word_id: i64) -> Result<Vec<Definition>, String> {
    let db_path = resolve_db_path(&app)?;
    db::get_word_definitions(&db_path, word_id)
}

#[tauri::command]
fn get_related_words(app: AppHandle, word_id: i64) -> Result<Vec<RelatedWord>, String> {
    let db_path = resolve_db_path(&app)?;
    db::get_related_words(&db_path, word_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            search_words,
            get_word_definitions,
            get_related_words
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
