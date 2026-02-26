import requests
import gzip
import shutil
import os

JMDICT_URL = "http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz"
OUTPUT_GZ = "data/JMdict_e.gz"
OUTPUT_XML = "data/JMdict_e.xml"

def download_jmdict():
    if os.path.exists(OUTPUT_GZ):
        print(f"{OUTPUT_GZ} already exists. Skipping download.")
        return

    print(f"Downloading {JMDICT_URL}...")
    try:
        with requests.get(JMDICT_URL, stream=True) as r:
            r.raise_for_status()
            with open(OUTPUT_GZ, 'wb') as f:
                for chunk in r.iter_content(chunk_size=8192):
                    f.write(chunk)
        print("Download complete.")
    except Exception as e:
        print(f"Download failed: {e}")
        # Clean up partial file
        if os.path.exists(OUTPUT_GZ):
            os.remove(OUTPUT_GZ)
        raise

def extract_jmdict():
    if os.path.exists(OUTPUT_XML):
        print(f"{OUTPUT_XML} already exists. Skipping extraction.")
        return

    print(f"Extracting {OUTPUT_GZ}...")
    try:
        with gzip.open(OUTPUT_GZ, 'rb') as f_in:
            with open(OUTPUT_XML, 'wb') as f_out:
                shutil.copyfileobj(f_in, f_out)
        print("Extraction complete.")
    except Exception as e:
        print(f"Extraction failed: {e}")
        if os.path.exists(OUTPUT_XML):
            os.remove(OUTPUT_XML)
        raise

if __name__ == "__main__":
    os.makedirs("data", exist_ok=True)
    download_jmdict()
    extract_jmdict()
