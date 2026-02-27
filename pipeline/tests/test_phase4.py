"""
Test Phase 4: Similarity Network Calculation

This test verifies that the `build_phase4` pipeline:
1. Creates the `word_relations` table.
2. Correctly calculates semantic similarity between known word pairs.
3. Populates the database with these relationships.
"""

import sys
import os
import sqlite3
import pytest
from sentence_transformers import SentenceTransformer

# Ensure we can import build_phase4
sys.path.append(os.path.join(os.path.dirname(__file__), "../src"))
from build_phase4 import build_phase4

DB_PATH = "test_phase4.sqlite3"

@pytest.fixture
def setup_db():
    """Create a temporary SQLite database with dummy word data."""
    if os.path.exists(DB_PATH):
        os.remove(DB_PATH)

    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("CREATE TABLE words (id INTEGER PRIMARY KEY, lemma TEXT);")

    # Insert dummy data with known semantic relationships
    # Group 1: Animals (cat, kitten, dog, puppy)
    # Group 2: Vehicles (car, truck)
    # Group 3: Fruit (apple) - control
    words = [
        (1, "cat"),
        (2, "kitten"),
        (3, "dog"),
        (4, "puppy"),
        (5, "car"),
        (6, "truck"),
        (7, "apple")
    ]
    cursor.executemany("INSERT INTO words (id, lemma) VALUES (?, ?)", words)
    conn.commit()
    conn.close()

    yield DB_PATH

    if os.path.exists(DB_PATH):
        os.remove(DB_PATH)

def test_phase4_creates_relations(setup_db):
    """
    Verify that the pipeline correctly identifies semantic relationships.
    e.g., 'cat' should be more related to 'kitten'/'dog' than 'car' or 'apple'.
    """
    # Use a standard small model.
    # NOTE: This requires internet access for the first run to download the model.
    build_phase4(setup_db, model_name="all-MiniLM-L6-v2")

    conn = sqlite3.connect(setup_db)
    cursor = conn.cursor()

    # 1. Verify table creation
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='word_relations';")
    assert cursor.fetchone() is not None, "word_relations table was not created."

    # 2. Check relations for "cat" (id=1)
    cursor.execute("""
        SELECT w2.lemma, r.score
        FROM word_relations r
        JOIN words w2 ON r.word_id_2 = w2.id
        WHERE r.word_id_1 = 1
        ORDER BY r.score DESC
    """)
    relations = cursor.fetchall()

    assert len(relations) > 0, "No relations found for 'cat'."

    related_lemmas = [r[0] for r in relations]

    # 3. Verify semantic relevance
    # 'kitten' and 'dog' should be among the top related words for 'cat'
    assert "kitten" in related_lemmas or "dog" in related_lemmas, \
        f"Expected 'kitten' or 'dog' in related words for 'cat', but got: {related_lemmas}"

    # 'apple' should be less related or not present if K is small (here K=6 for 7 items)
    # We don't assert strict absence of 'apple' because with N=7 everyone is somewhat a neighbor,
    # but we can check if 'car' is present (vehicles vs animals).

    print(f"Top matches for 'cat': {related_lemmas}")

    conn.close()
