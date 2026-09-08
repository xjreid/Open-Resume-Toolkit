"""Read-only regression gate for reviewed synthetic style output, not an importer."""
from pathlib import Path
import hashlib
import json
import sys

STYLES = ("technical", "professional", "modern")
KINDS = ("standard", "sparse", "unicode", "hostile", "dense", "optional", "structured", "paginated")


def snapshot(roots):
    result = {}
    for schema, root in enumerate(roots, 1):
        for style in STYLES:
            for kind in KINDS:
                pdf = root / style / "pdf"
                docx = root / style / "docx"
                receipt = json.loads((pdf / f"{kind}.json").read_text())
                assert receipt["documentSchemaVersion"] == schema
                assert receipt["templateId"] == f"{style}_pdf_v1"
                assert receipt["rendererVersion"] == "typst-0.15.1/ort-1"
                result[f"v{schema}/{style}/{kind}"] = {
                    "pdf": hashlib.sha256((pdf / f"{kind}.pdf").read_bytes()).hexdigest(),
                    "docx": hashlib.sha256((docx / f"{kind}.docx").read_bytes()).hexdigest(),
                    "text": hashlib.sha256((pdf / f"{kind}.txt").read_bytes()).hexdigest(),
                    "pages": receipt["pageCount"],
                    "templateSha256": receipt["templateSha256"],
                    "fontBundleSha256": receipt["fontBundleSha256"],
                }
    return result


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: verify-style-goldens.py V1_STYLE_DIRECTORY V2_STYLE_DIRECTORY")
    expected = json.loads((Path(__file__).resolve().parent.parent / "fixtures/documents/styles-v1.sha256.json").read_text())
    actual = snapshot([Path(value) for value in sys.argv[1:]])
    assert set(actual) == set(expected), "complete 48-pair style regression corpus"
    for key in actual:
        assert actual[key] == expected[key], f"{key}: reviewed style output changed; review/versioning required"
    print("48 PDF/DOCX/text style pairs match reviewed local regression hashes and page counts. Native qualification remains separate.")


if __name__ == "__main__":
    main()
