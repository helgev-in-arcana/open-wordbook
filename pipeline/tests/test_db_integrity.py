import sqlite3
import pytest
import os
import sys

sys.path.append(os.path.dirname(__file__))
from test_full_pipeline_dummy import test_full_pipeline_logic

DB_PATH = "test_words.sqlite3"

@pytest.fixture(autouse=True, scope="module")
def ensure_db():
    if not os.path.exists(DB_PATH):
        test_full_pipeline_logic()

def test_words_schema():
    assert os.path.exists(DB_PATH), "Database file not found."
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

    # 'the' is in dummy data
    cursor.execute("SELECT frequency_count FROM words WHERE lemma='the'")
    result = cursor.fetchone()
    assert result is not None
    assert result[0] >= 1

    # Check FTS search for 'cat' which is in dummy data
    cursor.execute("SELECT rowid FROM words_fts WHERE words_fts MATCH 'cat'")
    result = cursor.fetchall()
    assert len(result) > 0

    conn.close()
