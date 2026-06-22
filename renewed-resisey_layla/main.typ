#import "utils.typ": q

#set page(
  width: 4.25in, height: 6.87in,
  margin: (inside: 0.6in, outside: 0.4in, top: 0.6in, bottom: 0.4in),
  header: context {
    let p_num = counter(page).get().first()
    let headings = query(selector(heading.where(level: 1)).before(here()))
    let is_new_ep = query(heading.where(level: 1)).any(h => h.location().page() == p_num)
    if headings != () and not is_new_ep {
      let current_episode = headings.last().body
      if calc.even(p_num) { [#p_num #h(1fr) #current_episode] }
      else { [#current_episode #h(1fr) #p_num] }
    }
  }
)

#set text(font: "David Libre", size: 9pt, lang: "he")
#set par(justify: true, first-line-indent: 1.2em, leading: 0.55em)
#show regex("\[.*?\]"): it => { set text(size: 0.75em, fill: gray.darken(50%)); it }
#show heading.where(level: 1): it => {
  pagebreak(weak: true, to: "odd")
  set align(center); set text(size: 14pt); block(above: 12%, below: 0.6in, it.body)
}
#show heading.where(level: 2): it => {
  set align(right); set text(size: 10pt, weight: "bold"); block(above: 1.2em, below: 0.6em, it.body)
}

#include "typst/episode_001.typ"
#include "typst/episode_002.typ"
#include "typst/episode_003.typ"
#include "typst/episode_004.typ"
#include "typst/episode_005.typ"
#include "typst/episode_006.typ"
#include "typst/episode_007.typ"
#include "typst/episode_008.typ"
#include "typst/episode_009.typ"
#include "typst/episode_010.typ"
#include "typst/episode_011.typ"
#include "typst/episode_012.typ"
#include "typst/episode_013.typ"
#include "typst/episode_014.typ"
#include "typst/episode_015.typ"
#include "typst/episode_016.typ"
#include "typst/episode_017.typ"
#include "typst/episode_018.typ"
#include "typst/episode_019.typ"
#include "typst/episode_020.typ"
#include "typst/episode_021.typ"
#include "typst/episode_022.typ"
#include "typst/episode_023.typ"
#include "typst/episode_024.typ"
#include "typst/episode_025.typ"
#include "typst/episode_026.typ"
#include "typst/episode_027.typ"
#include "typst/episode_028.typ"
#include "typst/episode_029.typ"
#include "typst/episode_030.typ"
#include "typst/episode_031.typ"
#include "typst/episode_032.typ"
#include "typst/episode_033.typ"
#include "typst/episode_034.typ"
#include "typst/episode_035.typ"
#include "typst/episode_036.typ"
#include "typst/episode_037.typ"
#include "typst/episode_038.typ"
#include "typst/episode_039.typ"
#include "typst/episode_040.typ"
#include "typst/episode_041.typ"
#include "typst/episode_042.typ"
#include "typst/episode_043.typ"
#include "typst/episode_044.typ"
#include "typst/episode_045.typ"
#include "typst/episode_046.typ"
#include "typst/episode_047.typ"
#include "typst/episode_048.typ"
#include "typst/episode_049.typ"
#include "typst/episode_050.typ"
#include "typst/episode_051.typ"
#include "typst/episode_052.typ"
#include "typst/episode_053.typ"
#include "typst/episode_054.typ"
#include "typst/episode_055.typ"
#include "typst/episode_056.typ"
#include "typst/episode_057.typ"
#include "typst/episode_058.typ"
#include "typst/episode_059.typ"
#include "typst/episode_060.typ"
#include "typst/episode_061.typ"
#include "typst/episode_062.typ"
#include "typst/episode_063.typ"
#include "typst/episode_064.typ"
#include "typst/episode_065.typ"
#include "typst/episode_066.typ"
#include "typst/episode_067.typ"
#include "typst/episode_068.typ"
#include "typst/episode_069.typ"
