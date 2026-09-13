// PDF dimensions follow the fixed 816px View page at 72/96 pt per CSS px.
// Structured content is data, never code; all fonts remain bundled.
#set document(title: "Resume", author: (), date: none)
#set page(paper: "us-letter", margin: (x: 38.25pt, y: 33.75pt))
#set text(font: "Libertinus Serif", size: 10.125pt, top-edge: .8em, bottom-edge: .2em, fill: rgb("171717"), lang: "en", fallback: false, ligatures: false)
#set par(spacing: 0pt, justify: false, leading: 2.2275pt)
#show link: set text(fill: rgb("0A4F9E"))
#let accent = rgb("000000")
#let content-inset = 10.5pt
#let row-inset = 2pt
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
    if run.url == none { content } else { link(run.url, underline(content)) }
  }
}
#for p in json(bytes(sys.inputs.resume)) {
  if p.kind == "name" {
    align(center, block(above: 0pt, below: 4pt, text(size: 22.5pt, tracking: .015em, weight: "bold", fill: accent, rich(p.runs))))
  } else if p.kind == "contact" {
    align(center, block(inset: (top: 3pt, bottom: 3pt), above: 0pt, below: 9pt, rich(p.runs)))
  } else if p.kind == "section" {
    block(sticky: true, above: 15.75pt, below: 3.75pt, { text(size: 11.25pt, weight: "semibold", tracking: .025em, fill: accent, rich(p.runs)); v(1.5pt); line(length: 100%, stroke: .75pt + accent) })
  } else if p.kind == "entry" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 3pt, below: if p.entry_end { 7.5pt } else { 0pt },
      pad(left: content-inset, grid(columns: (1fr, .28fr), align: (left, right), column-gutter: 10.5pt,
        rich(p.runs),
        rich(p.right_runs),
      ))
    )
  } else if p.kind == "subrow" or p.kind == "meta" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 0pt, below: if p.entry_end { 7.5pt } else { 0pt },
      pad(left: content-inset, grid(columns: (1fr, .28fr), align: (left, right), column-gutter: 10.5pt,
        rich(p.runs), rich(p.right_runs),
      ))
    )
  } else if p.kind == "bullet" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: if p.body_start { 6pt } else { 0pt }, below: if p.entry_end { 7.5pt } else { .75pt }, pad(left: content-inset, list(tight: true, indent: 18pt, body-indent: 9pt, rich(p.runs))))
  } else if p.kind == "link" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 1pt, below: 2pt, pad(left: content-inset, rich(p.runs)))
  } else {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 2pt, below: 2pt, pad(left: content-inset, rich(p.runs)))
  }
}
