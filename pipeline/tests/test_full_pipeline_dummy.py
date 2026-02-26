import sys
import os
import polars as pl
import sqlite3
import pytest

# Ensure imports work
# Assumes running from pipeline directory
src_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "../src"))
if src_path not in sys.path:
    sys.path.append(src_path)

from build_db import build_database
from build_phase2 import build_phase2

def create_dummy_corpus(path):
    # Create simple text data
    data = {"text": ["The cat sat on the mat.", "The cat is happy.", "The dog runs."]}
    df = pl.DataFrame(data)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    df.write_parquet(path)

def create_dummy_dict(path):
    # Simple JMdict XML subset
    xml_content = """<?xml version="1.0" encoding="UTF-8"?>
<JMdict>
<entry>
<ent_seq>100</ent_seq>
<k_ele><keb>猫</keb></k_ele>
<r_ele><reb>ねこ</reb></r_ele>
<sense>
<pos>noun (common) (futsuumeishi)</pos>
<gloss>cat</gloss>
</sense>
</entry>
<entry>
<ent_seq>101</ent_seq>
<k_ele><keb>幸せ</keb></k_ele>
<r_ele><reb>しあわせ</reb></r_ele>
<sense>
<pos>adj</pos>
<gloss>happy</gloss>
</sense>
</entry>
</JMdict>
"""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(xml_content)

def test_full_pipeline_logic():
    dummy_parquet = "data/dummy.parquet"
    dummy_dict = "data/dummy_dict.xml"
    db_path = "words.sqlite3"

    # Cleanup before run
    if os.path.exists(db_path):
        os.remove(db_path)

    # 1. Create Data
    create_dummy_corpus(dummy_parquet)
    create_dummy_dict(dummy_dict)

    # 2. Phase 1: Build DB
    print("\n[Test] Running Phase 1 Build...")
    build_database(dummy_parquet, db_path)

    # Check Phase 1 results
    conn = sqlite3.connect(db_path)
    c = conn.cursor()
    c.execute("SELECT count(*) FROM words")
    count = c.fetchone()[0]
    print(f"[Test] Phase 1 words count: {count}")
    assert count > 0

    # Check 'cat' is there
    c.execute("SELECT frequency_count FROM words WHERE lemma='cat'")
    res = c.fetchone()
    assert res is not None, "cat not found in Phase 1"
    assert res[0] == 2, f"Expected count 2 for cat, got {res[0]}"

    # Insert garbage word with high rank (to test filtering)
    # rank > 50000. Let's say 60000.
    # Note: frequency_rank is populated by build_db.
    # With dummy data, ranks are 1-10. So 60000 is clearly outside.
    c.execute("INSERT INTO words (lemma, frequency_count, frequency_rank) VALUES ('garbage', 1, 60000)")
    conn.commit()
    conn.close()

    # 3. Phase 2: Dictionary Integration
    print("\n[Test] Running Phase 2 Build...")
    build_phase2(db_path, dummy_dict)

    # Check Phase 2 results
    conn = sqlite3.connect(db_path)
    c = conn.cursor()

    # 'cat' should be kept (in dict + high rank)
    c.execute("SELECT count(*) FROM words WHERE lemma='cat'")
    assert c.fetchone()[0] == 1, "cat deleted!"

    # 'garbage' should be deleted (rank > 50000 AND not in dict)
    c.execute("SELECT count(*) FROM words WHERE lemma='garbage'")
    assert c.fetchone()[0] == 0, "garbage not deleted!"

    # 'sit' (lemma of 'sat') should be kept because rank < 50000 (small corpus)
    c.execute("SELECT count(*) FROM words WHERE lemma='sit'")
    assert c.fetchone()[0] == 1, "sit deleted (should keep due to rank)"

    # Check definition for 'cat'
    c.execute("SELECT meaning, part_of_speech FROM definitions JOIN words ON definitions.word_id = words.id WHERE words.lemma='cat'")
    rows = c.fetchall()
    assert len(rows) > 0, "No definition for cat"
    print(f"[Test] Definition for cat: {rows[0]}")
    assert "猫" in rows[0][0] or "ねこ" in rows[0][0]
    # Verify POS is cleaned
    assert rows[0][1] == "Noun", f"POS not cleaned: {rows[0][1]}"

    conn.close()
    print("[Test] Logic Verification Passed!")

if __name__ == "__main__":
    test_full_pipeline_logic()
