use tauri::{AppHandle, Manager};
use std::path::PathBuf;

mod db;
use db::Word;

#[tauri::command]
fn search_words(app: AppHandle, query: String) -> Result<Vec<Word>, String> {
    // Attempt to resolve the path to the database
    let resource_path = app.path().resolve("resources/words.sqlite3", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve resource path: {}", e))?;

    let db_path = if resource_path.exists() {
        resource_path
    } else {
        // Fallback for dev environment
        let candidates = vec![
            PathBuf::from("resources/words.sqlite3"),
            PathBuf::from("src-tauri/resources/words.sqlite3"),
            PathBuf::from("../src-tauri/resources/words.sqlite3"),
            PathBuf::from("../../resources/words.sqlite3"),
        ];

        candidates.into_iter().find(|p| p.exists()).unwrap_or(resource_path)
    };

    println!("Attempting to open database at: {:?}", db_path);
    db::search_words_in_db(&db_path, &query)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![search_words])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
