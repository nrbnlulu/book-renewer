#import "../utils.typ": q

// pdf-pages: 129

*   The raw text provided *explicitly* starts with `= רסיסי לילה`.
        *   The previous episode title provided was `רסיסי לילה - מלכות רומא`.
        *   Is "רסיסי לילה" a new chapter or just a heading for the same sequence? In many Sefarim, there are sub-headings.
        *   Let's look at the content: "עם ישראל כבסנם...". This continues the "water" (מיא) theme from the end of the previous page.
        *   Instruction: "אם קיבלת תוכן Typst של פרק שטרם הסתיים... בדוק אם הטקסט החדש ממשיך את אותו הפרק מבחינה תוכנית. אם כן — אל תכתוב כותרת = חדשה".
        *   However, the user provided a snippet that *already* had `// END_EPISODE רסיסי לילה - מלכות רומא`. This usually implies the previous block is finished. But the prompt says "You are currently mid-episode".
        *   Let's assume it's a continuation because the text "עם ישראל כבסנם" follows "ושתא למיא".

    *   `עם ישראל כבסנם` -> `/* typo: אולי צ"ל כבשנם */` (Note: "כבשנם" makes sense in the context of fire/water).
    *   `ר"ל` -> `/* typo: רוצה לומר */` (Instruction says handle typos, but standard abbreviations like ר"ל are usually left or expanded. I'll follow the provided raw text's hint if it's there, but the user put them in the raw text block. I should format them as per instructions: `/* typo: ... */`).
    *   `וכש"כ` -> `/* typo: וכל שכן */`.
    *   `ז"א` -> (Not explicitly noted in raw but usually means זהו אומר/זהו שאמר).
    *   `#q[די כבה לנורא]` -> Quote.
    *   `#q[ואתא השוחט]` -> Quote.
    *   `[1,914]` -> Reference/Parentheses.

    *   No Boilerplate.
    *   Keep continuity if same episode.
    *   The raw text has a `=` title. If I decide it's a continuation, I skip the `=`. If it's a new chapter, I close the old and start new.
    *   Given the raw text starts with `= רסיסי לילה`, and the previous was `= רסיסי לילה - מלכות רומא`, it looks like a broader category or a new chapter. However, "רסיסי לילה" is the name of the book/section by R. Shimshon Ostropoli or similar style.
    *   Let's look at the flow: `ושתא למיא שבלעו.` (Previous) `עם ישראל כבסנם` (New). It's a direct sentence continuation. Therefore, I will NOT write the `=` header. I will treat it as continuation.

    *Wait*, the prompt raw text has `// END_EPISODE רסיסי לילה` at the bottom of the *raw* section. This is confusing. It seems the user provided text that already has some formatting. I should clean it up and follow the instructions.

    *   Skip the `=` header.
    *   Start with text.
    *   Quotes in `#q[]`.
    *   Typos in `/* typo: ... */`.
    *   Subtitle `== השתלשלות הגלויות ורמזי חד גדיא`.
    *   Subtitle `== ואתא השוחט ומלכות גרמניה`.
    *   Ending comment: `// END_EPISODE רסיסי לילה - מלכות רומא` (Since it's continuing that one).

    Wait, the raw text starts with a sentence fragment.
    Text: "עם ישראל כבסנם /* typo: אולי צ"ל כבשנם */ כמו מים. והגם שהרומאים אבד זכרם הניחו אותנו ליורשיהם..."
    This is definitely the same chapter.

    *Refining Typos:*
    The raw text already has some `/* typo: ... */` comments. I will preserve and format them.

    *Quote check:*
    "די כבה לנורא" -> `#q[די כבה לנורא]`
    "די הכה לכלבא" -> `#q[די הכה לכלבא]`
    "די נשך לשונרא" -> `#q[די נשך לשונרא]`
    "די אכלה לגדיא" -> `#q[די אכלה לגדיא]`
    "די זבין אבא" -> `#q[די זבין אבא]`
    "ואתא השוחט" -> `#q[ואתא השוחט]`

    *Note on Subtitles:*
    The raw text has:
    `== השתלשלות הגלויות ורמזי חד גדיא`
    `== ואתא השוחט ומלכות גרמניה`
    I will include these.

    *Handling the end:*
    The raw text ends mid-sentence: "ובעת מלחמת".
    I will end with `// END_EPISODE רסיסי לילה - מלכות רומא`.

    *Wait, a contradiction:*
    The raw text provided by the user *ends* with `// END_EPISODE רסיסי לילה`.
    The instruction says: "סיומת חובה: בסוף הקובץ, הוסף הערת קוד לפרסור: `