export interface Word {
  id: number;
  lemma: string;
  frequency_count: number;
  frequency_rank: number;
  surface_forms: string | null;
}

export interface Definition {
  id: number;
  word_id: number;
  part_of_speech: string;
  meaning: string;
  source: string;
}

export interface RelatedWord {
  id: number;
  word_id: number;
  lemma: string;
  relation_type: string;
  score: number;
}

export interface WordCard {
  word: Word;
  score_ema: number;
  variance_ema: number;
  last_reviewed_at: number;
  review_count: number;
  is_ignored: boolean;
  calculated_weight: number;
}
