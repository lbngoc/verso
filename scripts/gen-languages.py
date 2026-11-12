#!/usr/bin/env python3
"""Generate src-tauri/resources/languages.json from translate-shell's LanguageData.awk.

The AWK file is the upstream locale database (translate-shell, Unlicense). We only take the
fields the app needs: the locale code, its English name, its endonym, whether its script is
right-to-left, and the Google/Bing codes. Attribution lives in THIRD_PARTY.md.

Bing code mapping follows the engine contract (specs/_shared/contracts/engine-contract.md):
the Microsoft service uses its own codes for a handful of locales.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "references" / "translate-shell" / "include" / "LanguageData.awk"
TARGET = ROOT / "src-tauri" / "resources" / "languages.json"

LINE = re.compile(r'Locale\["([^"]+)"\]\["([^"]+)"\]\s*=\s*"((?:[^"\\]|\\.)*)"')

RTL_SCRIPTS = {
    "Adlm",
    "Arab",
    "Aran",
    "Hebr",
    "Mand",
    "Mend",
    "Nkoo",
    "Rohg",
    "Samr",
    "Syrc",
    "Thaa",
    "Yezi",
}

# locale -> Bing code (engine-contract § http-bing). Values default to the locale itself.
BING_CODES = {
    "zh-CN": "zh-Hans",
    "zh-TW": "zh-Hant",
    "zh-Hans": "zh-Hans",
    "zh-Hant": "zh-Hant",
    "pt": "pt",
    "pt-BR": "pt",
    "pt-PT": "pt-pt",
    "tl": "fil",
    "hmn": "mww",
    "ku": "kmr",
    "ckb": "ku",
    "mn": "mn-Cyrl",
    "no": "nb",
    "sr-Cyrl": "sr-Cyrl",
    "sr-Latn": "sr-Latn",
}


def parse_awklike(text: str) -> dict[str, dict[str, str]]:
    locales: dict[str, dict[str, str]] = {}
    for code, field, value in LINE.findall(text):
        locales.setdefault(code, {})[field] = value.replace('\\"', '"')
    return locales


def main() -> None:
    text = SOURCE.read_text(encoding="utf-8")
    locales = parse_awklike(text)

    rows = []
    for code, fields in locales.items():
        supported = {p.strip() for p in fields.get("supported-by", "").split(";") if p.strip()}
        google = code if "google" in supported else None
        bing = None
        if "bing" in supported:
            bing = BING_CODES.get(code, code)
        rows.append(
            {
                "code": code,
                "name_en": fields.get("name", code),
                "name_native": fields.get("endonym", fields.get("name", code)),
                "rtl": fields.get("script", "Latn") in RTL_SCRIPTS,
                "google": google,
                "bing": bing,
            }
        )

    rows.sort(key=lambda row: row["name_en"].lower())
    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(rows)} languages to {TARGET.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
