#let canisend_render_document(data) = {
  set document(title: data.title)
  set page(
    paper: "a4",
    margin: (top: 22mm, right: 24mm, bottom: 22mm, left: 24mm),
  )
  set text(font: "Libertinus Serif", size: 10.5pt)
  set par(justify: true, leading: 0.65em)
  show heading.where(level: 2): it => block(
    above: 1.1em,
    below: 0.45em,
    text(size: 12pt, weight: "semibold", it.body),
  )

  align(center, text(size: 16pt, weight: "semibold", data.title))
  v(0.35em)
  align(center, text(size: 8pt, fill: luma(45%), data.kind))
  v(1.2em)

  for section in data.sections {
    if section.heading != none {
      heading(level: 2, outlined: false, section.heading)
    }
    section.body
    parbreak()
  }
}
