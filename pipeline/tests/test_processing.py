import polars as pl
import spacy
import pytest

def test_lemmatization():
    nlp = spacy.load("en_core_web_sm")
    text = "The cats are running"
    doc = nlp(text)
    lemmas = [token.lemma_ for token in doc]
    assert "cat" in lemmas
    assert "run" in lemmas

def test_frequency_counting():
    data = {"lemma": ["cat", "cat", "dog", "cat", "dog"]}
    df = pl.DataFrame(data)
    counts = df.group_by("lemma").len().sort("len", descending=True)

    assert counts.filter(pl.col("lemma") == "cat")["len"][0] == 3
    assert counts.filter(pl.col("lemma") == "dog")["len"][0] == 2
