"""Optional independent pypdf audit of synthetic fixtures; not product code."""
from pathlib import Path
import sys
from pypdf import PdfReader

for kind in ("standard", "sparse", "unicode", "hostile", "dense"):
    reader = PdfReader(Path(sys.argv[1]) / f"{kind}.pdf", strict=True)
    assert not reader.is_encrypted
    assert not any(key in reader.metadata for key in ("/Author", "/Creator", "/CreationDate", "/ModDate"))
    assert "/StructTreeRoot" in reader.trailer["/Root"]
    total = 0
    for page in reader.pages:
        fonts = page["/Resources"]["/Font"]
        for ref in fonts.values():
            font = ref.get_object()
            assert font["/Subtype"] == "/Type0"
            assert "/ToUnicode" in font
            assert "LibertinusSerif" in font["/BaseFont"]
            for descendant in font["/DescendantFonts"]:
                descriptor = descendant.get_object()["/FontDescriptor"]
                streams = [descriptor[key].get_data() for key in ("/FontFile", "/FontFile2", "/FontFile3") if key in descriptor]
                assert len(streams) == 1 and len(streams[0]) > 0
            total += 1
    assert total > 0
    print(f"{kind}: embedded subset Libertinus fonts, ToUnicode maps, tagged structure, no personal/date metadata")
