"""
Phase 4: Similarity Network Pre-calculation

This script calculates semantic similarity between words using vector embeddings.
It updates the `words.sqlite3` database with a `word_relations` table containing
the top-10 most similar words for each entry.

Process:
1. Load all lemmas from the `words` table.
2. Generate vector embeddings using a pre-trained Sentence Transformer model (default: all-MiniLM-L6-v2).
3. Normalize vectors to unit length.
4. Use FAISS (IndexFlatIP) to perform efficient Inner Product search (equivalent to Cosine Similarity on normalized vectors).
5. Store the top-10 neighbors (excluding self) in `word_relations`.

Requirements:
- words.sqlite3 must exist and be populated (Phase 1).
- sentence-transformers, faiss-cpu, torch, numpy
"""

import sqlite3
import os
import sys
import numpy as np
from sentence_transformers import SentenceTransformer
import faiss

# Ensure we can import from the same directory if needed
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

def build_phase4(db_path, model_name="all-MiniLM-L6-v2", batch_size=32):
    """
    Main function to execute Phase 4.

    Args:
        db_path (str): Path to the SQLite database.
        model_name (str): HuggingFace model name for embeddings.
        batch_size (int): Batch size for model inference.
    """
    if not os.path.exists(db_path):
        print(f"Error: {db_path} not found. Run previous build phases first.")
        return

    print(f"Loading model: {model_name} ...")
    # Force CPU usage for consistency and to avoid CUDA dependencies on client machines if we were distributing this
    model = SentenceTransformer(model_name, device="cpu")

    print("Connecting to database...")
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Create word_relations table
    # Stores many-to-many relationships with a score
    cursor.execute("""
    CREATE TABLE IF NOT EXISTS word_relations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        word_id_1 INTEGER NOT NULL,
        word_id_2 INTEGER NOT NULL,
        relation_type TEXT NOT NULL,
        score REAL,
        FOREIGN KEY(word_id_1) REFERENCES words(id) ON DELETE CASCADE,
        FOREIGN KEY(word_id_2) REFERENCES words(id) ON DELETE CASCADE
    );
    """)
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_relations_lookup ON word_relations(word_id_1, relation_type);")

    # Clear existing relations to allow re-running cleanly
    cursor.execute("DELETE FROM word_relations")

    # Fetch all lemmas
    print("Fetching lemmas...")
    cursor.execute("SELECT id, lemma FROM words")
    rows = cursor.fetchall() # list of (id, lemma)

    if not rows:
        print("No words found in database.")
        conn.close()
        return

    word_ids = [r[0] for r in rows]
    lemmas = [r[1] for r in rows]

    print(f"Encoding {len(lemmas)} words...")
    # Encode
    # show_progress_bar=True provides visual feedback during potentially long processing
    embeddings = model.encode(lemmas, batch_size=batch_size, show_progress_bar=True, convert_to_numpy=True)

    # Normalize for Cosine Similarity (Inner Product on normalized vectors)
    print("Normalizing vectors...")
    faiss.normalize_L2(embeddings)

    # Build FAISS Index
    print("Building FAISS index...")
    d = embeddings.shape[1]
    # IndexFlatIP uses Inner Product (Dot Product). Since vectors are normalized, this equals Cosine Similarity.
    index = faiss.IndexFlatIP(d)
    index.add(embeddings)

    # Search Top-K
    # k = 11 because the closest neighbor is always the word itself (score ~1.0)
    k = 11
    if len(lemmas) < k:
        k = len(lemmas)

    print(f"Searching Top-{k-1} neighbors...")
    D, I = index.search(embeddings, k)

    print("Storing relations...")
    relations_to_insert = []

    for i in range(len(word_ids)):
        source_id = word_ids[i]

        found_count = 0
        for j in range(k):
            target_idx = I[i][j]
            score = float(D[i][j])

            if target_idx == -1: continue
            # Check if self (by index, since order matches)
            if target_idx == i: continue

            target_id = word_ids[target_idx]

            relations_to_insert.append((
                source_id,
                target_id,
                'similar_top_k',
                score
            ))

            found_count += 1
            if found_count >= 10: break

    # Batch insert for performance
    if relations_to_insert:
        chunk_size = 5000
        total = len(relations_to_insert)
        print(f"Inserting {total} relations...")

        for i in range(0, total, chunk_size):
            chunk = relations_to_insert[i:i+chunk_size]
            cursor.executemany(
                "INSERT INTO word_relations (word_id_1, word_id_2, relation_type, score) VALUES (?, ?, ?, ?)",
                chunk
            )
            print(f"Inserted {min(i+chunk_size, total)} / {total}", end="\r")
    else:
        print("No relations found.")

    print("\nCommiting changes...")
    conn.commit()
    conn.close()
    print("Phase 4 complete.")

if __name__ == "__main__":
    db_file = "words.sqlite3"
    build_phase4(db_file)
