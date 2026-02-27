import polars as pl
import spacy
from spacy.matcher import PhraseMatcher
from spacy.util import filter_spans
import sqlite3
import os
import sys
import json

# Ensure we can import from the same directory
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
try:
    from parse_dict import load_dictionary
except ImportError:
    # Allow running without dictionary logic if file missing (Phase 1 mode)
    load_dictionary = None

def build_database(input_parquet, output_db, dict_path=None):
    if not os.path.exists(input_parquet):
        print(f"Error: {input_parquet} not found. Run download_data.py first.")
        return

    # 1. Load MWEs if dictionary available
    mwes = []
    if dict_path and os.path.exists(dict_path) and load_dictionary:
        print("Loading dictionary for MWEs...")
        d = load_dictionary(dict_path)
        for k in d.keys():
            if " " in k:
                # Basic filter: ignore very long phrases?
                if len(k.split()) <= 4:
                    mwes.append(k)
        print(f"Extracted {len(mwes)} MWE candidates.")
    else:
        print("No dictionary provided or found. Skipping MWE extraction.")

    print(f"Loading data from {input_parquet}...")
    df = pl.read_parquet(input_parquet)
    texts = df["text"].to_list()

    print("Processing text with spaCy...")
    nlp = spacy.load("en_core_web_sm", disable=["ner", "parser"])

    # Setup Matcher
    matcher = None
    if mwes:
        print("Initializing PhraseMatcher...")
        matcher = PhraseMatcher(nlp.vocab)
        # Using nlp.make_doc for speed
        patterns = list(nlp.pipe(mwes)) # Process patterns
        matcher.add("MWE", patterns)

    lemmas = []
    surfaces = []

    doc_count = 0
    for doc in nlp.pipe(texts, batch_size=50):
        doc_count += 1
        if doc_count % 1000 == 0:
            print(f"Processed {doc_count} documents...", end="\r")

        # Apply Matcher
        if matcher:
            matches = matcher(doc)
            spans = [doc[start:end] for _, start, end in matches]
            spans = filter_spans(spans)
            with doc.retokenize() as retokenizer:
                for span in spans:
                    # Merge and set lemma
                    retokenizer.merge(span, attrs={"LEMMA": span.text.lower()})

        for token in doc:
            # Basic filtering: alpha or MWE (which might have space)
            # If merged, token.text has space. token.is_alpha is False.
            is_valid = token.is_alpha
            if not is_valid and " " in token.text:
                # Check if it looks like words (e.g. "take off")
                if all(part.isalpha() for part in token.text.split()):
                    is_valid = True

            if is_valid:
                lemmas.append(token.lemma_.lower())
                surfaces.append(token.text)

    print(f"\nExtracted {len(lemmas)} tokens.")

    if not lemmas:
        print("No lemmas extracted. Exiting.")
        return

    print("Counting frequencies...")
    df_tokens = pl.DataFrame({"lemma": lemmas, "surface": surfaces})

    # Aggregation
    # 1. Count (lemma, surface) -> count
    # 2. Group by lemma -> sum(count) as frequency_count, list of {surface, count}

    q = (
        df_tokens.lazy()
        .group_by(["lemma", "surface"])
        .len()
        .rename({"len": "count"})
        .group_by("lemma")
        .agg([
            pl.sum("count").alias("frequency_count"),
            pl.struct(["surface", "count"]).alias("surface_struct")
        ])
        .sort("frequency_count", descending=True)
        .with_row_index("frequency_rank", offset=1)
    )

    result_df = q.collect()

    print(f"Total unique words: {len(result_df)}")

    # Write to SQLite
    if os.path.exists(output_db):
        os.remove(output_db)

    conn = sqlite3.connect(output_db)
    cursor = conn.cursor()

    print("Creating tables...")
    cursor.execute("""
    CREATE TABLE words (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        lemma TEXT NOT NULL UNIQUE,
        frequency_count INTEGER NOT NULL,
        frequency_rank INTEGER NOT NULL,
        surface_forms JSON
    );
    """)

    cursor.execute("""
    CREATE VIRTUAL TABLE words_fts USING fts5(
        lemma,
        content='words',
        content_rowid='id',
        tokenize='unicode61 remove_diacritics 1'
    );
    """)

    print("Inserting data...")

    # Prepare data
    # Convert surface_struct to JSON string
    rows = []
    for row in result_df.iter_rows(named=True):
        lemma = row["lemma"]
        count = row["frequency_count"]
        rank = row["frequency_rank"]
        structs = row["surface_struct"] # list of dicts

        # Convert list of {surface, count} to dict {surface: count}
        # Sort by count desc
        structs.sort(key=lambda x: x["count"], reverse=True)
        surface_map = {item["surface"]: item["count"] for item in structs}
        surface_json = json.dumps(surface_map, ensure_ascii=False)

        rows.append((lemma, count, rank, surface_json))

    cursor.executemany("INSERT INTO words (lemma, frequency_count, frequency_rank, surface_forms) VALUES (?, ?, ?, ?)", rows)

    print("Building FTS index...")
    cursor.execute("INSERT INTO words_fts(rowid, lemma) SELECT id, lemma FROM words;")

    conn.commit()
    conn.close()
    print(f"Database created at {output_db}")

if __name__ == "__main__":
    input_file = os.path.join("data", "sample.parquet")
    output_file = "words.sqlite3"
    # Optional dictionary for MWEs
    dict_file = "data/JMdict_e.xml"
    if not os.path.exists(dict_file):
        dict_file = None

    build_database(input_file, output_file, dict_file)
