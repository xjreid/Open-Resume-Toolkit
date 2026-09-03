"""Independent, read-only output audit. Synthetic fixtures only; not an importer.

Uses standard-library ZIP/CRC/XML readers, independently of the Rust encoder.
Run after cargo run -p ort-documents --example docx_fixtures -- NEW_DIRECTORY.
"""
import io
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET
from urllib.parse import urlsplit
from zipfile import ZIP_STORED, ZipFile

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"
REL = "{http://schemas.openxmlformats.org/package/2006/relationships}"
CT = "{http://schemas.openxmlformats.org/package/2006/content-types}"
PARTS = {"[Content_Types].xml", "_rels/.rels", "word/document.xml",
         "word/_rels/document.xml.rels", "word/styles.xml", "word/numbering.xml"}
KINDS = ("standard", "sparse", "unicode", "hostile", "dense")


def normalized(value):
    return value.replace("\r\n", "\n").replace("\r", "\n").strip()


def expected_paragraphs(source):
    paragraphs, links = [], []

    def add(value, style="Normal", bullet=False):
        value = normalized(value)
        if value:
            paragraphs.append((value, style, bullet))

    def link(value):
        label, url = normalized(value["label"]), normalized(value["url"])
        add(url if not label or label == url else f"{label}: {url}")
        links.append(url)

    contact = source["contact"]
    add(contact["fullName"], "Title")
    for key in ("email", "phone", "location"):
        add(contact[key])
    for value in contact["links"]:
        link(value)
    for section in source["sections"]:
        begin = len(paragraphs)
        for entry in section["entries"]:
            add(entry["heading"], "Heading2")
            for key in ("subheading", "dateRange", "location"):
                add(entry[key])
            for field in entry["fields"]:
                if normalized(field["value"]):
                    label, value = normalized(field["label"]), normalized(field["value"])
                    add(f"{label}: {value}" if label else value)
            for value in entry["bullets"]:
                add(value["text"], "ListParagraph", True)
            for value in entry["links"]:
                link(value)
        if len(paragraphs) > begin:
            paragraphs.insert(begin, (normalized(section["heading"]), "Heading1", False))
    return paragraphs, links


def verify(data, source):
    assert 0 < len(data) <= 2097152, "archive bound"
    with ZipFile(io.BytesIO(data)) as archive:
        infos = archive.infolist()
        assert len(infos) == 6 and set(archive.namelist()) == PARTS, "fixed parts only"
        for info in infos:
            assert info.compress_type == ZIP_STORED and not info.flag_bits & 1
            assert info.date_time == (1980, 1, 1, 0, 0, 0), "deterministic timestamp"
            assert not info.extra and not info.comment and info.file_size <= 1048576
        assert not archive.comment
        assert archive.testzip() is None, "ZIP CRC"
        contents = {name: archive.read(name) for name in PARTS}
    assert all(b"<!DOCTYPE" not in body and b"<!ENTITY" not in body for body in contents.values())
    xml = {name: ET.fromstring(body) for name, body in contents.items()}
    document = xml["word/document.xml"]
    assert document.tag == W + "document"
    allowed = {W + tag for tag in ("document", "body", "p", "pPr", "pStyle", "r", "t",
               "br", "tab", "numPr", "ilvl", "numId", "hyperlink", "sectPr", "pgSz", "pgMar")}
    assert all(node.tag in allowed for node in document.iter()), "no fields, media or active content"
    body = document.find(W + "body")
    assert body is not None
    actual = []
    for p in body.findall(W + "p"):
        text = "".join((n.text or "") if n.tag == W + "t" else
                       "\n" if n.tag == W + "br" else "\t" if n.tag == W + "tab" else ""
                       for n in p.iter())
        style = p.find(f"{W}pPr/{W}pStyle")
        bullet = p.find(f"{W}pPr/{W}numPr/{W}numId")
        if bullet is not None:
            assert bullet.get(W + "val") == "1"
        actual.append((text, style.get(W + "val") if style is not None else "Normal", bullet is not None))
    expected, links = expected_paragraphs(source)
    assert actual == expected, "semantic text, heading, list and ordering parity"
    rels = xml["word/_rels/document.xml.rels"]
    assert rels.tag == REL + "Relationships" and len(rels) == len(links) + 2
    assert all(node.tag == REL + "Relationship" for node in rels)
    assert len({node.get("Id") for node in rels}) == len(rels)
    fixed = list(rels)[:2]
    for node, name in zip(fixed, ("styles", "numbering")):
        assert node.attrib == {"Id": name, "Type": R[1:-1] + "/" + name, "Target": name + ".xml"}
    external = list(rels)[2:]
    for index, (node, url) in enumerate(zip(external, links), 1):
        assert node.attrib == {"Id": f"link{index}", "Type": R[1:-1] + "/hyperlink", "TargetMode": "External", "Target": url}
        assert urlsplit(url).scheme in ("http", "https", "mailto")
    assert [n.get(R + "id") for n in body.iter(W + "hyperlink")] == [f"link{i}" for i in range(1, len(links) + 1)]
    root_rels = xml["_rels/.rels"]
    assert len(root_rels) == 1 and root_rels[0].attrib == {
        "Id": "document", "Type": R[1:-1] + "/officeDocument", "Target": "word/document.xml"}
    types = xml["[Content_Types].xml"]
    assert types.tag == CT + "Types" and len(types) == 4
    assert {n.get("PartName") for n in types if n.tag == CT + "Override"} == {
        "/word/document.xml", "/word/styles.xml", "/word/numbering.xml"}
    assert all("macroEnabled" not in value for node in types for value in node.attrib.values())
    styles = xml["word/styles.xml"]
    ids = {node.get(W + "styleId") for node in styles.findall(W + "style")}
    assert ids == {"Normal", "Title", "Heading1", "Heading2", "ListParagraph"}
    for style, level in (("Heading1", "0"), ("Heading2", "1")):
        node = next(n for n in styles if n.get(W + "styleId") == style)
        assert node.find(f"{W}pPr/{W}outlineLvl").get(W + "val") == level
        assert node.find(f"{W}pPr/{W}keepNext") is not None
    numbering = xml["word/numbering.xml"]
    assert numbering.find(f"{W}abstractNum/{W}lvl/{W}numFmt").get(W + "val") == "bullet"
    assert numbering.find(f"{W}num/{W}abstractNumId").get(W + "val") == "0"
    section = body.find(W + "sectPr")
    assert section.find(W + "pgSz").attrib == {W + "w": "12240", W + "h": "15840"}
    assert all(section.find(W + "pgMar").get(W + key) == "1440" for key in ("top", "right", "bottom", "left"))
    return contents


