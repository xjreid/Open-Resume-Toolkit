#!/usr/bin/env python3
"""Build the DOCX guest and verify/copy immutable PDFium Wasm and notices.

No JavaScript glue is copied or executed. Download requires --download.
Generated files live only in target/parser-guests, never in a user profile.
"""
import argparse
import hashlib
import io
import json
import pathlib
import subprocess
import tarfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
ARCHIVE_SHA = "added6e8ac024f71cb61cf2b77a205d178e2bdde2e4048fbcd916f68b7264d56"
WASM_SHA = "3283857c1d26d4b11c64743deb41390c98733892515efea4e4426cc06496d512"
URL = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7881/pdfium-wasm.tgz"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--download", action="store_true")
    args = parser.parse_args()
    archive = ROOT / "target/wasm-containment/pdfium-wasm.tgz"
    if not archive.exists() and args.download:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read(2_543_847)
        if len(data) != 2_543_846 or hashlib.sha256(data).hexdigest() != ARCHIVE_SHA:
            raise SystemExit("PDFium archive identity mismatch")
        archive.parent.mkdir(parents=True, exist_ok=True)
        archive.write_bytes(data)
    data = archive.read_bytes()
    if len(data) != 2_543_846 or hashlib.sha256(data).hexdigest() != ARCHIVE_SHA:
        raise SystemExit("PDFium archive identity mismatch")
    output = ROOT / "target/parser-guests"
    output.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(data)) as bundle:
        names = ["lib/pdfium.wasm", "LICENSE", "VERSION", "args.gn"]
        names += [member.name for member in bundle.getmembers()
                  if member.name.startswith("licenses/") and member.isfile()]
        for name in names:
            member = bundle.getmember(name)
            parts = pathlib.PurePosixPath(name).parts
            if not member.isfile() or member.size > 8 * 1024 * 1024 or ".." in parts:
                raise SystemExit("Unexpected PDFium archive member")
            content = bundle.extractfile(member).read(8 * 1024 * 1024 + 1)
            if name == "lib/pdfium.wasm":
                if len(content) != 5_233_982 or hashlib.sha256(content).hexdigest() != WASM_SHA:
                    raise SystemExit("PDFium module identity mismatch")
                target = output / "pdfium.wasm"
            else:
                target = output / "pdfium-notices" / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)
    subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-wasip1",
                    "-p", "ort-document-worker", "--no-default-features"], cwd=ROOT, check=True)
    docx = (ROOT / "target/wasm32-wasip1/release/ort-document-worker.wasm").read_bytes()
    if not docx.startswith(b"\x00asm\x01\x00\x00\x00") or len(docx) > 8 * 1024 * 1024:
        raise SystemExit("Unexpected DOCX module")
    (output / "docx.wasm").write_bytes(docx)
    manifest = {"schemaVersion": 1, "policy": "wasmi-guest-v1", "runtime": "wasmi-1.1.0",
                "rustTarget": "wasm32-wasip1", "pdfiumRelease": "chromium/7881",
                "pdfiumArchiveUrl": URL, "pdfiumArchiveSha256": ARCHIVE_SHA,
                "modules": {"docx.wasm": {"bytes": len(docx), "sha256": hashlib.sha256(docx).hexdigest()},
                            "pdfium.wasm": {"bytes": 5_233_982, "sha256": WASM_SHA}}}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print("Verified parser guests and PDFium license notices in target/parser-guests")


if __name__ == "__main__":
    main()
