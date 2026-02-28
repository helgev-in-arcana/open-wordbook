use rusqlite::Connection;
use std::path::Path;

pub fn init_user_db(db_path: &Path) -> Result<Connection, String> {
    let parent = db_path.parent().unwrap();
    if !parent.exists() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create user db dir: {}", e))?;
    }

    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open user db: {}", e))?;

    // Create learning states table
    let init_sql = "
        CREATE TABLE IF NOT EXISTS user_learning_states (
            word_id INTEGER PRIMARY KEY,
            score_ema REAL NOT NULL,
            variance_ema REAL NOT NULL,
            last_reviewed_at INTEGER NOT NULL,
            review_count INTEGER NOT NULL DEFAULT 0,
            is_ignored BOOLEAN NOT NULL DEFAULT 0
        );
    ";

    conn.execute(init_sql, [])
        .map_err(|e| format!("Failed to init user db table: {}", e))?;

    Ok(conn)
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserLearningState {
    pub word_id: i64,
    pub score_ema: f32,
    pub variance_ema: f32,
    pub last_reviewed_at: i64,
    pub review_count: i64,
    pub is_ignored: bool,
}

pub fn get_user_learning_state(
    conn: &Connection,
    word_id: i64,
) -> Result<Option<UserLearningState>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT word_id, score_ema, variance_ema, last_reviewed_at, review_count, is_ignored
             FROM user_learning_states WHERE word_id = ?1",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let mut rows = stmt
        .query([word_id])
        .map_err(|e| format!("Query failed: {}", e))?;

    if let Some(row) = rows.next().map_err(|e| format!("Row iteration failed: {}", e))? {
        let state = UserLearningState {
            word_id: row.get(0).unwrap(),
            score_ema: row.get::<_, f64>(1).unwrap() as f32,
            variance_ema: row.get::<_, f64>(2).unwrap() as f32,
            last_reviewed_at: row.get(3).unwrap(),
            review_count: row.get(4).unwrap(),
            is_ignored: row.get(5).unwrap(),
        };
        Ok(Some(state))
    } else {
        Ok(None)
    }
}

pub fn save_user_learning_state(
    conn: &Connection,
    state: &UserLearningState,
) -> Result<(), String> {
    let sql = "
        INSERT INTO user_learning_states (word_id, score_ema, variance_ema, last_reviewed_at, review_count, is_ignored)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(word_id) DO UPDATE SET
            score_ema = excluded.score_ema,
            variance_ema = excluded.variance_ema,
            last_reviewed_at = excluded.last_reviewed_at,
            review_count = excluded.review_count,
            is_ignored = excluded.is_ignored;
    ";

    conn.execute(
        sql,
        rusqlite::params![
            state.word_id,
            state.score_ema,
            state.variance_ema,
            state.last_reviewed_at,
            state.review_count,
            state.is_ignored,
        ],
    )
    .map_err(|e| format!("Upsert failed: {}", e))?;

    Ok(())
}

pub fn set_word_ignored(conn: &Connection, word_id: i64, ignored: bool) -> Result<(), String> {
    let sql = "
        INSERT INTO user_learning_states (word_id, score_ema, variance_ema, last_reviewed_at, review_count, is_ignored)
        VALUES (?1, 0.0, 0.0, 0, 0, ?2)
        ON CONFLICT(word_id) DO UPDATE SET
            is_ignored = excluded.is_ignored;
    ";

    conn.execute(sql, rusqlite::params![word_id, ignored])
        .map_err(|e| format!("Set ignored failed: {}", e))?;

    Ok(())
}
