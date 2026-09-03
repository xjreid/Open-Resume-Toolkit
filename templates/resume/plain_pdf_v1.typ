// Original plain fixture. No external resources, packages, eval, or user markup.
#set document(title: "Resume", author: (), date: none)
#set page(paper: "us-letter", margin: 1in)
#set text(font: "Libertinus Serif", size: 11pt, lang: "en", fallback: false, ligatures: false)
#set par(justify: false, leading: 3pt)
#set heading(numbering: none)
#show heading.where(level: 1): set text(size: 12pt, weight: "bold")
#show heading.where(level: 2): set text(size: 11pt, weight: "bold")
#let literal(value) = {
  for (index, part) in value.split("\n").enumerate() {
    if index > 0 { linebreak() }
    text(part.replace("\t", "    "))
  }
}
#let paragraphs = json(bytes(sys.inputs.resume))
#for p in paragraphs {
  if p.kind == "name" {
    block(above: 0pt, below: 8pt, text(size: 18pt, weight: "bold", literal(p.text)))
  } else if p.kind == "section" {
    heading(level: 1, literal(p.text))
  } else if p.kind == "entry" {
    heading(level: 2, literal(p.text))
  } else if p.kind == "bullet" {
    list(tight: true, indent: 0pt, body-indent: 12pt, literal(p.text))
  } else if p.kind == "link" {
    block(above: 0pt, below: 4pt, link(p.url, literal(p.text)))
  } else {
    block(above: 0pt, below: 4pt, literal(p.text))
  }
  v(6pt, weak: false)
}
