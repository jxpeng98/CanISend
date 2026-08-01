#let application_cover_letter(candidate_name, institution, body) = {
  set page(
    paper: "a4",
    margin: (top: 24mm, right: 24mm, bottom: 24mm, left: 24mm),
  )
  set text(font: "Libertinus Serif", size: 10.5pt)
  set par(justify: true, leading: 0.65em)
  align(right)[#candidate_name]
  v(1em)
  [Dear Selection Committee at #institution,]
  v(1em)
  body
}

#let canisend_render_document(data) = {
  set document(title: data.title)
  set page(
    paper: "a4",
    margin: (top: 24mm, right: 24mm, bottom: 24mm, left: 24mm),
  )
  set text(font: "Libertinus Serif", size: 10.5pt)
  set par(justify: true, leading: 0.65em)
  show heading.where(level: 2): it => block(
    above: 0.9em,
    below: 0.35em,
    text(size: 11pt, weight: "semibold", it.body),
  )

  text(size: 14pt, weight: "semibold", data.title)
  v(0.25em)
  line(length: 100%, stroke: 0.5pt + luma(75%))
  v(1.1em)

  for section in data.sections {
    if section.heading != none {
      heading(level: 2, outlined: false, section.heading)
    }
    section.body
    parbreak()
  }
}
