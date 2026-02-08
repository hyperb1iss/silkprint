---
title: Edge Cases
author: SilkPrint Test Suite
---

# Edge Cases

## Empty Heading

#

## Heading with Only Inline Code

### `render_to_typst()`

## Extremely Long Single Line

This is an extremely long single line that goes on and on and on without any line breaks to test how the rendering engine handles very long paragraphs that exceed the typical line width of a PDF page and should trigger line wrapping, justification, and hyphenation logic in the Typst compiler, and it just keeps going because we need to ensure that edge cases like this are handled gracefully without any overflow or clipping artifacts.

## Deeply Nested Blockquotes

> Level 1 blockquote
> > Level 2 blockquote
> > > Level 3 blockquote
> > > > Level 4 blockquote
> > > > > Level 5 blockquote --- how deep can we go?

## Very Large Table

| C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 | C9 | C10 | C11 | C12 |
|----|----|----|----|----|----|----|----|----|-----|-----|-----|
| a1 | a2 | a3 | a4 | a5 | a6 | a7 | a8 | a9 | a10 | a11 | a12 |
| b1 | b2 | b3 | b4 | b5 | b6 | b7 | b8 | b9 | b10 | b11 | b12 |
| c1 | c2 | c3 | c4 | c5 | c6 | c7 | c8 | c9 | c10 | c11 | c12 |

## Empty Document Section

The following section intentionally has no content between the headings.

##

## Consecutive Horizontal Rules

---

---

---

## Special Characters

Ampersand: AT&T, R&D, Q&A

Angle brackets: 5 < 10, 10 > 5, `<div>` tag

Backslash: C:\Users\test\file.txt

Pipes: true | false

Quotes: "double" and 'single' and "smart quotes"

HTML entities: &amp; &lt; &gt; &quot; &#39; &copy; &mdash;

Asterisks without emphasis: 2 * 3 = 6

Underscores in words: some_variable_name, __dunder__, _single_

Hash without heading: The color #ff6ac1 is coral.

## Trailing Whitespace

This line has trailing spaces.

This line has a trailing tab.

## Only Whitespace in Code Block

```

```

## Adjacent Formatting

***bold italic*** right next to ~~strikethrough~~ and `code` and [link](https://example.com).

## Unusual Nesting

> Blockquote containing a list:
>
> 1. First
> 2. Second
>    - Nested bullet
>    - With `code`
>
> And a code block:
>
> ```python
> print("hello from inside a blockquote")
> ```
>
> And a table:
>
> | A | B |
> |---|---|
> | 1 | 2 |
