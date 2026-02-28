import { useState } from "react";
import { getFlashcardDeck, submitCardAnswer, setWordIgnored } from "../api";
import type { WordCard } from "../types";

export function FlashcardSession() {
  const [deck, setDeck] = useState<WordCard[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [totalCards, setTotalCards] = useState(10);
  const [newRatio, setNewRatio] = useState(0.2);
  const [activeTierLimit, setActiveTierLimit] = useState<number | "none">(1000);
  const [sessionActive, setSessionActive] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startSession = async () => {
    setLoading(true);
    setError(null);
    try {
      const tierLimit = activeTierLimit === "none" ? null : activeTierLimit;
      const fetchedDeck = await getFlashcardDeck(
        totalCards,
        newRatio,
        tierLimit
      );
      setDeck(fetchedDeck);
      setCurrentIndex(0);
      setSessionActive(true);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleAnswer = async (score: number) => {
    const currentCard = deck[currentIndex];
    if (!currentCard) return;

    try {
      await submitCardAnswer(currentCard.word.id, score);
      moveToNextCard();
    } catch (e: any) {
      setError(e.toString());
    }
  };

  const handleIgnore = async () => {
    const currentCard = deck[currentIndex];
    if (!currentCard) return;

    try {
      await setWordIgnored(currentCard.word.id, true);
      moveToNextCard();
    } catch (e: any) {
      setError(e.toString());
    }
  };

  const moveToNextCard = () => {
    if (currentIndex < deck.length - 1) {
      setCurrentIndex((prev) => prev + 1);
    } else {
      // Session Complete
      setSessionActive(false);
      setDeck([]);
      setCurrentIndex(0);
      alert("Flashcard session complete!");
    }
  };

  if (!sessionActive) {
    return (
      <div style={{ padding: "1rem", border: "1px solid #ccc", borderRadius: "8px", maxWidth: "400px", margin: "auto" }}>
        <h2>Flashcard Setup</h2>
        <div style={{ marginBottom: "1rem" }}>
          <label style={{ display: "block" }}>
            Total Cards:
            <input
              type="number"
              value={totalCards}
              onChange={(e) => setTotalCards(Number(e.target.value))}
              style={{ marginLeft: "10px", width: "60px" }}
            />
          </label>
        </div>
        <div style={{ marginBottom: "1rem" }}>
          <label style={{ display: "block" }}>
            New Cards Ratio (0.0 - 1.0):
            <input
              type="number"
              step="0.1"
              value={newRatio}
              onChange={(e) => setNewRatio(Number(e.target.value))}
              style={{ marginLeft: "10px", width: "60px" }}
            />
          </label>
        </div>
        <div style={{ marginBottom: "1rem" }}>
          <label style={{ display: "block" }}>
            Tier Limit (Frequency Rank):
            <input
              type="text"
              value={activeTierLimit}
              onChange={(e) =>
                setActiveTierLimit(
                  e.target.value === "none" ? "none" : Number(e.target.value)
                )
              }
              style={{ marginLeft: "10px", width: "80px" }}
              placeholder="e.g. 1000 or none"
            />
          </label>
        </div>
        <button onClick={startSession} disabled={loading} style={{ padding: "0.5rem 1rem", cursor: "pointer" }}>
          {loading ? "Loading Deck..." : "Start Session"}
        </button>
        {error && <div style={{ color: "red", marginTop: "1rem" }}>Error: {error}</div>}
      </div>
    );
  }

  const currentCard = deck[currentIndex];

  if (!currentCard) {
    return <div>Deck is empty or invalid state.</div>;
  }

  return (
    <div style={{ padding: "1rem", border: "1px solid #ccc", borderRadius: "8px", maxWidth: "400px", margin: "auto", textAlign: "center" }}>
      <h2>
        Card {currentIndex + 1} / {deck.length}
      </h2>
      <div style={{ margin: "2rem 0" }}>
        <h1 style={{ fontSize: "2.5rem", margin: 0 }}>{currentCard.word.lemma}</h1>
        {currentCard.word.surface_forms && (
          <p style={{ color: "gray" }}>{currentCard.word.surface_forms}</p>
        )}
      </div>

      <div style={{ background: "#f9f9f9", padding: "1rem", borderRadius: "4px", marginBottom: "2rem", fontSize: "0.85rem", textAlign: "left" }}>
        <p><strong>Debug Info:</strong></p>
        <p>Rank: {currentCard.word.frequency_rank} | Freq: {currentCard.word.frequency_count}</p>
        <p>Review Count: {currentCard.review_count}</p>
        <p>Score (EMA): {currentCard.score_ema.toFixed(3)}</p>
        <p>Variance (EMA): {currentCard.variance_ema.toFixed(3)}</p>
        <p>Weight: {currentCard.calculated_weight.toFixed(3)}</p>
        <p>New/Review: {currentCard.review_count === 0 ? "NEW" : "REVIEW"}</p>
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", gap: "10px" }}>
        <button onClick={() => handleAnswer(0)} style={{ flex: 1, padding: "0.5rem", background: "#ffcccc", cursor: "pointer" }}>
          0 - Forgot
        </button>
        <button onClick={() => handleAnswer(1)} style={{ flex: 1, padding: "0.5rem", background: "#ffffcc", cursor: "pointer" }}>
          1 - Hard
        </button>
        <button onClick={() => handleAnswer(2)} style={{ flex: 1, padding: "0.5rem", background: "#ccffcc", cursor: "pointer" }}>
          2 - Easy
        </button>
      </div>

      <div style={{ marginTop: "1rem" }}>
         <button onClick={handleIgnore} style={{ padding: "0.5rem 1rem", cursor: "pointer", background: "#eee", color: "#555" }}>
          Ignore Word
        </button>
      </div>

      {error && <div style={{ color: "red", marginTop: "1rem", textAlign: "left" }}>Error: {error}</div>}
    </div>
  );
}
