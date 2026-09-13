// PDF dimensions follow the fixed 816px View page at 72/96 pt per CSS px.
// Structured content is data, never code; all fonts remain bundled.
#set document(title: "Resume", author: (), date: none)
#set page(paper: "us-letter", margin: (x: 44.25pt, y: 33.75pt))
#set text(font: ("Gelasio", "Liberation Serif"), size: 9.75pt, top-edge: .8em, bottom-edge: .2em, fill: rgb("171717"), lang: "en", fallback: false, ligatures: false)
#set par(spacing: 0pt, justify: false, leading: 6.045pt)
#let accent = rgb("243C5A")
#let rule = rgb("8A795F")
#show heading: it => it.body
#show heading.where(level: 1): set text(size: 22.5pt, weight: "regular")
#show heading.where(level: 2): set text(size: 10.5pt)
#show link: set text(fill: rgb("0A4F9E"))
#let content-inset = 10.5pt
#let row-inset = 3pt
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
#let entry-title(p) = layout(size => {
  let title = text(size: 10.125pt, rich(p.title_runs))
  if p.title_runs.len() == 0 or p.detail_runs.len() == 0 {
    if p.title_runs.len() == 0 { rich(p.runs) } else { title }
  } else if measure(title).width < size.width * .8 {
    grid(columns: (auto, auto, 1fr), column-gutter: 5.25pt,
      title, text(fill: rgb("596678"), "|"), text(fill: rgb("303B49"), rich(p.detail_runs)))
  } else {
    // Very long titles wrap naturally, while keeping all content extractable.
    rich(p.runs)
  }
})
#for p in json(bytes(sys.inputs.resume)) {
  if p.kind == "name" {
    align(center, block(inset: (top: 1.5pt), above: 0pt, below: 4pt, text(size: 22.5pt, tracking: .015em, weight: "regular", fill: accent, heading(level: 1, outlined: false, rich(p.runs)))))
  } else if p.kind == "contact" {
    align(center, block(inset: (top: 3pt, bottom: 3pt), above: 0pt, below: 12pt, rich(p.runs)))
  } else if p.kind == "section" {
    block(sticky: true, above: 15.75pt, below: 3.75pt, { text(size: 10.5pt, tracking: .035em, weight: "bold", fill: accent, heading(level: 2, outlined: true, rich(p.runs))); v(1.5pt); line(length: 100%, stroke: .75pt + rule) })
  } else if p.kind == "entry" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 3pt, below: if p.entry_end { 10.5pt } else { 0pt },
      pad(left: content-inset, grid(columns: (1fr, .28fr), align: (left, right), column-gutter: 10.5pt,
        stack(dir: ttb, spacing: 6.045pt, entry-title(p), ..(if p.subtitle_runs.len() > 0 { (rich(p.subtitle_runs),) } else { () })),
        rich(p.right_runs),
      ))
    )
  } else if p.kind == "subrow" or p.kind == "meta" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 0pt, below: if p.entry_end { 10.5pt } else { 0pt },
      pad(left: content-inset, grid(columns: (1fr, .28fr), align: (left, right), column-gutter: 10.5pt,
        rich(p.runs), rich(p.right_runs),
      ))
    )
  } else if p.kind == "bullet" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: if p.body_start { 6pt } else { 0pt }, below: if p.entry_end { 10.5pt } else { .75pt }, pad(left: content-inset, list(tight: true, indent: 13.5pt, body-indent: 10.5pt, rich(p.runs))))
  } else if p.kind == "link" {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 1pt, below: 2pt, pad(left: content-inset, rich(p.runs)))
  } else {
    block(sticky: p.sticky, inset: (top: row-inset, bottom: row-inset), above: 2pt, below: 2pt, pad(left: content-inset, rich(p.runs)))
  }
}
