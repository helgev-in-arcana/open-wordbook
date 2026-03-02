import { invoke } from '@tauri-apps/api/core';
import type { Word, Definition, RelatedWord, WordCard } from './types';

export async function searchWords(query: string): Promise<Word[]> {
  return await invoke('search_words', { query });
}

export async function getWordDefinitions(wordId: number): Promise<Definition[]> {
  return await invoke('get_word_definitions', { wordId });
}

export async function getRelatedWords(wordId: number): Promise<RelatedWord[]> {
  return await invoke('get_related_words', { wordId });
}

export async function getFlashcardDeck(
  totalCards: number,
  newRatio: number,
  tierMin: number | null,
  tierMax: number | null
): Promise<WordCard[]> {
  return await invoke('get_flashcard_deck', {
    totalCards,
    newRatio,
    tierMin,
    tierMax,
  });
}

export async function submitCardAnswer(wordId: number, score: number): Promise<void> {
  await invoke('submit_card_answer', { wordId, score });
}

export async function setWordIgnored(wordId: number, ignored: boolean): Promise<void> {
  await invoke('set_word_ignored', { wordId, ignored });
}
