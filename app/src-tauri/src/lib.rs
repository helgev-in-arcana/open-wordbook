use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

mod db;
pub mod config;
pub mod user_db;
pub mod algorithm;
pub mod flashcard;

use db::{Definition, RelatedWord, Word};
use flashcard::WordCard;
use config::FlashcardConfig;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub user_db: Mutex<Connection>,
    pub config: Mutex<FlashcardConfig>,
}

fn resolve_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    // This is for resource bundling
    // "resources/words.sqlite3" in tauri.conf.json
    let resource_path = app
        .path()
        .resolve(
            "resources/words.sqlite3",
            tauri::path::BaseDirectory::Resource,
        )
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

        candidates
            .into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| format!("DB not found. Looked at {:?} and candidates", resource_path))
    }
}

#[tauri::command]
fn search_words(state: State<'_, AppState>, query: String) -> Result<Vec<Word>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("Mutex lock failed: {}", e))?;
    db::search_words_in_db(&conn, &query)
}

#[tauri::command]
fn get_word_definitions(
    state: State<'_, AppState>,
    word_id: i64,
) -> Result<Vec<Definition>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("Mutex lock failed: {}", e))?;
    db::get_word_definitions(&conn, word_id)
}

#[tauri::command]
fn get_related_words(state: State<'_, AppState>, word_id: i64) -> Result<Vec<RelatedWord>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("Mutex lock failed: {}", e))?;
    db::get_related_words(&conn, word_id)
}

#[tauri::command]
fn get_flashcard_deck(
    state: State<'_, AppState>,
    total_cards: u32,
    new_ratio: f32,
    active_tier_limit: Option<u32>,
) -> Result<Vec<WordCard>, String> {
    flashcard::get_flashcard_deck(state, total_cards, new_ratio, active_tier_limit)
}

#[tauri::command]
fn submit_card_answer(state: State<'_, AppState>, word_id: i64, score: u8) -> Result<(), String> {
    flashcard::submit_card_answer(state, word_id, score)
}

#[tauri::command]
fn set_word_ignored(state: State<'_, AppState>, word_id: i64, ignored: bool) -> Result<(), String> {
    flashcard::handle_set_word_ignored(state, word_id, ignored)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db_path = resolve_db_path(app.handle()).unwrap_or_else(|e| {
                eprintln!("Warning: failed to resolve db path: {}. Falling back to default 'words.sqlite3'.", e);
                PathBuf::from("words.sqlite3")
            });
            let conn = Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ).map_err(|e| format!("Failed to open DB: {}", e))?;

            let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            let user_db_path = app_data_dir.join("user.sqlite3");
            let user_conn = user_db::init_user_db(&user_db_path)?;

            let config = config::load_config(&app_data_dir)?;

            app.manage(AppState {
                db: Mutex::new(conn),
                user_db: Mutex::new(user_conn),
                config: Mutex::new(config),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_words,
            get_word_definitions,
            get_related_words,
            get_flashcard_deck,
            submit_card_answer,
            set_word_ignored
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
