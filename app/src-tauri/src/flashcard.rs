use crate::algorithm::{calculate_weight, update_learning_state};
use crate::user_db::{get_user_learning_state, save_user_learning_state, set_word_ignored};
use crate::{AppState, db::Word};
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordCard {
    pub word: Word,
    pub score_ema: f32,
    pub variance_ema: f32,
    pub last_reviewed_at: i64,
    pub review_count: i64,
    pub is_ignored: bool,
    pub calculated_weight: f32,
}

pub fn get_flashcard_deck(
    state: State<'_, AppState>,
    total_cards: u32,
    new_ratio: f32,
    active_tier_limit: Option<u32>,
) -> Result<Vec<WordCard>, String> {
    let words_conn = state.db.lock().map_err(|e| format!("Words DB lock failed: {}", e))?;
    let user_conn = state.user_db.lock().map_err(|e| format!("User DB lock failed: {}", e))?;
    let config = state.config.lock().map_err(|e| format!("Config lock failed: {}", e))?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    // Build query for words
    let tier_condition = match active_tier_limit {
        Some(limit) => format!("WHERE frequency_rank <= {}", limit),
        None => "".to_string(),
    };

    let sql = format!(
        "SELECT id, lemma, frequency_count, frequency_rank, surface_forms FROM words {}",
        tier_condition
    );

    let mut stmt = words_conn.prepare(&sql).map_err(|e| format!("Prepare failed: {}", e))?;
    let word_iter = stmt.query_map([], |row| {
        Ok(Word {
            id: row.get(0)?,
            lemma: row.get(1)?,
            frequency_count: row.get(2)?,
            frequency_rank: row.get(3)?,
            surface_forms: row.get(4)?,
        })
    }).map_err(|e| format!("Query failed: {}", e))?;

    let mut review_cards: Vec<WordCard> = Vec::new();
    let mut new_cards: Vec<WordCard> = Vec::new();

    for w_res in word_iter {
        let w = match w_res {
            Ok(w) => w,
            Err(_) => continue,
        };

        let user_state = get_user_learning_state(&user_conn, w.id).unwrap_or(None);
        let weight = calculate_weight(user_state.as_ref(), w.frequency_count, now, &config);

        let mut card = WordCard {
            word: w,
            score_ema: 0.0,
            variance_ema: 0.0,
            last_reviewed_at: 0,
            review_count: 0,
            is_ignored: false,
            calculated_weight: weight,
        };

        if let Some(s) = user_state {
            if s.is_ignored { continue; }
            card.score_ema = s.score_ema;
            card.variance_ema = s.variance_ema;
            card.last_reviewed_at = s.last_reviewed_at;
            card.review_count = s.review_count;
            card.is_ignored = s.is_ignored;

            if s.review_count > 0 {
                review_cards.push(card);
            } else {
                new_cards.push(card);
            }
        } else {
            new_cards.push(card);
        }
    }

    let review_count_target = ((total_cards as f32) * (1.0 - new_ratio)).round() as usize;
    let new_count_target = total_cards as usize - review_count_target;

    let mut final_deck = Vec::new();
    let mut rng_thread = rng();

    // Sample review cards
    if !review_cards.is_empty() && review_count_target > 0 {
        let weights: Vec<f32> = review_cards.iter().map(|c| c.calculated_weight.max(0.001)).collect();
        if let Ok(dist) = WeightedIndex::new(weights) {
            let mut sampled_indices: Vec<usize> = Vec::new();
            for _ in 0..review_count_target {
                let idx: usize = dist.sample(&mut rng_thread);
                if !sampled_indices.contains(&idx) { // Simple unique check, could be improved
                    sampled_indices.push(idx);
                }
            }
            for idx in sampled_indices {
                final_deck.push(review_cards[idx].clone());
            }
        }
    }

    // Sample new cards
    if !new_cards.is_empty() && new_count_target > 0 {
        let weights: Vec<f32> = new_cards.iter().map(|c| c.calculated_weight.max(0.001)).collect();
        if let Ok(dist) = WeightedIndex::new(weights) {
            let mut sampled_indices: Vec<usize> = Vec::new();
            for _ in 0..new_count_target {
                let idx: usize = dist.sample(&mut rng_thread);
                if !sampled_indices.contains(&idx) {
                    sampled_indices.push(idx);
                }
            }
            for idx in sampled_indices {
                final_deck.push(new_cards[idx].clone());
            }
        }
    }

    Ok(final_deck)
}

pub fn submit_card_answer(
    state: State<'_, AppState>,
    word_id: i64,
    score: u8,
) -> Result<(), String> {
    let user_conn = state.user_db.lock().map_err(|e| format!("User DB lock failed: {}", e))?;
    let config = state.config.lock().map_err(|e| format!("Config lock failed: {}", e))?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let current_state = get_user_learning_state(&user_conn, word_id).unwrap_or(None);

    let updated_state = update_learning_state(current_state.as_ref(), word_id, score, &config, now);

    save_user_learning_state(&user_conn, &updated_state)?;

    Ok(())
}

pub fn handle_set_word_ignored(
    state: State<'_, AppState>,
    word_id: i64,
    ignored: bool,
) -> Result<(), String> {
    let user_conn = state.user_db.lock().map_err(|e| format!("User DB lock failed: {}", e))?;
    set_word_ignored(&user_conn, word_id, ignored)?;
    Ok(())
}
