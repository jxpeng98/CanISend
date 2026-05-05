#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-cv:1.3.0": *

#show: cv-single.with(
  font-type: "PT Serif",
  name: [Example Applicant],
  address: [],
  contacts: (),
)

#section("Education")
#education(institution: [Example University], major: [Economics], degree: [PhD], date: [2026])

#section("Teaching")
#job(position: [Teaching Fellow in Econometrics], institution: [Example University], date: [2024--2026])

#section("Research")
+ @example-applied-economics-2026
