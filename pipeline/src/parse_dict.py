import xml.etree.ElementTree as ET
from collections import defaultdict
import os

def load_dictionary(xml_file):
    if not os.path.exists(xml_file):
        print(f"Error: {xml_file} not found.")
        return {}

    print(f"Parsing {xml_file}...")
    dictionary = defaultdict(list)

    # Use iterparse to handle large XML files
    context = ET.iterparse(xml_file, events=("start", "end"))
    context = iter(context)
    event, root = next(context) # Get root element

    count = 0
    for event, elem in context:
        if event == "end" and elem.tag == "entry":
            count += 1
            if count % 10000 == 0:
                print(f"Processed {count} entries...", end="\r")

            # Extract Japanese Headword
            japanese_word = ""
            k_ele = elem.find("k_ele")
            if k_ele is not None:
                keb_elem = k_ele.find("keb")
                if keb_elem is not None and keb_elem.text:
                    japanese_word = keb_elem.text
                    # Append reading if available
                    r_ele = elem.find("r_ele")
                    if r_ele is not None:
                        reb_elem = r_ele.find("reb")
                        if reb_elem is not None and reb_elem.text:
                            japanese_word += f" ({reb_elem.text})"
            else:
                r_ele = elem.find("r_ele")
                if r_ele is not None:
                    reb_elem = r_ele.find("reb")
                    if reb_elem is not None and reb_elem.text:
                        japanese_word = reb_elem.text

            if not japanese_word:
                root.clear()
                continue

            # Process Senses
            for sense in elem.findall("sense"):
                # POS
                pos_list = [p.text for p in sense.findall("pos") if p.text]
                pos = ", ".join(pos_list)

                # Glosses
                for gloss in sense.findall("gloss"):
                    text = gloss.text
                    if text:
                        english_word = text.strip().lower()

                        # Heuristic: Remove "to " from verbs
                        if english_word.startswith("to ") and ("v" in pos or "verb" in pos):
                             english_word = english_word[3:]

                        entry = {
                            "pos": pos,
                            "meaning": japanese_word,
                            "source": "jmdict"
                        }

                        # Add if not duplicate of last entry (simple dedup)
                        if not (dictionary[english_word] and dictionary[english_word][-1] == entry):
                            dictionary[english_word].append(entry)

            # Clear processed element
            root.clear()

    print(f"\nParsed {len(dictionary)} unique English words.")
    return dictionary

if __name__ == "__main__":
    xml_path = "data/JMdict_e.xml"
    if os.path.exists(xml_path):
        d = load_dictionary(xml_path)
        if "cat" in d:
            print("cat:", d["cat"])
    else:
        print(f"{xml_path} not found.")
