// Original ORT template. Resume content is decoded as data, never evaluated.
#set document(title: "Resume", author: (), date: none)
#set page(paper: "us-letter", margin: .85in)
#set text(font: "Libertinus Serif", size: 10.5pt, lang: "en", fallback: false, ligatures: false)
#set par(justify: false, leading: 2pt)
#set heading(numbering: none)
#show heading.where(level: 1): set text(size: 11pt, weight: "bold", fill: rgb("000000"))
#show heading.where(level: 2): set text(size: 10.5pt, weight: "bold")
#let literal(value) = {
  for (i, part) in value.split("\n").enumerate() {
    if i > 0 { linebreak() }
    text(part.replace("\t", "    "))
  }
}
#let paragraphs = json(bytes(sys.inputs.resume))
#for p in paragraphs {
  if p.kind == "name" {
    block(text(size: 17pt, weight: "bold", fill: rgb("000000"), literal(p.text)))
  } else if p.kind == "section" {
    heading(level: 1, literal(p.text))
  } else if p.kind == "entry" {
    heading(level: 2, literal(p.text))
  } else if p.kind == "bullet" {
    list(indent: 0pt, body-indent: 12pt, literal(p.text))
  } else if p.kind == "link" {
    block(link(p.url, literal(p.text)))
  } else {
    block(literal(p.text))
  }
  v(2pt, weak: false)
}
