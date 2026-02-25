use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
pub struct Word {
    pub lemma: String,
    pub frequency_count: i64,
    pub frequency_rank: i64,
}

pub fn search_words_in_db(db_path: &Path, query: &str) -> Result<Vec<Word>, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB at {:?}: {}", db_path, e))?;

    let sql = if query.trim().is_empty() {
        "SELECT lemma, frequency_count, frequency_rank FROM words ORDER BY frequency_rank ASC LIMIT 50".to_string()
    } else {
        "SELECT lemma, frequency_count, frequency_rank FROM words WHERE lemma LIKE ?1 ORDER BY frequency_rank ASC LIMIT 50".to_string()
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {}", e))?;

    let mut rows = if query.trim().is_empty() {
        stmt.query([]).map_err(|e| format!("Query failed: {}", e))?
    } else {
        stmt.query([format!("{}%", query)]).map_err(|e| format!("Query failed: {}", e))?
    };

    let mut words = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        words.push(Word {
            lemma: row.get(0).unwrap_or_default(),
            frequency_count: row.get(1).unwrap_or_default(),
            frequency_rank: row.get(2).unwrap_or_default(),
        });
    }

    Ok(words)
}
