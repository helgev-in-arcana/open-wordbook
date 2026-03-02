use rusqlite::Connection;
use std::path::Path;

pub fn init_user_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create user db dir: {}", e))?;
        }
    }

    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open user db: {}", e))?;
    setup_tables(&conn)?;
    Ok(conn)
}

pub fn setup_tables(conn: &Connection) -> Result<(), String> {
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
    Ok(())
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

use std::collections::HashMap;

pub fn get_all_user_learning_states(
    conn: &Connection,
) -> Result<HashMap<i64, UserLearningState>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT word_id, score_ema, variance_ema, last_reviewed_at, review_count, is_ignored
             FROM user_learning_states",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(UserLearningState {
                word_id: row.get(0)?,
                score_ema: row.get::<_, f64>(1)? as f32,
                variance_ema: row.get::<_, f64>(2)? as f32,
                last_reviewed_at: row.get(3)?,
                review_count: row.get(4)?,
                is_ignored: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut map = HashMap::new();
    for state_result in rows {
        let state = state_result.map_err(|e| format!("Row error: {}", e))?;
        map.insert(state.word_id, state);
    }

    Ok(map)
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
        .query_map([word_id], |row| {
            Ok(UserLearningState {
                word_id: row.get(0)?,
                score_ema: row.get::<_, f64>(1)? as f32,
                variance_ema: row.get::<_, f64>(2)? as f32,
                last_reviewed_at: row.get(3)?,
                review_count: row.get(4)?,
                is_ignored: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    match rows.next() {
        Some(Ok(state)) => Ok(Some(state)),
        Some(Err(e)) => Err(format!("Row error: {}", e)),
        None => Ok(None),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        setup_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn test_save_and_get_state() {
        let conn = setup_memory_db();
        let state = UserLearningState {
            word_id: 1,
            score_ema: 1.5,
            variance_ema: 0.2,
            last_reviewed_at: 1600000000,
            review_count: 5,
            is_ignored: false,
        };

        // Assert empty initially
        let fetched_none = get_user_learning_state(&conn, 1).unwrap();
        assert!(fetched_none.is_none());

        // Save and get
        save_user_learning_state(&conn, &state).unwrap();
        let fetched = get_user_learning_state(&conn, 1).unwrap().unwrap();
        assert_eq!(fetched, state);

        // Update existing state (Upsert)
        let updated_state = UserLearningState {
            word_id: 1,
            score_ema: 1.8,
            variance_ema: 0.1,
            last_reviewed_at: 1600001000,
            review_count: 6,
            is_ignored: false,
        };

        save_user_learning_state(&conn, &updated_state).unwrap();
        let fetched_updated = get_user_learning_state(&conn, 1).unwrap().unwrap();
        assert_eq!(fetched_updated, updated_state);
    }

    #[test]
    fn test_set_word_ignored() {
        let conn = setup_memory_db();

        // 1. Set unlearned word to ignored
        set_word_ignored(&conn, 2, true).unwrap();
        let state1 = get_user_learning_state(&conn, 2).unwrap().unwrap();
        assert!(state1.is_ignored);
        assert_eq!(state1.review_count, 0);

        // 2. Un-ignore it
        set_word_ignored(&conn, 2, false).unwrap();
        let state2 = get_user_learning_state(&conn, 2).unwrap().unwrap();
        assert!(!state2.is_ignored);

        // 3. Set existing learned word to ignored (should keep other stats but set ignored flag)
        let state = UserLearningState {
            word_id: 3,
            score_ema: 1.5,
            variance_ema: 0.2,
            last_reviewed_at: 1600000000,
            review_count: 5,
            is_ignored: false,
        };
        save_user_learning_state(&conn, &state).unwrap();

        set_word_ignored(&conn, 3, true).unwrap();
        let state3 = get_user_learning_state(&conn, 3).unwrap().unwrap();
        assert!(state3.is_ignored);
        assert_eq!(state3.review_count, 5); // Kept existing stats
    }
}
