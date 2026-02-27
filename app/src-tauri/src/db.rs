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

#[derive(Serialize, Debug, PartialEq)]
pub struct RelatedWord {
    pub id: i64,
    pub word_id: i64,
    pub lemma: String,
    pub relation_type: String,
    pub score: f64,
}

pub fn search_words_in_db(db_path: &Path, query: &str) -> Result<Vec<Word>, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB at {:?}: {}", db_path, e))?;

    let sql = if query.trim().is_empty() {
        "SELECT id, lemma, frequency_count, frequency_rank, surface_forms FROM words ORDER BY frequency_rank ASC LIMIT 50".to_string()
    } else {
        // Use parameterized query for safety
        "SELECT id, lemma, frequency_count, frequency_rank, surface_forms FROM words WHERE lemma LIKE ?1 ORDER BY frequency_rank ASC LIMIT 50".to_string()
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {}", e))?;

    let rows_result = if query.trim().is_empty() {
        stmt.query([])
    } else {
        stmt.query([format!("{}%", query)])
    };

    let mut rows = rows_result.map_err(|e| format!("Query failed: {}", e))?;

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
        // Unwrap result or return error string
        match def {
            Ok(d) => definitions.push(d),
            Err(e) => return Err(format!("Row error: {}", e)),
        }
    }

    Ok(definitions)
}

pub fn get_related_words(db_path: &Path, word_id: i64) -> Result<Vec<RelatedWord>, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB: {}", e))?;

    // Join with words table to get lemma of the related word
    let sql = "
        SELECT r.id, r.word_id_2, w.lemma, r.relation_type, r.score
        FROM word_relations r
        JOIN words w ON r.word_id_2 = w.id
        WHERE r.word_id_1 = ?1
        ORDER BY r.score DESC
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| format!("Prepare failed: {}", e))?;

    let rows = stmt.query_map([word_id], |row| {
        Ok(RelatedWord {
            id: row.get(0)?,
            word_id: row.get(1)?,
            lemma: row.get(2)?,
            relation_type: row.get(3)?,
            score: row.get(4)?,
        })
    }).map_err(|e| format!("Query failed: {}", e))?;

    let mut related = Vec::new();
    for r in rows {
        match r {
            Ok(rw) => related.push(rw),
            Err(e) => return Err(format!("Row error: {}", e)),
        }
    }

    Ok(related)
}
