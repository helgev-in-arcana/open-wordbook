import sys
import os
import sqlite3
import pytest
import numpy as np
from sentence_transformers import SentenceTransformer
import faiss

# Ensure we can import build_phase4
sys.path.append(os.path.join(os.path.dirname(__file__), "../src"))
from build_phase4 import build_phase4

DB_PATH = "test_phase4.sqlite3"

@pytest.fixture
def setup_db():
    if os.path.exists(DB_PATH):
        os.remove(DB_PATH)

    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("CREATE TABLE words (id INTEGER PRIMARY KEY, lemma TEXT);")

    # Insert dummy data: "cat", "kitten", "dog", "puppy", "car", "truck"
    # We expect cat-kitten, dog-puppy, car-truck to be closer
    words = [
        (1, "cat"),
        (2, "kitten"),
        (3, "dog"),
        (4, "puppy"),
        (5, "car"),
        (6, "truck"),
        (7, "apple") # Unrelated
    ]
    cursor.executemany("INSERT INTO words (id, lemma) VALUES (?, ?)", words)
    conn.commit()
    conn.close()

    yield DB_PATH

    if os.path.exists(DB_PATH):
        os.remove(DB_PATH)

def test_phase4_creates_relations(setup_db):
    # Use a tiny model for speed in tests
    # "paraphrase-MiniLM-L3-v2" is small enough, or "all-MiniLM-L6-v2"
    # If network is restricted, we might need a mock.
    # But since we installed sentence-transformers, it will try to download.
    # We assume internet access is available for the first run or cached.

    # Run Phase 4
    # Using a very small model just to verify the pipeline logic,
    # but since quality matters for "cat" vs "kitten", we use a standard small model.
    build_phase4(setup_db, model_name="all-MiniLM-L6-v2")

    conn = sqlite3.connect(setup_db)
    cursor = conn.cursor()

    # Check table existence
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='word_relations';")
    assert cursor.fetchone() is not None

    # Check relations for "cat" (id=1)
    cursor.execute("""
        SELECT w2.lemma, r.score
        FROM word_relations r
        JOIN words w2 ON r.word_id_2 = w2.id
        WHERE r.word_id_1 = 1
        ORDER BY r.score DESC
    """)
    relations = cursor.fetchall()

    assert len(relations) > 0
    top_match = relations[0][0]
    print(f"Top match for 'cat': {top_match}")

    # "kitten" or "dog" should be high. "car" should be low.
    # We can't strictly guarantee model performance without a fixed seed/model,
    # but "kitten" or "dog" is significantly semantically closer than "apple".

    related_words = [r[0] for r in relations]
    assert "kitten" in related_words or "dog" in related_words

    conn.close()
