use crate::algorithm::{calculate_weight, update_learning_state};
use crate::user_db::{get_user_learning_state, save_user_learning_state, set_word_ignored};
use crate::{db::Word, AppState};
use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use rand::rng;
use rand::Rng;
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
    tier_min: Option<u32>,
    tier_max: Option<u32>,
) -> Result<Vec<WordCard>, String> {
    let words_conn = state
        .db
        .lock()
        .map_err(|e| format!("Words DB lock failed: {}", e))?;
    let user_conn = state
        .user_db
        .lock()
        .map_err(|e| format!("User DB lock failed: {}", e))?;
    let config = state
        .config
        .lock()
        .map_err(|e| format!("Config lock failed: {}", e))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Build query for words
    let mut conditions = Vec::new();
    if let Some(min) = tier_min {
        conditions.push(format!("frequency_rank >= {}", min));
    }
    if let Some(max) = tier_max {
        conditions.push(format!("frequency_rank <= {}", max));
    }

    let tier_condition = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, lemma, frequency_count, frequency_rank, surface_forms FROM words {}",
        tier_condition
    );

    let mut stmt = words_conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {}", e))?;
    let word_iter = stmt
        .query_map([], |row| {
            Ok(Word {
                id: row.get(0)?,
                lemma: row.get(1)?,
                frequency_count: row.get(2)?,
                frequency_rank: row.get(3)?,
                surface_forms: row.get(4)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

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
            if s.is_ignored {
                continue;
            }
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

    // Sample cards without replacement
    sample_cards(
        &mut final_deck,
        &review_cards,
        review_count_target,
        &mut rng_thread,
    );
    sample_cards(
        &mut final_deck,
        &new_cards,
        new_count_target,
        &mut rng_thread,
    );

    Ok(final_deck)
}

/// Helper function to perform weighted random sampling without replacement
fn sample_cards(
    final_deck: &mut Vec<WordCard>,
    source_cards: &[WordCard],
    target_count: usize,
    rng: &mut impl Rng,
) {
    if source_cards.is_empty() || target_count == 0 {
        return;
    }

    let mut weights: Vec<f32> = source_cards
        .iter()
        .map(|c| c.calculated_weight.max(0.001))
        .collect();

    let count_to_sample = target_count.min(source_cards.len());

    for _ in 0..count_to_sample {
        if let Ok(dist) = WeightedIndex::new(&weights) {
            let idx: usize = dist.sample(rng);
            final_deck.push(source_cards[idx].clone());
            // Set weight to 0 to prevent picking the same card again
            weights[idx] = 0.0;
        } else {
            // Break if all weights become zero (or invalid)
            break;
        }
    }
}

pub fn submit_card_answer(
    state: State<'_, AppState>,
    word_id: i64,
    score: u8,
) -> Result<(), String> {
    let user_conn = state
        .user_db
        .lock()
        .map_err(|e| format!("User DB lock failed: {}", e))?;
    let config = state
        .config
        .lock()
        .map_err(|e| format!("Config lock failed: {}", e))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let current_state = get_user_learning_state(&user_conn, word_id).unwrap_or(None);

    let updated_state =
        update_learning_state(current_state.as_ref(), word_id, score, &config, now);

    save_user_learning_state(&user_conn, &updated_state)?;

    Ok(())
}

pub fn handle_set_word_ignored(
    state: State<'_, AppState>,
    word_id: i64,
    ignored: bool,
) -> Result<(), String> {
    let user_conn = state
        .user_db
        .lock()
        .map_err(|e| format!("User DB lock failed: {}", e))?;
    set_word_ignored(&user_conn, word_id, ignored)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_card(id: i64, weight: f32) -> WordCard {
        WordCard {
            word: Word {
                id,
                lemma: format!("word{}", id),
                frequency_count: 10,
                frequency_rank: 1,
                surface_forms: None,
            },
            score_ema: 0.0,
            variance_ema: 0.0,
            last_reviewed_at: 0,
            review_count: 0,
            is_ignored: false,
            calculated_weight: weight,
        }
    }

    #[test]
    fn test_sample_cards_basic() {
        let cards = vec![
            mock_card(1, 10.0),
            mock_card(2, 5.0),
            mock_card(3, 1.0),
        ];

        let mut deck = Vec::new();
        let mut rng = rand::rng();

        // Request 2 cards
        sample_cards(&mut deck, &cards, 2, &mut rng);
        assert_eq!(deck.len(), 2);

        // Ensure no duplicates were picked
        assert_ne!(deck[0].word.id, deck[1].word.id);
    }

    #[test]
    fn test_sample_cards_more_than_available() {
        let cards = vec![
            mock_card(1, 10.0),
            mock_card(2, 5.0),
        ];

        let mut deck = Vec::new();
        let mut rng = rand::rng();

        // Request 5 cards, but only 2 available
        sample_cards(&mut deck, &cards, 5, &mut rng);

        // Should clamp to 2
        assert_eq!(deck.len(), 2);
    }

    #[test]
    fn test_sample_cards_empty_or_zero() {
        let cards = vec![mock_card(1, 10.0)];
        let mut deck = Vec::new();
        let mut rng = rand::rng();

        // Zero requested
        sample_cards(&mut deck, &cards, 0, &mut rng);
        assert_eq!(deck.len(), 0);

        // Empty source
        sample_cards(&mut deck, &[], 2, &mut rng);
        assert_eq!(deck.len(), 0);
    }

    #[test]
    fn test_sample_cards_zero_weight_fallback() {
        let cards = vec![
            mock_card(1, 0.0), // Weights should be clamped to 0.001
            mock_card(2, 0.0),
        ];

        let mut deck = Vec::new();
        let mut rng = rand::rng();

        sample_cards(&mut deck, &cards, 2, &mut rng);
        assert_eq!(deck.len(), 2); // Should still pick them due to max(0.001) safeguard
    }
}
