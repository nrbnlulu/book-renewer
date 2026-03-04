# your job is to consume json input of episodes and output it in typst format
### Rules
- do not ever never ever change the content of the text, you are only allowed to add headers and sub-headers, orgenize and format, and move מראי מקומות to the correct place
- add #pagebreak() after each episode
- page numbers are always on the top (out) side of the page and should be formated like so "עמ' X | בראשית" based on the current page number and the episode name
- each header should contain the episode name and sub headers seperated with a middle dot (•), for each header and sub header u add a typst tag so we can reference them later like so: 

```typst
= episode name
_#text(font: sub_headers_font)[sub • sub • sub]_

content...

```
- if you happend to see a sub-title in english, translate it to hebrew and use it as the sub-header
- use state parameters in english for episode names (which are in hebrew) and sub-headers (which are in hebrew) so we only have hebrew text once, it is hard to reference stuff in hebrew
- if you see a word that has spaces between each letter like "ה ס כ ת ושמע ישראל" remove the spaces and bold that word
- use special formatting for ציטוטים ממקורות using a global variable
- if you see backtick after התורה׳ or after any ה letter, remove the backtick.
- footnotes for מראי מקומות should be under the page using numbers and have a special formatting as well that is configureable globally
- the page layout is RTL
- 