use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
pub struct Word {
    pub id: i64,
    pub lemma: String,
    pub frequency_count: i64,
    pub frequency_rank: i64,
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
        "SELECT id, lemma, frequency_count, frequency_rank FROM words ORDER BY frequency_rank ASC LIMIT 50".to_string()
    } else {
        "SELECT id, lemma, frequency_count, frequency_rank FROM words WHERE lemma LIKE ?1 ORDER BY frequency_rank ASC LIMIT 50".to_string()
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
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_search_and_definitions() {
        let db_path = PathBuf::from("resources/words.sqlite3");

        if !db_path.exists() {
             // Maybe running from crate root
             let p = PathBuf::from("src-tauri/resources/words.sqlite3");
             if p.exists() {
                 // Use that
             } else {
                 // Skip if not found (but expected to fail in CI if not set up)
                 println!("DB not found, skipping test");
                 return;
             }
        }

        // Search for 'cat'
        let results = search_words_in_db(&db_path, "cat").expect("Search failed");
        assert!(!results.is_empty());
        let cat = results.iter().find(|w| w.lemma == "cat").expect("cat not found");

        // Get definitions for 'cat'
        let defs = get_word_definitions(&db_path, cat.id).expect("Get definitions failed");
        assert!(!defs.is_empty());
        assert!(defs[0].meaning.contains("猫") || defs[0].meaning.contains("ねこ"));
    }
}
