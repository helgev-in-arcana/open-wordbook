import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface Word {
  lemma: string;
  frequency_count: number;
  frequency_rank: number;
}

function App() {
  const [query, setQuery] = useState("");
  const [words, setWords] = useState<Word[]>([]);
  const [error, setError] = useState("");

  async function search(q: string) {
    try {
      // In Tauri v2, invoke arguments are passed as an object
      const result: Word[] = await invoke("search_words", { query: q });
      setWords(result);
      setError("");
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
              <tr key={word.lemma}>
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
    </main>
  );
}

export default App;
