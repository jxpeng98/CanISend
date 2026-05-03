// Base cover letter template for AAP Copilot generated source files.
// Uses the public Typst Universe package maintained outside this project.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#let cover-letter(body) = [
  #show: coverletter.with(
    font-type: "PT Serif",
    margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
    name: [Applicant Name],
    address: [],
    salutation: [Yours sincerely,],
    contacts: (),
    recipient: (
      start-title: [Dear Selection Committee,],
      cl-title: [Academic Job Application],
      date: [],
      department: [],
      institution: [],
      address: [],
      postcode: [],
    ),
  )
  #body
]
