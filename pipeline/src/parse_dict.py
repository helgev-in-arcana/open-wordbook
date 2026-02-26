import xml.etree.ElementTree as ET
from collections import defaultdict
import os

def clean_pos(pos_text):
    if not pos_text:
        return ""
    p = pos_text.strip().lower()

    # Specific Grammar Types (preserve nuance)
    if "transitive verb" in p: return "Vt"
    if "intransitive verb" in p: return "Vi"
    if "adjectival noun" in p or "na-adjective" in p: return "Na"
    if "godan verb" in p: return "Verb (Godan)"
    if "ichidan verb" in p: return "Verb (Ichidan)"
    if "suru verb" in p: return "Vs"

    # General Categories
    if "noun" in p: return "Noun"
    if "verb" in p: return "Verb"
    if "adjective" in p or p == "adj": return "Adj"
    if "adverb" in p or p == "adv": return "Adv"
    if "pronoun" in p: return "Pron"
    if "preposition" in p: return "Prep"
    if "conjunction" in p or p == "conj": return "Conj"
    if "interjection" in p: return "Int"
    if "particle" in p: return "Part"
    if "auxiliary" in p: return "Aux"
    if "counter" in p: return "Ctr"
    if "expression" in p or p == "exp": return "Expr"
    if "numeric" in p: return "Num"
    if "prefix" in p: return "Prefix"
    if "suffix" in p: return "Suffix"

    # Short codes (e.g. 'n', 'v'), Title Case (e.g. 'N', 'V')
    if len(p) <= 5:
        return p.title()

    # Default fallback for unhandled long strings
    # Return "Other" or generic "Misc" to avoid UI clutter
    if len(p) > 20:
        return "Other"

    return p.title()

def load_dictionary(xml_file):
    if not os.path.exists(xml_file):
        print(f"Error: {xml_file} not found.")
        return {}

    print(f"Parsing {xml_file}...")
    dictionary = defaultdict(list)

    context = ET.iterparse(xml_file, events=("start", "end"))
    context = iter(context)
    event, root = next(context)

    count = 0
    for event, elem in context:
        if event == "end" and elem.tag == "entry":
            count += 1
            if count % 10000 == 0:
                print(f"Processed {count} entries...", end="\r")

            is_common = False

            japanese_word = ""
            k_ele = elem.find("k_ele")
            if k_ele is not None:
                keb_elem = k_ele.find("keb")
                if keb_elem is not None and keb_elem.text:
                    japanese_word = keb_elem.text
                    for pri in k_ele.findall("ke_pri"):
                        if pri.text and (pri.text.startswith("news") or pri.text.startswith("ichi") or pri.text.startswith("spec") or pri.text.startswith("nf")):
                            is_common = True

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
                        for pri in r_ele.findall("re_pri"):
                             if pri.text and (pri.text.startswith("news") or pri.text.startswith("ichi") or pri.text.startswith("spec") or pri.text.startswith("nf")):
                                is_common = True

            if not japanese_word:
                root.clear()
                continue

            for sense in elem.findall("sense"):
                pos_list = [p.text for p in sense.findall("pos") if p.text]
                cleaned_pos_list = sorted(list(set(clean_pos(p) for p in pos_list)))
                pos = ", ".join(cleaned_pos_list)

                for gloss in sense.findall("gloss"):
                    text = gloss.text
                    if text:
                        parts = text.split(";")
                        for part in parts:
                            english_word = part.strip().lower()
                            if not english_word: continue

                            if english_word.startswith("to ") and ("v" in pos.lower() or "verb" in pos.lower()):
                                 english_word = english_word[3:]

                            entry = {
                                "pos": pos,
                                "meaning": japanese_word,
                                "source": "jmdict",
                                "is_common": is_common
                            }

                            if not (dictionary[english_word] and dictionary[english_word][-1] == entry):
                                dictionary[english_word].append(entry)

            root.clear()

    print(f"\nParsed {len(dictionary)} unique English words.")

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
