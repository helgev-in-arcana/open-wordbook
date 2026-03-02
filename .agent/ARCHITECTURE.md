# Architecture Documentation: Open Word Book

This document provides a comprehensive overview of the system architecture for the "Open Word Book" project, a fully offline, local English vocabulary application powered by open data and AI.

## 1. Core Principles

*   **Offline-First:** No runtime dependencies on external APIs (Google Translate, OpenAI, etc.). All data is pre-calculated and embedded.
*   **Separation of Concerns:**
    *   **Data Pipeline (Python):** Handles heavy lifting (NLP, Vectorization, Database Construction).
    *   **Application (Rust/Tauri/React):** Lightweight, read-only viewer.
*   **Agile/Incremental Development:** Features are built in distinct phases (MVP -> Dictionary -> MWE -> Vectors -> Flashcards).

## 2. System Overview

### 2.1. Data Pipeline (Python)

This component is responsible for transforming raw open data into a structured SQLite database (`words.sqlite3`). It runs on the developer's machine (or CI/CD), not on the user's device.

**Key Technologies:**
*   **Python 3.12+**
*   **Polars:** High-performance DataFrame library for aggregating corpus statistics.
*   **spaCy:** Natural Language Processing (Tokenization, Lemmatization, Phrase Matching).
*   **Sentence Transformers:** Generating vector embeddings for semantic search.
*   **FAISS:** Efficient similarity search for pre-calculating word relationships.

**Workflow:**
1.  **Corpus Processing (Phase 1):** Reads raw text (e.g., FineWeb-Edu), tokenizes, lemmatizes, and counts frequencies. Output: `words` table with `lemma` and `frequency_rank`.
2.  **Dictionary Integration (Phase 2):** Parses JMdict (XML) to extract definitions and POS tags. Filters words based on rank (<50k) or dictionary existence. Output: `definitions` table.
3.  **Multi-Word Expressions (Phase 3):** Uses `spaCy PhraseMatcher` to identify idioms (e.g., "take off") as single tokens. Aggregates original surface forms (e.g., "Cat", "cat") into JSON.
4.  **Vector Similarity (Phase 4):**
    *   Generates embeddings for all lemmas using `all-MiniLM-L6-v2`.
    *   Calculates Cosine Similarity using FAISS.
    *   Stores top-10 neighbors in `word_relations` table.

### 2.2. Client Application (Tauri)

A cross-platform desktop application that consumes the `words.sqlite3` database.

**Key Technologies:**
*   **Tauri:** Application framework (Rust + WebView).
*   **Rust (Backend):** Handles file I/O and SQLite queries via `rusqlite`.
*   **React + TypeScript (Frontend):** UI logic and state management.

**Architecture:**
*   **Database Access:** The Rust backend opens `words.sqlite3` in **Read-Only** mode. It exposes high-level commands (e.g., `search_words`, `get_word_definitions`, `get_related_words`) to the frontend.
*   **Frontend:**
    *   **Search Box:** Real-time incremental search (debounced).
    *   **Word List:** Virtualized or paginated list of results.
    *   **Definition Panel:** Displays definitions, POS tags, surface forms, and related words.
    *   **Navigation:** Clicking a related word triggers a new search.
    *   **Flashcard Mode:** EMA-based spaced repetition with weighted random sampling.

### 2.3. User Data (Flashcard System)

User learning progress is stored in a separate writable database (`user.sqlite3`) in the app data directory, keeping the vocabulary database (`words.sqlite3`) read-only.

**Key Components:**
*   **`user_db.rs`:** Manages `user.sqlite3` (create, read, upsert learning states).
*   **`algorithm.rs`:** EMA-based state updates and weight calculation (see `FLASHCARD_ALGORITHM.md`).
*   **`flashcard.rs`:** Deck generation with weighted random sampling, partitioned into review/new card pools.
*   **`config.rs`:** Persisted hyperparameters (alpha, weight coefficients, etc.) in `config.json`.

## 3. Database Schema (`words.sqlite3`)

The database is the single source of truth for the application.

### `words` Table
Stores the core vocabulary list.

| Column | Type | Description |
| :--- | :--- | :--- |
| `id` | INTEGER PK | Unique ID |
| `lemma` | TEXT | Normalized base form (e.g., "run") |
| `frequency_rank` | INTEGER | Rank in the corpus (1 = most frequent) |
| `frequency_count` | INTEGER | Raw occurrence count |
| `surface_forms` | JSON | Stats on original forms (e.g., `{"Run": 10, "run": 90}`) |

### `definitions` Table
Stores meanings and parts of speech (1-to-Many with `words`).

| Column | Type | Description |
| :--- | :--- | :--- |
| `id` | INTEGER PK | Unique ID |
| `word_id` | INTEGER FK | References `words.id` |
| `part_of_speech` | TEXT | POS tag (e.g., "Noun", "Vt") |
| `meaning` | TEXT | Japanese definition |
| `source` | TEXT | "jmdict" or "wiktionary" |

### `word_relations` Table
Stores pre-calculated semantic relationships (Many-to-Many).

| Column | Type | Description |
| :--- | :--- | :--- |
| `id` | INTEGER PK | Unique ID |
| `word_id_1` | INTEGER FK | Source word ID |
| `word_id_2` | INTEGER FK | Target (related) word ID |
| `relation_type` | TEXT | "similar_top_k" |
| `score` | REAL | Cosine similarity score (0.0 - 1.0) |

## 4. Design Decisions & Trade-offs

### 4.1. Pre-calculation vs. Runtime AI
*   **Decision:** All vector calculations are done in the pipeline (Phase 4), not in the app.
*   **Reason:** Running a Transformer model inside a lightweight desktop app bloats the binary size (hundreds of MBs) and requires significant RAM/CPU. Pre-calculating relations into SQLite keeps the app fast and small (~100MB database).

### 4.2. SQLite as a Read-Only Artifact
*   **Decision:** The app treats `words.sqlite3` as an immutable asset.
*   **Reason:** Simplifies concurrency (no write locks), enables easy updates (replace the file), and prevents user data corruption. User-specific data (bookmarks, history) would be stored in a separate DB (future phase).

### 4.3. Full-Text Search (FTS)
*   **Decision:** Used `words_fts` (FTS5) for fast prefix searching.
*   **Reason:** FTS5's `MATCH` operator with prefix queries (`"query"*`) provides efficient, ranked prefix search. The search query is quoted and escaped to handle special characters safely.

## 5. Future Considerations

*   **Incremental Updates:** Distributing only the delta of the database instead of the full file.
*   **Mobile Support:** Tauri supports mobile, so the architecture (Rust/React) is ready for iOS/Android adaptation.
*   **Flashcard UI Enhancement:** Context-based flashcards (fill-in-the-blank), progress visualization.
