import { useState, useEffect, useRef, useMemo } from "react";
import { getFlashcardDeck, submitCardAnswer, setWordIgnored } from "../api";
import type { WordCard } from "../types";

export function FlashcardSession() {
  const [deckQueue, setDeckQueue] = useState<WordCard[]>([]);
  const [newRatio, setNewRatio] = useState(0.2);
  const [tierMin, setTierMin] = useState<number | "none">("none");
  const [tierMax, setTierMax] = useState<number | "none">(1000);
  const [sessionActive, setSessionActive] = useState(false);
  const [isFetching, setIsFetching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const BATCH_SIZE = 10;
  const isFetchingRef = useRef(false);

  const fetchMoreCards = async (currentQueue: WordCard[]) => {
    if (isFetchingRef.current) return;

    isFetchingRef.current = true;
    setIsFetching(true);
    setError(null);

    try {
      const parsedTierMin = tierMin === "none" ? null : Number(tierMin);
      const parsedTierMax = tierMax === "none" ? null : Number(tierMax);

      // Validate tier values
      if (parsedTierMin !== null && isNaN(parsedTierMin)) {
        setError("Tier Min must be a number or 'none'.");
        return;
      }
      if (parsedTierMax !== null && isNaN(parsedTierMax)) {
        setError("Tier Max must be a number or 'none'.");
        return;
      }

      const newCards = await getFlashcardDeck(
        BATCH_SIZE,
        newRatio,
        parsedTierMin,
        parsedTierMax
      );

      // Filter out cards that are already in the queue to avoid duplicates
      const uniqueNewCards = newCards.filter(
        (nc) => !currentQueue.some((qc) => qc.word.id === nc.word.id)
      );

      setDeckQueue((prev) => [...prev, ...uniqueNewCards]);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setIsFetching(false);
      isFetchingRef.current = false;
    }
  };

  // Triggers a fetch whenever queue gets low
  useEffect(() => {
    if (sessionActive && deckQueue.length < 3) {
      fetchMoreCards(deckQueue);
    }
  }, [deckQueue.length, sessionActive]);

  const startSession = async () => {
    setSessionActive(true);
    setDeckQueue([]); // Clear existing queue on restart
    await fetchMoreCards([]);
  };

  const handleAnswer = async (score: number) => {
    const currentCard = deckQueue[0];
    if (!currentCard) return;

    // Immediately pop card for snappy UI
    setDeckQueue((prev) => prev.slice(1));

    try {
      await submitCardAnswer(currentCard.word.id, score);
    } catch (e: any) {
      setError(`Failed to save answer: ${e.toString()}`);
    }
  };

  const handleIgnore = async () => {
    const currentCard = deckQueue[0];
    if (!currentCard) return;

    setDeckQueue((prev) => prev.slice(1));

    try {
      await setWordIgnored(currentCard.word.id, true);
    } catch (e: any) {
      setError(`Failed to ignore: ${e.toString()}`);
    }
  };

  const stopSession = () => {
    setSessionActive(false);
    setDeckQueue([]);
  };

  const currentCard = deckQueue[0];

  const surfaceFormsDisplay = useMemo(() => {
    const raw = currentCard?.word.surface_forms;
    if (!raw) return null;
    try {
      const map = JSON.parse(raw) as Record<string, number>;
      const items = Object.entries(map).sort((a, b) => b[1] - a[1]).slice(0, 5);
      if (items.length === 0) return null;
      return items.map(([form, count]) => `${form} (${count})`).join(", ");
    } catch {
      return null;
    }
  }, [currentCard?.word?.id]);

  if (!sessionActive) {
    return (
      <div style={{ padding: "1rem", border: "1px solid #ccc", borderRadius: "8px", maxWidth: "400px", margin: "auto" }}>
        <h2>Flashcard Setup</h2>
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
            Tier Min (Frequency Rank):
            <input
              type="text"
              value={tierMin}
              onChange={(e) =>
                setTierMin(
                  e.target.value === "none" ? "none" : Number(e.target.value)
                )
              }
              style={{ marginLeft: "10px", width: "80px" }}
              placeholder="e.g. 1 or none"
            />
          </label>
        </div>
        <div style={{ marginBottom: "1rem" }}>
          <label style={{ display: "block" }}>
            Tier Max (Frequency Rank):
            <input
              type="text"
              value={tierMax}
              onChange={(e) =>
                setTierMax(
                  e.target.value === "none" ? "none" : Number(e.target.value)
                )
              }
              style={{ marginLeft: "10px", width: "80px" }}
              placeholder="e.g. 1000 or none"
            />
          </label>
        </div>
        <button onClick={startSession} disabled={isFetching} style={{ padding: "0.5rem 1rem", cursor: "pointer" }}>
          {isFetching ? "Loading..." : "Start Endless Session"}
        </button>
        {error && <div style={{ color: "red", marginTop: "1rem" }}>Error: {error}</div>}
      </div>
    );
  }

  return (
    <div style={{ padding: "1rem", border: "1px solid #ccc", borderRadius: "8px", maxWidth: "400px", margin: "auto", textAlign: "center", position: "relative" }}>
      <button
        onClick={stopSession}
        style={{ position: "absolute", top: "1rem", right: "1rem", cursor: "pointer", background: "none", border: "none", fontSize: "1.2rem" }}
      >
        ×
      </button>

      <div style={{ fontSize: "0.8rem", color: "gray", textAlign: "left" }}>
        Queue: {deckQueue.length} {isFetching ? "(Fetching...)" : ""}
      </div>

      {!currentCard ? (
        <div style={{ padding: "3rem" }}>Loading next card...</div>
      ) : (
        <>
          <div style={{ margin: "2rem 0" }}>
            <h1 style={{ fontSize: "2.5rem", margin: 0 }}>{currentCard.word.lemma}</h1>
            {surfaceFormsDisplay && (
              <p style={{ color: "gray", fontSize: "0.9rem" }}>{surfaceFormsDisplay}</p>
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
        </>
      )}

      {error && <div style={{ color: "red", marginTop: "1rem", textAlign: "left" }}>Error: {error}</div>}
    </div>
  );
}
