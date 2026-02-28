import sqlite3
import os
import sys

# Ensure we can import from the same directory
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
from parse_dict import load_dictionary

def build_phase2(db_path, dict_path):
    if not os.path.exists(db_path):
        print(f"Error: {db_path} not found. Run build_db.py first.")
        return

    # Load dictionary
    print("Loading dictionary...")
    dictionary = load_dictionary(dict_path)
    if not dictionary:
        print("Dictionary empty or failed to load. Aborting phase 2.")
        return

    print("Updating database...")
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("PRAGMA foreign_keys = ON;")

    # Create definitions table
    cursor.execute("""
    CREATE TABLE IF NOT EXISTS definitions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        word_id INTEGER NOT NULL,
        part_of_speech TEXT NOT NULL,
        meaning TEXT NOT NULL,
        source TEXT NOT NULL,
        FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE
    );
    """)
    # Index
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_definitions_word_id ON definitions(word_id);")

    # Get all words
    cursor.execute("SELECT id, lemma, frequency_rank FROM words")
    words = cursor.fetchall() # list of (id, lemma, rank)

    print(f"Processing {len(words)} words...")

    words_to_delete = []
    definitions_to_insert = []

    count = 0
    kept_count = 0

    # Clear existing definitions if running repeatedly
    cursor.execute("DELETE FROM definitions")

    for word_id, lemma, rank in words:
        count += 1
        if count % 1000 == 0:
            print(f"Processed {count} words...", end="\r")

        # Check condition: Rank <= 50000 OR Exists in Dict
        lookup = lemma.lower()
        in_dict = lookup in dictionary
        keep = (rank <= 50000) or in_dict

        if not keep:
            words_to_delete.append(word_id)
        else:
            kept_count += 1
            if in_dict:
                entries = dictionary[lookup]
                for entry in entries:
                    definitions_to_insert.append((
                        word_id,
                        entry["pos"],
                        entry["meaning"],
                        entry["source"]
                    ))

    print(f"\nKept {kept_count} words. Deleting {len(words_to_delete)} words.")

    # Batch delete
    if words_to_delete:
        chunk_size = 900
        for i in range(0, len(words_to_delete), chunk_size):
            chunk = words_to_delete[i:i+chunk_size]
            placeholders = ",".join("?" * len(chunk))
            cursor.execute(f"DELETE FROM words WHERE id IN ({placeholders})", chunk)

    print(f"Inserting {len(definitions_to_insert)} definitions...")
    if definitions_to_insert:
        chunk_size = 1000
        for i in range(0, len(definitions_to_insert), chunk_size):
            chunk = definitions_to_insert[i:i+chunk_size]
            cursor.executemany(
                "INSERT INTO definitions (word_id, part_of_speech, meaning, source) VALUES (?, ?, ?, ?)",
                chunk
            )

    # Rebuild FTS index to reflect deletions
    print("Rebuilding FTS index...")
    cursor.execute("INSERT INTO words_fts(words_fts) VALUES('rebuild')")

    conn.commit()
    conn.close()
    print("Database update complete.")

if __name__ == "__main__":
    db_file = "words.sqlite3"
    dict_file = "data/JMdict_e.xml"
    build_phase2(db_file, dict_file)
