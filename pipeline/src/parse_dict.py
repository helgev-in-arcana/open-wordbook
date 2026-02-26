import xml.etree.ElementTree as ET
from collections import defaultdict
import os

def clean_pos(pos_text):
    if not pos_text:
        return ""
    p = pos_text.lower()

    # Common mappings based on JMdict codes/expansions
    if "noun" in p: return "Noun"
    if "verb" in p: return "Verb"
    if "adjective" in p or p == "adj": return "Adj"
    if "adverb" in p or p == "adv": return "Adv"
    if "pronoun" in p: return "Pron"
    if "preposition" in p: return "Prep"
    if "conjunction" in p: return "Conj"
    if "interjection" in p: return "Int"
    if "particle" in p: return "Part"
    if "auxiliary" in p: return "Aux"
    if "counter" in p: return "Ctr"

    # If short code (e.g. 'n', 'v', 'vt'), uppercase
    if len(p) <= 5:
        return p.upper()

    # Fallback: Truncate or Capitalize
    # Some descriptions are very long. Just return "Other" or Capitalized?
    # Let's Capitalize but truncate if too long?
    # Usually the above covers most.
    return p.capitalize()

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

            # Extract Priority (frequency info)
            is_common = False

            # Extract Japanese Headword
            japanese_word = ""
            k_ele = elem.find("k_ele")
            if k_ele is not None:
                keb_elem = k_ele.find("keb")
                if keb_elem is not None and keb_elem.text:
                    japanese_word = keb_elem.text
                    # Check priority
                    for pri in k_ele.findall("ke_pri"):
                        if pri.text and (pri.text.startswith("news") or pri.text.startswith("ichi") or pri.text.startswith("spec") or pri.text.startswith("nf")):
                            is_common = True

                    # Append reading
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
                        # Check priority
                        for pri in r_ele.findall("re_pri"):
                             if pri.text and (pri.text.startswith("news") or pri.text.startswith("ichi") or pri.text.startswith("spec") or pri.text.startswith("nf")):
                                is_common = True

            if not japanese_word:
                root.clear()
                continue

            # Process Senses
            for sense in elem.findall("sense"):
                # POS
                pos_list = [p.text for p in sense.findall("pos") if p.text]
                # Clean and dedup POS
                cleaned_pos_list = sorted(list(set(clean_pos(p) for p in pos_list)))
                pos = ", ".join(cleaned_pos_list)

                # Glosses
                for gloss in sense.findall("gloss"):
                    text = gloss.text
                    if text:
                        # Split by semicolon for multiple meanings in one gloss
                        parts = text.split(";")
                        for part in parts:
                            english_word = part.strip().lower()
                            if not english_word: continue

                            # Heuristic: Remove "to " from verbs
                            if english_word.startswith("to ") and ("v" in pos.lower() or "verb" in pos.lower()):
                                 english_word = english_word[3:]

                            entry = {
                                "pos": pos,
                                "meaning": japanese_word,
                                "source": "jmdict",
                                "is_common": is_common
                            }

                            # Avoid appending exact duplicate consecutively
                            if not (dictionary[english_word] and dictionary[english_word][-1] == entry):
                                dictionary[english_word].append(entry)

            # Clear processed element
            root.clear()

    print(f"\nParsed {len(dictionary)} unique English words.")

    # Sort entries by commonness (True first)
    print("Sorting dictionary entries by priority...")
    for k in dictionary:
        dictionary[k].sort(key=lambda x: x.get('is_common', False), reverse=True)

    return dictionary

if __name__ == "__main__":
    xml_path = "data/JMdict_e.xml"
    if os.path.exists(xml_path):
        d = load_dictionary(xml_path)
        if "cat" in d:
            print("cat:", d["cat"])
    else:
        print(f"{xml_path} not found.")
