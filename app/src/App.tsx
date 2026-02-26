import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import "./DefinitionPanel.css";
import DefinitionPanel from "./DefinitionPanel";

interface Word {
  id: number;
  lemma: string;
  frequency_count: number;
  frequency_rank: number;
}

function App() {
  const [query, setQuery] = useState("");
  const [words, setWords] = useState<Word[]>([]);
  const [error, setError] = useState("");
  const [selectedWord, setSelectedWord] = useState<Word | null>(null);

  async function search(q: string) {
    try {
      const result: Word[] = await invoke("search_words", { query: q });
      setWords(result);
      setError("");
      // Don't clear selectedWord automatically to allow browsing list while keeping detail open?
      // Or clear? Let's keep it if it's in the new list, otherwise clear.
      // But simpler to clear or keep. Let's keep.
    } catch (e) {
      console.error(e);
      setError(String(e));
    }
  }

  useEffect(() => {
    // Initial load
    search("");
  }, []);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      search(query);
    }, 300); // Debounce
    return () => clearTimeout(timeoutId);
  }, [query]);

  return (
    <main className="container">
      <h1>Open Word Book</h1>

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
              onClose={() => setSelectedWord(null)}
            />
          </div>
        )}
      </div>
    </main>
  );
}

export default App;
