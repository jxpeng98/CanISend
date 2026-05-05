// Base application package template for AAP Copilot generated source files.
// Generated job-specific sources should read application_package_content.json.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#let application-package(package) = [
  #show: statement.with(
    font-type: "PT Serif",
    margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
    name: [Applicant Name],
    address: [],
    contacts: (),
  )

  = Application Package

  == Job Information
  - Title: #package.job.title
  - Institution: #package.job.institution
  - Department: #package.job.department
  - Deadline: #package.job.deadline
  - Application URL: #package.job.application_url

  == Fit Report
  #package.fit_report

  == Cover Letter Draft
  #package.cover_letter

  == CV Tailoring Notes
  #package.cv_tailoring_notes

  == Criteria Checklist
  #package.criteria_checklist
]
