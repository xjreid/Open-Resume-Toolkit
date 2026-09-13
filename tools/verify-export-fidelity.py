"""Independent pypdf/OOXML audit of the synthetic m25_designs/export_fidelity examples.

Usage: python tools/verify-export-fidelity.py OUTPUT_DIRECTORY
Visual inspection of the PDFs and rendered DOCX pages is still required.
"""
from collections import Counter
import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET
from zipfile import ZipFile

from pypdf import PdfReader

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def normalize(text):
    text = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", text)
    return re.sub(r"[\s*_•|]+", "", text).casefold()


def phrases(source):
    contact = source["contact"]
    yield from (contact.get(key, "") for key in ("fullName", "email", "phone", "location"))
    yield from (link["label"] for link in contact["links"])
    for section in source["sections"]:
        yield section["heading"]
        for entry in section["entries"]:
            yield from (entry.get(key, "") for key in ("heading", "subheading", "location", "dateRange"))
            yield from (field["value"] for field in entry["fields"])
            yield from (bullet["text"] for bullet in entry["bullets"])
            yield from (link["label"] for link in entry["links"])


def check_content(text, source):
    normalized = normalize(text)
    for phrase, count in Counter(normalize(value) for value in phrases(source) if value).items():
        assert normalized.count(phrase) >= count, f"Missing source text: {phrase}"
    cursor = 0
    for section in source["sections"]:
        cursor = normalized.index(normalize(section["heading"]), cursor)
        for entry in section["entries"]:
            if entry["heading"]:
                cursor = normalized.index(normalize(entry["heading"]), cursor)
            for bullet in entry["bullets"]:
                cursor = normalized.index(normalize(bullet["text"]), cursor) + len(normalize(bullet["text"]))


def roles(node):
    node = node.get_object() if hasattr(node, "get_object") else node
    if isinstance(node, list):
        for child in node:
            yield from roles(child)
    elif isinstance(node, dict):
        yield node.get("/S")
        yield from roles(node.get("/K"))


def audit(root, name, source_name):
    source = json.loads((root / source_name).read_text())
    pdf = PdfReader(root / f"{name}.pdf", strict=True)
    assert 1 <= len(pdf.pages) <= 5 and not pdf.is_encrypted
    structure = set(roles(pdf.trailer["/Root"]["/StructTreeRoot"]))
    assert {"/H1", "/H2", "/L", "/LI", "/LBody"} <= structure
    text = ""
    for page in pdf.pages:
        assert list(page.mediabox) == [0, 0, 612, 792]
        assert not page.images, "Resume must remain real text"
        text += page.extract_text() + "\n"
        for reference in page["/Resources"]["/Font"].values():
            font = reference.get_object()
            assert "/ToUnicode" in font
            for descendant in font["/DescendantFonts"]:
                descriptor = descendant.get_object()["/FontDescriptor"]
                assert any(descriptor.get(key) for key in ("/FontFile", "/FontFile2", "/FontFile3"))
        for reference in page.get("/Annots", []):
            annotation = reference.get_object()
            assert annotation["/Subtype"] == "/Link"
            assert annotation["/A"]["/S"] == "/URI"
            assert annotation["/A"]["/URI"].startswith(("http://", "https://", "mailto:"))
    check_content(text, source)
    with ZipFile(root / f"{name}.docx") as archive:
        assert not any("header" in path or "footer" in path for path in archive.namelist())
        document = ET.fromstring(archive.read("word/document.xml"))
        assert next(document.iter(W + "tbl"), None) is None
        assert next(document.iter(W + "txbxContent"), None) is None
        assert next(document.iter(W + "numPr"), None) is not None
        check_content(" ".join(element.text or "" for element in document.iter(W + "t")), source)
    rendered_docx = root / f"docx-{name}" / f"{name}.pdf"
    if rendered_docx.exists():
        rendered = PdfReader(rendered_docx, strict=True)
        check_content("\n".join(page.extract_text() for page in rendered.pages), source)
    print(f"{name}: {len(pdf.pages)} PDF page(s); all source text, reading order, embedded fonts, heading/list tags; semantic DOCX; rendered DOCX checked: {rendered_docx.exists()}")


root = Path(sys.argv[1])
for style in ("technical", "professional", "modern"):
    audit(root, style, "representative.source.json")
    audit(root, f"edge-{style}", "edge-cases.source.json")
audit(root, "technical-long", "technical-long.source.json")
