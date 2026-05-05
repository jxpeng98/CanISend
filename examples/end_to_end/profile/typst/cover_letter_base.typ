#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#show: coverletter.with(
  font-type: "PT Serif",
  name: [Example Applicant],
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
