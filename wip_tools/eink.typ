// 1404 x 1872 px → 1053pt x 1404pt
#let page-width = 1053pt
#let page-height = 1404pt
#set page(width: page-width, height: page-height, margin: 0pt)

#let logo-ratio = 0.3819 // something akin to the golden ratio
#let calendar-ratio = 1 - logo-ratio

#let logo-height = (page-height * logo-ratio)
#let calendar-height = (page-height * calendar-ratio)

// Load SVG logo
#let logo = image("logo.svg", width: page-width * 0.5)
// Event data: (relative, date, name, optional location)
#let events = (
  ("in 1·d", "04.26. 18:00", "Team Sync", "Room A"),
  ("in 2·d", "04.28. 14:15", "Client Demo", "Berlin HQ"),
  ("in 4·d", "04.29. 17:30", "Engineering Review", ""),
  ("in 6·d", "05.02. 12:15", "UX Testing Session", "User Lab"),
  ("in 1·mo", "05.31.", "Q2 Planning", "Zoom"),
  ("in 1·mo", "06.05.", "Dev Onboarding", "SF Office"),
  ("in 1·mo", "06.15.", "Company All-Hands", ""),
)
// Calculate dynamic spacing
#let event-count = events.len()
#let row-height = calendar-height / event-count
#let spacing = row-height * 0.15

#let event(rel, date, name, loc) = block()[#{
  text(size: row-height * 0.22, weight: "semibold", fill: gray)[#rel]
  text(size: row-height * 0.18, fill: gray)[#date]
  text(size: row-height * 0.26, weight: "medium")[#name]
  text(size: row-height * 0.22, fill: gray)[#loc]
}]

#block(width: page-width, height: page-height, clip: true)[
  // --- Top 40%: Centered logo
  #align(center)[
      #box(
        height: logo-height,
        inset: 0pt,
        [
          #align(horizon)[
                #logo
          ]
        ]
      )
  ]

  // --- Bottom 60%: Event list
  #align(center)[
    #box(
      height: calendar-height,
      inset: 0pt,
      clip: true
    )[
      #align(left)[
        #box()[
          #events.map(((rel, date, name, loc)) =>
            event(rel, date,name,loc)
          ).join[]
        ]
      ]
    ]
  ]
]
