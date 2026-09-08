#!/usr/bin/env python3
"""Native qualification of the signed, job-only parser helper using synthetic data."""
import hashlib
import json
import io
import zipfile
import pathlib
import resource
import select
import struct
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
HELPER = ROOT / "target/m2-candidate/Open Resume Toolkit Dev.app/Contents/Helpers/ORT Parser Helper.app/Contents/MacOS/ort-parser-helper"
REPORT = ROOT / "target/m2-parser-helper-native.json"


def packet(data, format_id=1):
    return b"ORTW" + bytes([format_id]) + struct.pack("<I", len(data)) + data


def run(data, expected_success):
    result = subprocess.run([str(HELPER)], input=data, capture_output=True, timeout=65)
    assert (result.returncode == 0) == expected_success, result.returncode
    assert not result.stderr
    assert len(result.stdout) <= 512 * 1024
    if expected_success:
        output = json.loads(result.stdout)
        assert output["blocks"] and output["version"] == 1
    else:
        assert not result.stdout


def main():
    if sys.platform != "darwin":
        raise SystemExit("macOS qualification only")
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", str(HELPER)], check=True)
    docx = (ROOT / "target/m2-style-parity-repeat-v1/technical/docx/standard.docx").read_bytes()
    pdf = (ROOT / "target/m2-style-parity-repeat-v1/technical/pdf/standard.pdf").read_bytes()
    checks = []
    for _ in range(3):
        run(packet(docx), True)
        run(packet(pdf, 2), True)
    checks.append("repeated_pdf_docx_jobs")
    for invalid in [b"", b"BAD!" + bytes(5), b"ORTW" + bytes([3]) + bytes(4),
                    packet(b"not-docx"), packet(b"%PDF-1.7\n%%EOF", 2),
                    packet(docx) + b"trailing", packet(docx)[:-1],
                    b"ORTW" + bytes([1]) + struct.pack("<I", 10 * 1024 * 1024 + 1)]:
        run(invalid, False)
    checks.append("invalid_truncated_trailing_oversized_input")
    # Kill the immediate parent while its child has an incomplete request and
    # the parent keeps stdin open. EOF may race watchdog; either must terminate
    # the owned worker without any result. No unrelated PID is signalled.
    code = """import subprocess,sys,time
child=subprocess.Popen([sys.argv[1]],stdin=subprocess.PIPE,stdout=sys.stdout,stderr=sys.stderr)
print(child.pid,flush=True)
time.sleep(120)
"""
    parent = subprocess.Popen([sys.executable, "-c", code, str(HELPER)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        assert select.select([parent.stdout], [], [], 5)[0]
        pid = int(parent.stdout.readline())
        assert pid > 1
        time.sleep(0.2)
        parent.kill()
        parent.wait(timeout=5)
        # Held inherited output pipe reaches EOF only when the helper closes it.
        assert select.select([parent.stdout], [], [], 3)[0]
        assert parent.stdout.read() == b""
        assert parent.stderr.read() == b""
    finally:
        if parent.poll() is None:
            parent.kill()
            parent.wait(timeout=5)
    checks.append("parent_death_closes_private_output")
    # A valid large extraction fills the private pipe. Do not drain it until
    # the independent watchdog exits, proving blocked output cannot live forever.
    container = io.BytesIO()
    text = "Synthetic bounded output line " * 3
    paragraphs = "".join(f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>" for _ in range(520))
    xml = f'<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{paragraphs}</w:body></w:document>'.encode()
    with zipfile.ZipFile(io.BytesIO(docx)) as original, zipfile.ZipFile(container, "w", compression=zipfile.ZIP_STORED) as rebuilt:
        for item in original.infolist():
            rebuilt.writestr(item.filename, xml if item.filename == "word/document.xml" else original.read(item.filename))
    large = packet(container.getvalue())
    baseline = subprocess.run([str(HELPER)], input=large, capture_output=True, timeout=10)
    assert baseline.returncode == 0 and 64 * 1024 < len(baseline.stdout) <= 512 * 1024, (baseline.returncode, len(baseline.stdout))
    stalled_start = time.monotonic()
    stalled = subprocess.Popen([str(HELPER)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stalled.stdin.write(large)
    stalled.stdin.close()
    # Keep a partial request open. The trusted helper watchdog owns the full
    # real 60-second deadline; this is not a shortened test-only timer.
    start = time.monotonic()
    child = subprocess.Popen([str(HELPER)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        child.stdin.write(b"ORTW")
        child.stdin.flush()
        status = child.wait(timeout=65)
        elapsed = time.monotonic() - start
        assert status == 124 and 59 <= elapsed < 65, (status, elapsed)
        child.stdin.close()
        assert child.stdout.read() == b"" and child.stderr.read() == b""
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(timeout=5)
    checks.append("real_60_second_partial_input_deadline")
    try:
        assert stalled.wait(timeout=5) == 124
        assert 59 <= time.monotonic() - stalled_start < 70
        # This direct helper may have put bytes in its pipe. The production
        # parent rejects them because exit 124 never grants a successful result.
        assert 0 < len(stalled.stdout.read()) <= 512 * 1024
        assert stalled.stderr.read() == b""
    finally:
        if stalled.poll() is None:
            stalled.kill()
            stalled.wait(timeout=5)
    checks.append("real_60_second_blocked_output_deadline")
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    REPORT.write_text(json.dumps({"passed": checks, "helperSha256": hashlib.sha256(HELPER.read_bytes()).hexdigest(),
                                 "deadlineElapsedSeconds": elapsed, "peakChildResidentBytes": usage.ru_maxrss}, indent=2) + "\n")
    print("PASS: signed helper lifecycle and input boundary checks")


if __name__ == "__main__":
    main()
