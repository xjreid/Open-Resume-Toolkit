use std::path::PathBuf;

use ort_document_worker::extract_pdf;
use ort_documents::import::{InputFormat, ValidatedExtraction};

fn synthetic_pdf() -> Vec<u8> {
    let stream = b"BT /F1 18 Tf 72 720 Td (Experience) Tj 0 -24 Td (- Built safely) Tj ET";
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        format!("<< /Length {} >>\nstream\n", stream.len())
            .into_bytes()
            .into_iter()
            .chain(stream.iter().copied())
            .chain(b"\nendstream".iter().copied())
            .collect(),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[test]
#[ignore = "requires an independently verified, target-matching PDFium library"]
fn pinned_native_pdfium_extracts_synthetic_text() {
    let library = PathBuf::from(
        std::env::var_os("ORT_TEST_PDFIUM_LIBRARY")
            .expect("set ORT_TEST_PDFIUM_LIBRARY to the absolute pinned PDFium library path"),
    );
    let wire = extract_pdf(&mut synthetic_pdf().as_slice(), &library).unwrap();
    let extraction = ValidatedExtraction::decode(&wire, InputFormat::Pdf).unwrap();
    assert_eq!(extraction.page_count(), 1);
    assert!(
        extraction
            .blocks()
            .iter()
            .any(|block| block.text.contains("Experience"))
    );
    assert!(
        extraction
            .blocks()
            .iter()
            .any(|block| block.text.contains("Built safely"))
    );
}