def rejection_checks(data, source):
    original = verify(data, source)
    changes = [
        {"word/vbaProject.bin": b"unexpected"},
        {"word/document.xml": original["word/document.xml"].replace(b"<w:body>", b"<w:body><w:object/>")},
        {"word/document.xml": original["word/document.xml"].replace(b"Software Engineer", b"Changed content")},
        {"word/document.xml": original["word/document.xml"].replace(b'val="Heading1"', b'val="Normal"')},
        {"word/_rels/document.xml.rels": original["word/_rels/document.xml.rels"].replace(b"/hyperlink", b"/attachedTemplate")},
        {"word/_rels/document.xml.rels": original["word/_rels/document.xml.rels"].replace(b"https://example.org/project", b"file:///private/secret")},
    ]
    for change in changes:
        mutated = io.BytesIO()
        # Preserve valid ZIP metadata so rejection is about the changed content.
        from zipfile import ZipInfo
        with ZipFile(mutated, "w") as archive:
            for name, body in (original | change).items():
                archive.writestr(ZipInfo(name), body)
        try:
            verify(mutated.getvalue(), source)
        except AssertionError:
            continue
        raise AssertionError("mutated package was incorrectly accepted")


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify-docx-fixtures.py SYNTHETIC_DIRECTORY")
    root = Path(sys.argv[1])
    goldens = json.loads((Path(__file__).resolve().parent.parent / "fixtures/documents/docx-v1.sha256.json").read_text(encoding="utf-8"))
    assert set(goldens) == set(KINDS), "complete golden corpus"
    for name in KINDS:
        data = (root / (name + ".docx")).read_bytes()
        source = json.loads((root / (name + ".json")).read_text(encoding="utf-8"))
        verify(data, source)
        assert hashlib.sha256(data).hexdigest() == goldens[name], "review required: deterministic DOCX bytes changed"
    rejection_checks((root / "standard.docx").read_bytes(), json.loads((root / "standard.json").read_text(encoding="utf-8")))
    print("Five DOCX fixtures: golden SHA-256, ZIP/CRC, fixed OPC parts, XML, semantic parity, relationships, headings/lists and geometry passed; six negative controls rejected.")


if __name__ == "__main__":
    main()
