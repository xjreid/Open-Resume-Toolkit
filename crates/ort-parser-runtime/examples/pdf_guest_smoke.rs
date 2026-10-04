use ort_parser_runtime::extract_pdf;
use std::fmt::Write as _;
fn main() {
    let bytes = std::fs::read("target/wasm-containment/pdfium.wasm").unwrap();
    let result = extract_pdf(&bytes, &synthetic_pdf()).expect("guest parse");
    assert_eq!(result.page_count(), 1);
    assert!(
        result
            .blocks()
            .iter()
            .any(|block| block.text.contains("Built safely"))
    );
    assert!(extract_pdf(&bytes, &image_pdf("")).is_err());
    assert!(extract_pdf(&bytes, &image_pdf("page 1")).is_err());
    assert!(
        extract_pdf(
            &bytes,
            &image_pdf("Substantial synthetic readable employment history")
        )
        .is_ok()
    );
    assert!(extract_pdf(&bytes, b"%PDF-1.7\ninvalid\n%%EOF").is_err());
    let extraction = extract_pdf(&bytes, &positioned_resume()).expect("positioned PDF parse");
    assert!(
        extraction
            .blocks()
            .iter()
            .all(|block| block.layout.is_some())
    );
    let resume = ort_documents::resume_import::map_resume(&extraction).unwrap();
    resume
        .validate(ort_domain::DocumentLimits::default())
        .unwrap();
    assert_eq!(resume.sections.len(), 2);
    assert_eq!(resume.sections[0].entries.len(), 1);
    let entry = &resume.sections[0].entries[0];
    assert_eq!(entry.heading, "Example Company");
    assert_eq!(entry.subheading, "Senior Engineer");
    assert_eq!(entry.location, "Boston, MA");
    assert_eq!(entry.date_range, "2022 - Present");
    assert_eq!(
        entry
            .fields
            .iter()
            .find(|field| field.label == "Details")
            .unwrap()
            .value,
        "Rust, SQL"
    );
    assert_eq!(
        entry
            .fields
            .iter()
            .find(|field| field.label == "Extra")
            .unwrap()
            .value,
        "Hybrid"
    );
    assert!(
        entry
            .fields
            .iter()
            .find(|field| field.label == "__ort_body_paragraph__")
            .unwrap()
            .value
            .contains("Built tools")
    );
    verify_scaled_matrix(&bytes);
    let sidebar = extract_pdf(&bytes, &column_resume()).expect("column PDF parse");
    let sidebar_resume = ort_documents::resume_import::map_resume(&sidebar).unwrap();
    sidebar_resume
        .validate(ort_domain::DocumentLimits::default())
        .unwrap();
    assert_eq!(sidebar_resume.contact.full_name, "Jane Example");
    assert_eq!(sidebar_resume.sections.len(), 2);
    assert_eq!(sidebar_resume.sections[0].heading, "Skills");
    assert_eq!(
        sidebar_resume.sections[0].entries[0].fields[0].value,
        "Languages: Rust\nTools: Git, Docker"
    );
    assert_eq!(sidebar_resume.sections[1].heading, "Experience");
    assert_eq!(
        sidebar_resume.sections[1].entries[0].heading,
        "Example Company"
    );
    assert_eq!(sidebar_resume.sections[1].entries[0].subheading, "Engineer");
    assert_eq!(resume.sections[1].entries.len(), 1);
    assert_eq!(
        resume.sections[1].entries[0].fields[0].value,
        "Programming: Rust, TypeScript\nTools: Git, Docker"
    );

    println!(
        "PASS: bounded PDF extraction, six header slots, nonbulleted skills, visual reading order, columns, scanned-page denial, logo acceptance and malformed-source denial"
    );
}
fn verify_scaled_matrix(bytes: &[u8]) {
    let scaled = extract_pdf(bytes, &stream_pdf(
        b"BT /F1 1 Tf 14 0 0 14 72 720 Tm (Experience) Tj ET BT /F1 1 Tf 12 0 0 12 72 690 Tm (Example Company) Tj ET BT /F1 1 Tf 11 0 0 11 72 670 Tm (Engineer) Tj ET BT /F1 1 Tf 11 0 0 11 425 670 Tm (2020 - Present) Tj ET"
    )).expect("scaled text matrix");
    assert_eq!(scaled.blocks().len(), 4);
    assert_eq!(scaled.blocks()[0].layout.unwrap().font_size, 14_000);
    let scaled_resume = ort_documents::resume_import::map_resume(&scaled).unwrap();
    assert_eq!(scaled_resume.sections[0].heading, "Experience");
    assert_eq!(
        scaled_resume.sections[0].entries[0].heading,
        "Example Company"
    );
    assert_eq!(scaled_resume.sections[0].entries[0].subheading, "Engineer");
    assert_eq!(
        scaled_resume.sections[0].entries[0].date_range,
        "2020 - Present"
    );
}
fn synthetic_pdf() -> Vec<u8> {
    stream_pdf(b"BT /F1 18 Tf 72 720 Td (Experience) Tj 0 -24 Td (- Built safely) Tj ET")
}
fn positioned_resume() -> Vec<u8> {
    // Deliberately paint the body/right column before the left header. Reading
    // order must follow visual placement, not PDF content-stream order.
    let lines = [
        (72, 625, 11, "Built tools for distributed teams."),
        (465, 692, 11, "Boston, MA"),
        (420, 673, 11, "2022 - Present"),
        (465, 654, 11, "Hybrid"),
        (72, 720, 14, "Experience"),
        (72, 692, 12, "Example Company"),
        (215, 692, 11, "Rust, SQL"),
        (72, 673, 11, "Senior Engineer"),
        (72, 590, 14, "Skills"),
        (72, 562, 11, "Programming: Rust, TypeScript"),
        (72, 546, 11, "Tools: Git, Docker"),
    ];
    let stream = lines
        .iter()
        .fold(String::new(), |mut stream, (x, y, font, text)| {
            writeln!(stream, "BT /F1 {font} Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET").unwrap();
            stream
        });
    stream_pdf(stream.as_bytes())
}
fn column_resume() -> Vec<u8> {
    let lines = [
        (72, 750, 16, "Jane Example"),
        (72, 720, 14, "Skills"),
        (300, 720, 14, "Experience"),
        (72, 692, 11, "Languages: Rust"),
        (300, 692, 12, "Example Company"),
        (72, 676, 11, "Tools: Git, Docker"),
        (300, 673, 11, "Engineer"),
        (460, 673, 11, "2020 - 2024"),
        (300, 644, 11, "Built reliable services."),
    ];
    let stream = lines
        .iter()
        .fold(String::new(), |mut stream, (x, y, font, text)| {
            writeln!(stream, "BT /F1 {font} Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET").unwrap();
            stream
        });
    stream_pdf(stream.as_bytes())
}
fn stream_pdf(stream: &[u8]) -> Vec<u8> {
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

fn image_pdf(text: &str) -> Vec<u8> {
    let stream = format!("q 100 0 0 100 72 500 cm /Im Do Q BT /F1 18 Tf 72 720 Td ({text}) Tj ET");
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /XObject << /Im 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len()).into_bytes(),
        b"<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 1 >>\nstream\nX\nendstream".to_vec(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    pdf
}
