use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
pub struct Word {
    pub id: i64,
    pub lemma: String,
    pub frequency_count: i64,
    pub frequency_rank: i64,
    pub surface_forms: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Definition {
    pub id: i64,
    pub word_id: i64,
    pub part_of_speech: String,
    pub meaning: String,
    pub source: String,
}

pub fn search_words_in_db(db_path: &Path, query: &str) -> Result<Vec<Word>, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB at {:?}: {}", db_path, e))?;

    let sql = if query.trim().is_empty() {
        "SELECT id, lemma, frequency_count, frequency_rank, surface_forms FROM words ORDER BY frequency_rank ASC LIMIT 50".to_string()
    } else {
        "SELECT id, lemma, frequency_count, frequency_rank, surface_forms FROM words WHERE lemma LIKE ?1 ORDER BY frequency_rank ASC LIMIT 50".to_string()
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
            id: row.get(0).unwrap_or_default(),
            lemma: row.get(1).unwrap_or_default(),
            frequency_count: row.get(2).unwrap_or_default(),
            frequency_rank: row.get(3).unwrap_or_default(),
            surface_forms: row.get(4).unwrap_or(None),
        });
    }

    Ok(words)
}

pub fn get_word_definitions(db_path: &Path, word_id: i64) -> Result<Vec<Definition>, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, word_id, part_of_speech, meaning, source FROM definitions WHERE word_id = ?1"
    ).map_err(|e| format!("Prepare failed: {}", e))?;

    let definitions_iter = stmt.query_map([word_id], |row| {
        Ok(Definition {
            id: row.get(0)?,
            word_id: row.get(1)?,
            part_of_speech: row.get(2)?,
            meaning: row.get(3)?,
            source: row.get(4)?,
        })
    }).map_err(|e| format!("Query failed: {}", e))?;

    let mut definitions = Vec::new();
    for def in definitions_iter {
        definitions.push(def.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(definitions)
}

#[cfg(test)]
mod tests {
    // Tests omitted to avoid dependency issues in this file update,
    // relying on integration tests or separate test file.
}
