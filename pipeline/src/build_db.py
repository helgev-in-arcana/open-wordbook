import polars as pl
import spacy
import sqlite3
import os

def build_database(input_parquet, output_db):
    if not os.path.exists(input_parquet):
        print(f"Error: {input_parquet} not found. Run download_data.py first.")
        return

    print(f"Loading data from {input_parquet}...")
    df = pl.read_parquet(input_parquet)
    texts = df["text"].to_list()

    print("Processing text with spaCy...")
    # Disable NER and Parser for speed, keep lemmatizer
    nlp = spacy.load("en_core_web_sm", disable=["ner", "parser"])

    lemmas = []
    # Process in batches
    # Use nlp.pipe for efficiency
    doc_count = 0
    for doc in nlp.pipe(texts, batch_size=50):
        doc_count += 1
        if doc_count % 1000 == 0:
            print(f"Processed {doc_count} documents...")

        for token in doc:
            # Basic filtering: only alphabetic tokens for now
            if token.is_alpha:
                lemmas.append(token.lemma_.lower())

    print(f"Extracted {len(lemmas)} lemmas.")

    if not lemmas:
        print("No lemmas extracted. Exiting.")
        return

    print("Counting frequencies...")
    lemma_df = pl.DataFrame({"lemma": lemmas})
    # Count frequencies
    counts = lemma_df.group_by("lemma").len().rename({"len": "frequency_count"})

    print("Calculating ranks...")
    # Sort by count descending and add rank
    counts = counts.sort("frequency_count", descending=True).with_row_index("frequency_rank", offset=1)

    print(f"Total unique words: {len(counts)}")
    print(counts.head())

    # Write to SQLite
    if os.path.exists(output_db):
        os.remove(output_db)

    conn = sqlite3.connect(output_db)
    cursor = conn.cursor()

    print("Creating tables...")
    # Create tables
    cursor.execute("""
    CREATE TABLE words (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        lemma TEXT NOT NULL UNIQUE,
        frequency_count INTEGER NOT NULL,
        frequency_rank INTEGER NOT NULL
    );
    """)

    # FTS table linked to 'words' table
    cursor.execute("""
    CREATE VIRTUAL TABLE words_fts USING fts5(
        lemma,
        content='words',
        content_rowid='id',
        tokenize='unicode61 remove_diacritics 1'
    );
    """)

    print("Inserting data...")
    # Polars to list of tuples
    rows = counts.select(["lemma", "frequency_count", "frequency_rank"]).rows()
    cursor.executemany("INSERT INTO words (lemma, frequency_count, frequency_rank) VALUES (?, ?, ?)", rows)

    print("Building FTS index...")
    # Populate FTS index
    cursor.execute("INSERT INTO words_fts(rowid, lemma) SELECT id, lemma FROM words;")

    conn.commit()
    conn.close()
    print(f"Database created at {output_db}")

if __name__ == "__main__":
    # Assuming running from pipeline directory
    input_file = os.path.join("data", "sample.parquet")
    output_file = "words.sqlite3"
    build_database(input_file, output_file)
