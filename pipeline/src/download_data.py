from datasets import load_dataset
import polars as pl
import os

def download_sample(output_path, num_rows=5000):
    print(f"Downloading sample of Fineweb-Edu (first {num_rows} rows)...")
    # Using a specific config or default. 'sample-10BT' is a common subset for this dataset.
    try:
        ds = load_dataset("HuggingFaceFW/fineweb-edu", name="sample-10BT", split="train", streaming=True)
    except Exception as e:
        print(f"Could not load sample-10BT, trying default: {e}")
        ds = load_dataset("HuggingFaceFW/fineweb-edu", split="train", streaming=True)

    data = []
    for i, item in enumerate(ds):
        if i >= num_rows:
            break
        data.append(item["text"])

    df = pl.DataFrame({"text": data})
    # ensure directory exists
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    df.write_parquet(output_path)
    print(f"Sample saved to {output_path} with {len(df)} rows.")

if __name__ == "__main__":
    # Script is run from pipeline root usually
    output_file = os.path.join("data", "sample.parquet")
    download_sample(output_file)
