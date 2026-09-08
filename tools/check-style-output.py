"""Generate and independently check synthetic style output in a fresh directory."""
from pathlib import Path
import subprocess
import sys
import tempfile


def run(*command):
    subprocess.run(command, check=True)


def main():
    Path("target").mkdir(exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="style-output-", dir="target"))
    print(f"Synthetic style evidence: {root}", flush=True)
    for schema in (1, 2):
        directory = root / f"v{schema}"
        mode = ("schema-v2",) if schema == 2 else ()
        run("cargo", "run", "--locked", "-p", "ort-render", "--example", "style_fixtures", "--", str(directory), *mode)
        for style in ("technical", "professional", "modern"):
            run(sys.executable, "tools/verify-docx-fixtures.py", str(directory / style / "docx"), style, *mode)
            run("node", "tools/verify-pdf-fixtures.mjs", str(directory / style / "pdf"), style, *mode)
    run(sys.executable, "tools/verify-style-goldens.py", str(root / "v1"), str(root / "v2"))


if __name__ == "__main__":
    main()
