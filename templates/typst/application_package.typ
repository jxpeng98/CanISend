// Base application package template for AAP Copilot generated source files.
// Cover-letter components come from the public modernpro-coverletter package.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#let application-package(body) = [
  #show: statement.with(
    font-type: "PT Serif",
    margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
    name: [Applicant Name],
    address: [],
    contacts: (),
  )
  #body
]
