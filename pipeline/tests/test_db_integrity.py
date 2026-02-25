import sqlite3
import pytest
import os

DB_PATH = "words.sqlite3"

def test_words_schema():
    assert os.path.exists(DB_PATH), "Database file not found"
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # Check words table schema
    cursor.execute("PRAGMA table_info(words)")
    columns = {row[1]: row[2] for row in cursor.fetchall()}
    assert "id" in columns
    assert "lemma" in columns
    assert "frequency_count" in columns
    assert "frequency_rank" in columns

    # Check fts table
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='words_fts'")
    assert cursor.fetchone() is not None

    conn.close()

def test_data_content():
    assert os.path.exists(DB_PATH), "Database file not found"
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # Check some common words exist and have counts
    cursor.execute("SELECT frequency_count FROM words WHERE lemma='the'")
    result = cursor.fetchone()
    assert result is not None
    assert result[0] > 100 # Should be very high

    # Check FTS search
    # Note: 'cat' might not be in the top 1000 lines if unlucky, but 'the' definitely is.
    # Let's search for 'be' which is rank 3.
    cursor.execute("SELECT rowid FROM words_fts WHERE words_fts MATCH 'be'")
    result = cursor.fetchall()
    assert len(result) > 0

    conn.close()
