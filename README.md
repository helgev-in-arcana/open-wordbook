# Open Word Book (Agile Master)

**Open Word Book** is a fully offline, local-first English vocabulary application built with **Python**, **Rust**, **Tauri**, and **React**. It is built on the philosophy of **total data freedom**—leveraging completely open data (FineWeb-Edu corpus, JMdict dictionary) and modern AI techniques (Sentence Transformers, FAISS) to provide a rich, distraction-free learning environment. You are never bound by restrictive proprietary rights or locked into closed ecosystems.

## Key Features

*   **100% Open Data & Freedom:** Utilizing entirely open datasets and open-source models. No proprietary APIs, no subscriptions, and completely free from restrictive licensing. Your data, your rules.
*   **Offline-First:** All data is embedded. No internet required.
*   **High Performance:** Rust backend with SQLite for instant search.
*   **Smart Suggestions:** AI-powered semantic search (e.g., searching "cat" suggests "kitten", "feline").
*   **Rich Definitions:** Integrated JMdict/Wiktionary definitions and POS tags.
*   **Real Usage Stats:** Word frequency based on massive web corpora (FineWeb-Edu).
*   **Flashcard Learning:** Built-in spaced repetition system with EMA-based scoring and weighted sampling.

## Development Status (Phase 4 Complete)

The project follows an Agile/Incremental development model.

- [x] **Phase 1 (MVP):** Basic word frequency count and search.
- [x] **Phase 2 (Dictionary):** Integrated JMdict definitions and POS filtering.
- [x] **Phase 3 (MWE):** Multi-word expressions ("take off") and surface form analysis.
- [x] **Phase 4 (Vectors):** Pre-calculated semantic similarity network using AI embeddings.
- [x] **Phase 5 (Flashcards):** EMA-based spaced repetition with weighted random sampling.

## Getting Started

### Prerequisites

*   **Python 3.12+** (for data pipeline)
*   **Rust & Cargo** (for backend)
*   **Node.js & npm/yarn** (for frontend)

### 1. Build the Database (Pipeline)

Run the Python pipeline to generate `words.sqlite3`. This requires downloading data and models (approx. 2GB+ disk space for temporary files).

```bash
# Install dependencies
pip install -r pipeline/requirements.txt

# Download raw data (Corpus)
python pipeline/src/download_data.py

# Download dictionary (JMdict)
python pipeline/src/download_dict.py

# Build Core Database (Phase 1 & 3)
python pipeline/src/build_db.py

# Integrate Definitions (Phase 2)
python pipeline/src/build_phase2.py

# Calculate Similarity Vectors (Phase 4)
python pipeline/src/build_phase4.py
```

### 2. Run the Application

Once `words.sqlite3` is generated in the root directory (or `app/resources/`), run the Tauri app.

```bash
cd app
npm install
npm run tauri dev
```

## Project Structure

*   `pipeline/`: Python scripts for data processing.
    *   `src/`: Main logic (`build_db.py`, `build_phase4.py`, etc.).
    *   `tests/`: Unit tests for pipeline logic.
*   `app/`: Tauri application source code.
    *   `src-tauri/`: Rust backend.
    *   `src/`: React frontend.
*   `.agent/`: detailed documentation (ARCHITECTURE.md).

## License

MIT License (Code). 

All incorporated data is sourced from open and free datasets (e.g., CC-BY-SA for JMdict, permissive usage for FineWeb-Edu), strictly adhering to the principle of remaining unbound by restrictive, proprietary data rights.
