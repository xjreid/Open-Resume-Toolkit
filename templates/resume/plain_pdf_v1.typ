// Original plain fixture. No external resources, packages, eval, or user markup.
#set document(title: "Resume", author: (), date: none)
#set page(paper: "us-letter", margin: 1in)
#set text(font: "Libertinus Serif", size: 11pt, lang: "en", fallback: false, ligatures: false)
#set par(justify: false, leading: 3pt)
#show link: set text(fill: rgb("0A4F9E"))
#set heading(numbering: none)
#show heading.where(level: 1): set text(size: 12pt, weight: "bold")
#show heading.where(level: 2): set text(size: 11pt, weight: "bold")
#let content-inset = 10pt
#let literal(value) = {
  for (index, part) in value.split("\n").enumerate() {
    if index > 0 { linebreak() }
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
    if run.url == none { content } else { link(run.url, underline(content)) }
  }
}
#let paragraphs = json(bytes(sys.inputs.resume))
#for p in paragraphs {
  if p.kind == "name" {
    block(above: 0pt, below: 8pt, text(size: 18pt, weight: "bold", rich(p.runs)))
  } else if p.kind == "section" {
    heading(level: 1, rich(p.runs))
  } else if p.kind == "entry" {
    block(sticky: p.sticky, above: 4pt, below: 1pt,
      pad(left: content-inset, grid(columns: (1fr, 23%), align: (left, right), column-gutter: 10.5pt,
        rich(p.runs), rich(p.right_runs),
      ))
    )
  } else if p.kind == "subrow" or p.kind == "meta" {
    block(sticky: p.sticky, above: 0pt, below: 1pt,
      pad(left: content-inset, grid(columns: (1fr, 23%), align: (left, right), column-gutter: 10.5pt,
        rich(p.runs), rich(p.right_runs),
      ))
    )
  } else if p.kind == "bullet" {
    block(sticky: p.sticky, above: 0pt, below: 3pt, pad(left: content-inset, list(tight: true, indent: 10pt, body-indent: 12pt, rich(p.runs))))
  } else if p.kind == "link" {
    block(sticky: p.sticky, above: 1pt, below: 1pt, pad(left: content-inset, rich(p.runs)))
  } else {
    block(sticky: p.sticky, above: 1pt, below: 2pt, pad(left: content-inset, rich(p.runs)))
  }
}
