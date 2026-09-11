// Original ORT professional layout. Structured content is data, never code.
#set document(title: "Resume", author: (), date: none)
#set page(paper: "us-letter", margin: .85in)
#set text(font: "Libertinus Serif", size: 11pt, lang: "en", fallback: false, ligatures: false)
#set par(spacing: 0pt, justify: false, leading: 2.8pt)
#let accent = rgb("243C5A")
#let rule = rgb("8A795F")
#let literal(value) = {
  for (i, part) in value.split("\n").enumerate() {
    if i > 0 { linebreak() }
    text(part.replace("\t", "    "))
  }
}
#let rich(runs) = {
  for run in runs {
    let content = text(
      weight: if run.bold { "bold" } else { "regular" },
      style: if run.italic { "italic" } else { "normal" },
      literal(run.text),
    )
    if run.url == none { content } else { link(run.url, content) }
  }
}
#for p in json(bytes(sys.inputs.resume)) {
  if p.kind == "name" {
    align(center, block(above: 0pt, below: 4pt, text(size: 22pt, weight: "bold", fill: accent, rich(p.runs))))
  } else if p.kind == "contact" {
    align(center, block(above: 0pt, below: 3pt, text(size: 9.5pt, rich(p.runs))))
  } else if p.kind == "section" {
    block(sticky: true, above: 13pt, below: 4pt, { text(size: 11pt, weight: "bold", fill: accent, rich(p.runs)); v(2pt); line(length: 100%, stroke: .5pt + rule) })
  } else if p.kind == "entry" {
    block(sticky: true, above: 5pt, below: 1pt,
      grid(columns: (1fr, 30%), align: (left, right), column-gutter: 12pt,
        rich(p.runs),
        text(size: 10pt, rich(p.right_runs)),
      )
    )
  } else if p.kind == "subrow" or p.kind == "meta" {
    block(above: 0pt, below: 1pt,
      grid(columns: (1fr, 30%), align: (left, right), column-gutter: 12pt,
        rich(p.runs), text(size: 10pt, rich(p.right_runs)),
      )
    )
  } else if p.kind == "bullet" {
    block(above: 0pt, below: 1pt, list(tight: true, indent: 0pt, body-indent: 12pt, rich(p.runs)))
  } else if p.kind == "link" {
    block(above: 1pt, below: 1pt, text(size: 9.5pt, rich(p.runs)))
  } else {
    block(above: 2pt, below: 1pt, rich(p.runs))
  }
}
