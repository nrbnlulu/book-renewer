
your task is to consume text pages and organize them into a JSON format.
you will be given an input in this format (JSON):
```json
{
    "current_episode": {
    "name": "<episode name>",
    "content": "<content>"
    "sub_headers": ["<sub-topic 1>", "<sub-topic 2>"]
    },
    "new_raw_text": "<new raw text>"
}
```
note that current_episode is optional and can be null if there is no previous episode.

### possible outputs (JSON)
a list of JSON entries, each representing a new episode or appending to the last episode.
```json
[
    {
    "type": "append_last_episode",
    "name": "<name of the episode being appended to>",
    "data": "<more content>",
    "new_topics": ["<sub-topic 1>", "<sub-topic 2>"]
    },
    {
    "type": "new_episode",
    "name": "<episode name>",
    "data": "<new content>",
    "new_topics": ["<sub-topic 1>", "<sub-topic 2>"]
    }
]
```

### Definition of an episode
- an episode is a פרשה like בראשית, נח, לך לך, וירא etc... there can also be episodes for הקדמה \ פתיחה
- `new_topics` is a list of short phrases for the sub-topics introduced by the new text in this action. Only include topics from the new content — not the full accumulated list.
- note that you MUST not change the text at all (unless u see text that is not in the core context of the book, like page numbers, headers, copy rights etc..), just group it based on episodes context
- note that not every דיבור המתחיל is a new sub-topic always look at the context.
- if there is no episodes (like it is an introduction AKA הסכמות) don't give anything usually הסכמות are in the start of the book.

### IMPORTANT: JSON String Escaping Rules
All string values MUST be valid JSON strings. This means:
- Newlines must be escaped as \n (the two characters: backslash and 'n')
- Tabs must be escaped as \t
- Backslashes must be escaped as \\
- Double quotes must be escaped as \"
- All other text should remain as-is (Hebrew, Aramaic, etc. are fine)

Example of correct escaping:
```json
{
    "data": "first line\nsecond line\nthird line"
}
```

NOT like this (invalid - literal newlines in string):
```json
{
    "data": "first line
second line
third line"
}
```
