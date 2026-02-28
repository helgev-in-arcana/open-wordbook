import sys
import os
import polars as pl
import sqlite3
import pytest
import json

# Ensure imports work
src_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "../src"))
if src_path not in sys.path:
    sys.path.append(src_path)

from build_db import build_database
from build_phase2 import build_phase2

def create_dummy_corpus(path):
    data = {
        "text": [
            "The cat sat on the mat.",
            "The cat is happy.",
            "The dog runs.",
            "The dog makes a mess.",
            "I give up on this.", # MWE usage
            "Please give me a break." # Normal usage
        ]
    }
    df = pl.DataFrame(data)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    df.write_parquet(path)

def create_dummy_dict(path):
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
<pos>Expressions (phrases, clauses, etc.)</pos>
<gloss>happy</gloss>
</sense>
</entry>
<entry>
<ent_seq>103</ent_seq>
<k_ele><keb>作る</keb></k_ele>
<sense>
<pos>Godan verb with 'u' ending</pos>
<pos>Transitive verb</pos>
<gloss>make</gloss>
</sense>
</entry>
<entry>
<ent_seq>200</ent_seq>
<k_ele><keb>諦める</keb></k_ele>
<sense>
<pos>v5r</pos>
<gloss>give up</gloss>
</sense>
</entry>
</JMdict>
"""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(xml_content)

def test_full_pipeline_logic():
    dummy_parquet = "data/dummy.parquet"
    dummy_dict = "data/dummy_dict.xml"
    db_path = "test_words.sqlite3"

    if os.path.exists(db_path):
        os.remove(db_path)

    create_dummy_corpus(dummy_parquet)
    create_dummy_dict(dummy_dict)

    # 2. Phase 1: Build DB (with MWE extraction enabled by passing dict)
    print("\n[Test] Running Phase 1 Build (MWE)...")
    build_database(dummy_parquet, db_path, dummy_dict)

    conn = sqlite3.connect(db_path)
    c = conn.cursor()

    # Check 'cat' count
    c.execute("SELECT frequency_count FROM words WHERE lemma='cat'")
    res = c.fetchone()
    assert res[0] == 2

    # Check MWE 'give up'
    c.execute("SELECT frequency_count, surface_forms FROM words WHERE lemma='give up'")
    res = c.fetchone()
    assert res is not None, "MWE 'give up' not found"
    assert res[0] == 1
    surfaces = json.loads(res[1])
    assert "give up" in surfaces
    print(f"[Test] MWE 'give up' found. Surfaces: {surfaces}")

    # Check 'give' count (standalone)
    # "Please give me a break." -> give
    c.execute("SELECT frequency_count FROM words WHERE lemma='give'")
    res = c.fetchone()
    assert res is not None
    assert res[0] == 1, f"Expected 'give' count 1, got {res[0]}"

    # 3. Phase 2: Dictionary Integration
    print("\n[Test] Running Phase 2 Build...")
    build_phase2(db_path, dummy_dict)

    # Verify 'make' POS (cleaned)
    c.execute("SELECT part_of_speech FROM definitions JOIN words ON definitions.word_id = words.id WHERE words.lemma='make'")
    res = c.fetchone()
    assert res[0] == "Verb (Godan), Vt"

    conn.close()
    print("[Test] Logic Verification Passed!")

if __name__ == "__main__":
    test_full_pipeline_logic()
