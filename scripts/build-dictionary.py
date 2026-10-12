"""Builds src-tauri/data/en-vi-viwiktionary.tsv from the Vietnamese Wiktionary.

Input: raw-wiktextract-data.jsonl.gz from https://kaikki.org/viwiktionary/rawdata.html
Usage: python -I -X utf8 scripts/build-dictionary.py path/to/raw-wiktextract-data.jsonl.gz

One line per English word, sorted by the bytes of the lower-case word so the
app can binary-search the file: word, IPA, then one field per part of speech
written as `pos=gloss|gloss|...`, all separated by tabs.
"""
import gzip
import io
import json
import os
import re
import sys

OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "data", "en-vi-viwiktionary.tsv")
WORD = re.compile(r"^[A-Za-z][A-Za-z '\-]{0,39}$")
POS = {"adj": "adjective", "adv": "adverb", "prep": "preposition", "pron": "pronoun", "conj": "conjunction",
       "intj": "interjection", "num": "numeral", "name": "proper noun", "det": "determiner"}
MAX_POS, MAX_GLOSSES, MAX_LENGTH = 4, 4, 100


def clean(gloss):
    gloss = re.sub(r"\s+", " ", gloss).replace("|", "/").strip().rstrip(".").strip()
    if len(gloss) > MAX_LENGTH:
        gloss = gloss[:MAX_LENGTH].rsplit(" ", 1)[0] + "…"
    return gloss


def main(source):
    words = {}
    with gzip.open(source, "rt", encoding="utf-8") as lines:
        for line in lines:
            entry = json.loads(line)
            word = entry.get("word", "")
            if entry.get("lang_code") != "en" or not WORD.match(word):
                continue
            glosses = []
            for sense in entry.get("senses", []):
                # "Plural of …" and the like say nothing about the meaning.
                if sense.get("form_of") or sense.get("alt_of") or "form-of" in sense.get("tags", []):
                    continue
                for gloss in sense.get("glosses", [])[:1]:
                    gloss = clean(gloss)
                    if gloss and gloss not in glosses:
                        glosses.append(gloss)
            if not glosses:
                continue
            record = words.setdefault(word.lower(), {"ipa": "", "senses": {}})
            if not record["ipa"]:
                record["ipa"] = next((s["ipa"].strip("/[]") for s in entry.get("sounds", []) if s.get("ipa")), "")
            pos = POS.get(entry.get("pos", ""), entry.get("pos", "") or "word")
            kept = record["senses"].setdefault(pos, [])
            kept.extend(g for g in glosses if g not in kept)

    with io.open(OUT, "w", encoding="utf-8", newline="\n") as out:
        for key in sorted(words, key=lambda k: k.encode("utf-8")):
            record = words[key]
            fields = [key, record["ipa"].replace("\t", " ")]
            for pos, glosses in list(record["senses"].items())[:MAX_POS]:
                fields.append(pos + "=" + "|".join(glosses[:MAX_GLOSSES]))
            out.write("\t".join(fields) + "\n")
    print(len(words), "words,", os.path.getsize(OUT), "bytes")


if __name__ == "__main__":
    main(sys.argv[1])
