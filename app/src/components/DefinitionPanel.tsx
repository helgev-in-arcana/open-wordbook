import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./DefinitionPanel.css";

interface Definition {
  id: number;
  word_id: number;
  part_of_speech: string;
  meaning: string;
  source: string;
}

interface RelatedWord {
  id: number;
  word_id: number;
  lemma: string;
  relation_type: string;
  score: number;
}

interface DefinitionPanelProps {
  wordId: number;
  lemma: string;
  surfaceForms?: string;
  onClose: () => void;
  onSelectWord: (lemma: string) => void;
}

const POS_MAP: Record<string, string> = {
  "Noun": "名詞",
  "Verb": "動詞",
  "Vt": "他動詞",
  "Vi": "自動詞",
  "Adj": "形容詞",
  "Na": "形容動詞",
  "Adv": "副詞",
  "Pron": "代名詞",
  "Prep": "前置詞",
  "Conj": "接続詞",
  "Int": "間投詞",
  "Part": "助詞",
  "Aux": "助動詞",
  "Ctr": "助数詞",
  "Expr": "表現",
  "Num": "数詞",
  "Prefix": "接頭辞",
  "Suffix": "接尾辞",
  "Vs": "サ変接続",
  "Verb (Godan)": "五段動詞",
  "Verb (Ichidan)": "一段動詞",
};

function localizePos(posString: string): string {
  if (!posString) return "";

  return posString.split(",").map(p => {
    const trimmed = p.trim();
    return POS_MAP[trimmed] || trimmed;
  }).join("・");
}

function parseSurfaceForms(json: string): string {
  try {
    const map = JSON.parse(json) as Record<string, number>;
    // Sort by count descending
    const items = Object.entries(map).sort((a, b) => b[1] - a[1]);
    // Format: "form (count)"
    // Limit to top 5
    const topItems = items.slice(0, 5);
    if (topItems.length === 0) return "";

    return topItems.map(([form, count]) => `${form} (${count})`).join(", ");
  } catch (e) {
    console.error("Failed to parse surface forms", e);
    return "";
  }
}

export default function DefinitionPanel({ wordId, lemma, surfaceForms, onClose, onSelectWord }: DefinitionPanelProps) {
  const [definitions, setDefinitions] = useState<Definition[]>([]);
  const [relatedWords, setRelatedWords] = useState<RelatedWord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    setLoading(true);
    setError("");

    // Fetch definitions and related words in parallel
    Promise.all([
        invoke("get_word_definitions", { wordId }),
        invoke("get_related_words", { wordId })
    ])
      .then(([defs, related]) => {
        setDefinitions(defs as Definition[]);
        setRelatedWords(related as RelatedWord[]);
        setLoading(false);
      })
      .catch((e) => {
        setError(String(e));
        setLoading(false);
      });
  }, [wordId]);

  const formsDisplay = surfaceForms ? parseSurfaceForms(surfaceForms) : "";

  return (
    <div className="definition-panel">
      <div className="panel-header">
        <div>
            <h2>{lemma}</h2>
            {formsDisplay && (
                <div className="surface-forms">
                    <span className="surface-label">出現形:</span> {formsDisplay}
                </div>
            )}
        </div>
        <button onClick={onClose} className="close-btn" aria-label="Close">×</button>
      </div>

      {loading && <div className="loading">Loading details...</div>}
      {error && <div className="error">Error: {error}</div>}

      {!loading && !error && (
        <div className="panel-content">
            <div className="definitions-section">
              <h3>Definitions</h3>
              {definitions.length === 0 ? (
                <p>No definitions found.</p>
              ) : (
                <ul className="definition-list">
                  {definitions.map((def) => (
                    <li key={def.id} className="definition-item">
                      <div className="def-header">
                        <span className="pos-tag">{localizePos(def.part_of_speech)}</span>
                        <span className="source-tag">{def.source}</span>
                      </div>
                      <div className="meaning">{def.meaning}</div>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            {relatedWords.length > 0 && (
                <div className="related-section">
                    <h3>Related Words</h3>
                    <div className="related-chips">
                        {relatedWords.map((rw) => (
                            <button
                                key={rw.id}
                                className="related-chip"
                                onClick={() => onSelectWord(rw.lemma)}
                                title={`Score: ${rw.score.toFixed(3)}`}
                            >
                                {rw.lemma}
                            </button>
                        ))}
                    </div>
                </div>
            )}
        </div>
      )}
    </div>
  );
}
