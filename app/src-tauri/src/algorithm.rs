use crate::config::FlashcardConfig;
use crate::user_db::UserLearningState;

pub fn update_learning_state(
    current_state: Option<&UserLearningState>,
    word_id: i64,
    score: u8,
    config: &FlashcardConfig,
    current_timestamp: i64,
) -> UserLearningState {
    let alpha = config.alpha;
    let score_f32 = score as f32;

    if let Some(state) = current_state {
        let diff = score_f32 - state.score_ema;
        let new_score_ema = state.score_ema + alpha * diff;
        let new_variance_ema = (1.0 - alpha) * (state.variance_ema + alpha * diff.powi(2));
        let new_count = state.review_count + 1;

        UserLearningState {
            word_id,
            score_ema: new_score_ema,
            variance_ema: new_variance_ema,
            last_reviewed_at: current_timestamp,
            review_count: new_count,
            is_ignored: state.is_ignored,
        }
    } else {
        // Initial state
        UserLearningState {
            word_id,
            score_ema: score_f32,
            variance_ema: 0.0,
            last_reviewed_at: current_timestamp,
            review_count: 1,
            is_ignored: false,
        }
    }
}

pub fn calculate_weight(
    state: Option<&UserLearningState>,
    frequency_count: i64,
    current_timestamp: i64,
    config: &FlashcardConfig,
) -> f32 {
    let f_corpus = (frequency_count as f32).max(1.0);
    let log_f = f_corpus.log10().max(1.0); // Ensure log is at least 1.0 to avoid zero or negative weight

    match state {
        Some(s) if s.review_count > 0 && !s.is_ignored => {
            let w_diff = (2.0 - s.score_ema) * config.weight_mean;
            let w_var = s.variance_ema * config.weight_variance;
            let time_elapsed = (current_timestamp - s.last_reviewed_at).max(0) as f32;
            let w_time = time_elapsed * config.time_decay_factor;

            // To avoid negative weights, clamp to a small positive value
            let total_learning_weight = (w_diff + w_var + w_time).max(0.1);
            total_learning_weight * log_f
        }
        Some(s) if s.is_ignored => 0.0,
        _ => {
            // Unlearned or review_count == 0
            config.new_weight_initial_value * log_f
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_learning_state_initial() {
        let config = FlashcardConfig::default();
        let state = update_learning_state(None, 1, 2, &config, 1000);

        assert_eq!(state.word_id, 1);
        assert_eq!(state.score_ema, 2.0);
        assert_eq!(state.variance_ema, 0.0);
        assert_eq!(state.last_reviewed_at, 1000);
        assert_eq!(state.review_count, 1);
        assert!(!state.is_ignored);
    }

    #[test]
    fn test_update_learning_state_subsequent() {
        let config = FlashcardConfig {
            alpha: 0.5,
            ..FlashcardConfig::default()
        };

        let initial_state = UserLearningState {
            word_id: 1,
            score_ema: 2.0,
            variance_ema: 0.0,
            last_reviewed_at: 1000,
            review_count: 1,
            is_ignored: false,
        };

        // Score 0 (completely forgot)
        let state = update_learning_state(Some(&initial_state), 1, 0, &config, 2000);

        // new_score = 2.0 + 0.5 * (0.0 - 2.0) = 1.0
        assert_eq!(state.score_ema, 1.0);
        // new_variance = (1.0 - 0.5) * (0.0 + 0.5 * (-2.0)^2) = 0.5 * (2.0) = 1.0
        assert_eq!(state.variance_ema, 1.0);
        assert_eq!(state.review_count, 2);
        assert_eq!(state.last_reviewed_at, 2000);
    }

    #[test]
    fn test_calculate_weight_unlearned() {
        let config = FlashcardConfig::default(); // new_weight_initial_value = 3.0
                                                 // freq = 100 => log10(100) = 2.0
        let weight = calculate_weight(None, 100, 1000, &config);
        assert_eq!(weight, 3.0 * 2.0); // 6.0
    }

    #[test]
    fn test_calculate_weight_ignored() {
        let config = FlashcardConfig::default();
        let state = UserLearningState {
            word_id: 1,
            score_ema: 2.0,
            variance_ema: 0.0,
            last_reviewed_at: 1000,
            review_count: 1,
            is_ignored: true,
        };
        let weight = calculate_weight(Some(&state), 100, 2000, &config);
        assert_eq!(weight, 0.0);
    }

    #[test]
    fn test_calculate_weight_learned() {
        let config = FlashcardConfig {
            weight_mean: 1.0,
            weight_variance: 1.0,
            time_decay_factor: 0.001,
            ..FlashcardConfig::default()
        };

        let state = UserLearningState {
            word_id: 1,
            score_ema: 1.0,    // w_diff = (2.0 - 1.0) * 1.0 = 1.0
            variance_ema: 0.5, // w_var = 0.5 * 1.0 = 0.5
            last_reviewed_at: 1000,
            review_count: 1,
            is_ignored: false,
        };

        // w_time = (2000 - 1000) * 0.001 = 1.0
        // total = 1.0 + 0.5 + 1.0 = 2.5
        // log_f (freq 10) = 1.0
        // expected weight = 2.5 * 1.0 = 2.5
        let weight = calculate_weight(Some(&state), 10, 2000, &config);
        assert_eq!(weight, 2.5);
    }
}
