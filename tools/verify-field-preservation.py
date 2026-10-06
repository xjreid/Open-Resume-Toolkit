"""Independently parse synthetic DOCX packages and plain text for retained markers."""
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET
from zipfile import ZipFile

root = Path(sys.argv[1])
markers = json.loads((root / "markers.json").read_text())
for style in ("plain", "technical", "professional", "modern"):
    with ZipFile(root / f"{style}.docx") as archive:
        assert archive.testzip() is None
        document = ET.fromstring(archive.read("word/document.xml"))
        text = "".join(document.itertext())
        for marker in markers:
            assert marker in text, (style, marker)
for marker in markers:
    assert marker in (root / "expected.txt").read_text(), marker
print("All seven retained markers survived every DOCX style and plain text.")
