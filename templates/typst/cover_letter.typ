// Base cover letter template for CanISend generated source files.
// Generated job-specific sources should read cover_letter_content.json.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#let cover-letter(content) = [
  #show: coverletter.with(
    font-type: "PT Serif",
    margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
    name: [Applicant Name],
    address: [],
    salutation: [#content.salutation],
    contacts: (),
    recipient: (
      start-title: [#content.recipient.start_title],
      cl-title: [#content.recipient.cl_title],
      date: [#content.recipient.date],
      department: [#content.recipient.department],
      institution: [#content.recipient.institution],
      address: [#content.recipient.address],
      postcode: [#content.recipient.postcode],
    ),
  )

  #content.opening

  == Research Fit
  #content.sections.research_fit

  == Teaching Fit
  #content.sections.teaching_fit

  == Departmental Contribution
  #content.sections.departmental_contribution

  == Service and Leadership
  #content.sections.service_leadership

  #content.closing
]
