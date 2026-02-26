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

interface DefinitionPanelProps {
  wordId: number;
  lemma: string;
  onClose: () => void;
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

export default function DefinitionPanel({ wordId, lemma, onClose }: DefinitionPanelProps) {
  const [definitions, setDefinitions] = useState<Definition[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    setLoading(true);
    setError("");
    invoke("get_word_definitions", { wordId })
      .then((res) => {
        setDefinitions(res as Definition[]);
        setLoading(false);
      })
      .catch((e) => {
        setError(String(e));
        setLoading(false);
      });
  }, [wordId]);

  return (
    <div className="definition-panel">
      <div className="panel-header">
        <h2>{lemma}</h2>
        <button onClick={onClose} className="close-btn" aria-label="Close">×</button>
      </div>

      {loading && <div className="loading">Loading definitions...</div>}
      {error && <div className="error">Error: {error}</div>}

      {!loading && !error && (
        <div className="definitions-list">
          {definitions.length === 0 ? (
            <p>No definitions found.</p>
          ) : (
            <ul>
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
      )}
    </div>
  );
}
