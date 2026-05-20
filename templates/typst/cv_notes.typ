// Base CV/CV-notes template for CanISend generated source files.
// Uses the public Typst Universe package maintained outside this project.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-cv:1.3.0": *

#let cv-notes(body) = [
  #show: cv-single.with(
    font-type: "PT Serif",
    continue-header: "false",
    margin: (left: 1.75cm, right: 1.75cm, top: 2cm, bottom: 2cm),
    name: [Applicant Name],
    address: [],
    lastupdated: "true",
    pagecount: "true",
    contacts: (),
  )
  #body
]
