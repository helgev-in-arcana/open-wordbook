import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import DefinitionPanel from "./components/DefinitionPanel";
import { FlashcardSession } from "./components/FlashcardSession";

interface Word {
  id: number;
  lemma: string;
  frequency_count: number;
  frequency_rank: number;
  surface_forms?: string;
}

function App() {
  const [query, setQuery] = useState("");
  const [words, setWords] = useState<Word[]>([]);
  const [error, setError] = useState("");
  const [selectedWord, setSelectedWord] = useState<Word | null>(null);
  const [showFlashcards, setShowFlashcards] = useState(false);
  const lastQueryRef = useRef<string | null>(null);

  async function search(q: string, signal?: AbortSignal): Promise<Word[]> {
    // Skip redundant search if the query is already strictly the last one fully handled
    if (q === lastQueryRef.current) return words;

    try {
      // Note: Tauri 'invoke' does not support cancellation.
      // The AbortSignal here prevents updating the UI state with stale results, 
      // but the backend request will still run to completion.
      const result: Word[] = await invoke("search_words", { query: q });
      if (signal?.aborted) return [];

      lastQueryRef.current = q;
      setWords(result);
      setError("");
      return result;
    } catch (e) {
      if (signal?.aborted) return [];
      console.error(e);
      setError(String(e));
      return [];
    }
  }

  // Handle clicking a related word -> search and select it
  async function handleSelectRelated(lemma: string) {
    // 1. Set query to the lemma (triggers useEffect, but lastQueryRef will skip duplicate backend call)
    setQuery(lemma);

    // 2. Search for it immediately
    const result = await search(lemma);

    // 3. If exact match found, select it
    const match = result.find(w => w.lemma === lemma);
    if (match) {
      setSelectedWord(match);
    }
  }

  useEffect(() => {
    const controller = new AbortController();
    // Initial load
    search("", controller.signal);
    return () => controller.abort();
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => {
      search(query, controller.signal);
    }, 300); // Debounce
    return () => {
      controller.abort();
      clearTimeout(timeoutId);
    };
  }, [query]);

  return (
    <main className="container">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1>Open Word Book</h1>
        <button
          onClick={() => setShowFlashcards(!showFlashcards)}
          style={{ padding: "0.5rem 1rem", cursor: "pointer", height: "fit-content" }}
        >
          {showFlashcards ? "Close Flashcards" : "Study Flashcards"}
        </button>
      </div>

      {showFlashcards ? (
        <div style={{ marginTop: "2rem" }}>
          <FlashcardSession />
        </div>
      ) : (
        <>
          <div className="search-box">
            <input
          id="search-input"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          placeholder="Type to search..."
          autoFocus
        />
      </div>

      {error && <p className="error">Error: {error}</p>}

      <div className="content-area">
        <div className="results-container">
          <table className="results-table">
            <thead>
              <tr>
                <th>Rank</th>
                <th>Lemma</th>
                <th>Count</th>
              </tr>
            </thead>
            <tbody>
              {words.map((word) => (
                <tr
                  key={word.id}
                  onClick={() => setSelectedWord(word)}
                  className={selectedWord?.id === word.id ? "selected-row" : ""}
                >
                  <td>{word.frequency_rank}</td>
                  <td className="lemma-cell">{word.lemma}</td>
                  <td>{word.frequency_count.toLocaleString()}</td>
                </tr>
              ))}
              {words.length === 0 && !error && (
                <tr>
                  <td colSpan={3} style={{ textAlign: "center" }}>No results found</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

          {selectedWord && (
            <div className="side-panel-container">
              <DefinitionPanel
                wordId={selectedWord.id}
                lemma={selectedWord.lemma}
                surfaceForms={selectedWord.surface_forms}
                onClose={() => setSelectedWord(null)}
                onSelectWord={handleSelectRelated}
              />
            </div>
          )}
        </div>
      </>
      )}
    </main>
  );
}

export default App;
