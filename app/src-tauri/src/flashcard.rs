use crate::algorithm::{calculate_weight, update_learning_state};
use crate::user_db::{
    get_all_user_learning_states, get_user_learning_state, save_user_learning_state,
    set_word_ignored, UserLearningState,
};
use crate::{db, db::Word, AppState};
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

/// Build a WordCard from a Word and optional user learning state.
fn build_word_card(word: Word, state: Option<&UserLearningState>, weight: f32) -> WordCard {
    if let Some(s) = state {
        WordCard {
            word,
            score_ema: s.score_ema,
            variance_ema: s.variance_ema,
            last_reviewed_at: s.last_reviewed_at,
            review_count: s.review_count,
            is_ignored: s.is_ignored,
            calculated_weight: weight,
        }
    } else {
        WordCard {
            word,
            score_ema: 0.0,
            variance_ema: 0.0,
            last_reviewed_at: 0,
            review_count: 0,
            is_ignored: false,
            calculated_weight: weight,
        }
    }
}

pub fn get_flashcard_deck(
    state: State<'_, AppState>,
    total_cards: u32,
    new_ratio: f32,
    tier_min: Option<u32>,
    tier_max: Option<u32>,
) -> Result<Vec<WordCard>, String> {
    // Lock order: when acquiring multiple locks, always use db -> user_db -> config.
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

    // User learning states are bounded by the number of words the user has interacted with
    let user_states_map = get_all_user_learning_states(&user_conn)?;

    let review_count_target = ((total_cards as f32) * (1.0 - new_ratio)).round() as usize;
    let new_count_target = total_cards as usize - review_count_target;

    let mut final_deck = Vec::new();
    let mut rng_thread = rng();

    // === Review Cards ===
    // Only fetch words that have been reviewed (small, bounded set)
    if review_count_target > 0 {
        let reviewed_ids: Vec<i64> = user_states_map
            .iter()
            .filter(|(_, s)| s.review_count > 0 && !s.is_ignored)
            .map(|(id, _)| *id)
            .collect();

        if !reviewed_ids.is_empty() {
            let review_words =
                db::fetch_words_by_ids(&words_conn, &reviewed_ids, tier_min, tier_max)?;
            let review_cards: Vec<WordCard> = review_words
                .into_iter()
                .map(|w| {
                    let user_state = user_states_map.get(&w.id);
                    let weight = calculate_weight(user_state, w.frequency_count, now, &config);
                    build_word_card(w, user_state, weight)
                })
                .collect();
            sample_cards(
                &mut final_deck,
                &review_cards,
                review_count_target,
                &mut rng_thread,
            );
        }
    }

    // === New Cards ===
    // Fetch a limited pool of unreviewed candidates ordered by frequency (most common first)
    if new_count_target > 0 {
        let exclude_ids: Vec<i64> = user_states_map
            .iter()
            .filter(|(_, s)| s.review_count > 0 || s.is_ignored)
            .map(|(id, _)| *id)
            .collect();

        // Pool multiplier: sample from a pool 10x the target to give weighted
        // random sampling enough diversity. Floor of 50 avoids degenerate tiny pools.
        const NEW_POOL_MULTIPLIER: usize = 10;
        const NEW_POOL_MIN: usize = 50;
        let pool_size = (new_count_target * NEW_POOL_MULTIPLIER).max(NEW_POOL_MIN);
        let new_words = db::fetch_new_word_candidates(
            &words_conn,
            &exclude_ids,
            tier_min,
            tier_max,
            pool_size,
        )?;

        let new_cards: Vec<WordCard> = new_words
            .into_iter()
            .map(|w| {
                let user_state = user_states_map.get(&w.id);
                let weight = calculate_weight(user_state, w.frequency_count, now, &config);
                build_word_card(w, user_state, weight)
            })
            .collect();
        sample_cards(
            &mut final_deck,
            &new_cards,
            new_count_target,
            &mut rng_thread,
        );
    }

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
    if score > 2 {
        return Err(format!("Invalid score: {}. Must be 0, 1, or 2.", score));
    }

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

    let updated_state = update_learning_state(current_state.as_ref(), word_id, score, &config, now);

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
        let cards = vec![mock_card(1, 10.0), mock_card(2, 5.0), mock_card(3, 1.0)];

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
        let cards = vec![mock_card(1, 10.0), mock_card(2, 5.0)];

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
