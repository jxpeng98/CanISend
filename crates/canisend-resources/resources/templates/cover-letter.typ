#let application_cover_letter(candidate_name, institution, body) = {
  set text(size: 11pt)
  align(right)[#candidate_name]
  v(1em)
  [Dear Selection Committee at #institution,]
  v(1em)
  body
}
