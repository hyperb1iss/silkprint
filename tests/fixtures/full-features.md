---
title: Full Feature Showcase
subtitle: Every Supported Markdown Feature
author: Stefanie Jane
date: 2025-06-15
theme: silk-light
---

# Heading Level 1

## Heading Level 2

### Heading Level 3

#### Heading Level 4

##### Heading Level 5

###### Heading Level 6

## Text Formatting

This paragraph has **bold**, *italic*, ***bold italic***, ~~strikethrough~~,
`inline code`, and ==highlighted== text. You can also do **bold with *nested italic*
inside** and *italic with **nested bold** inside*.

## Links

An inline link: [SilkPrint on GitHub](https://github.com/hyperb1iss/silkprint)

An autolink: <https://example.com>

A reference link: [Typst documentation][typst-docs]

[typst-docs]: https://typst.app/docs "Typst Documentation"

## Images

![SilkPrint Logo](./img/logo.png "SilkPrint")

Reference-style image:

![Architecture][arch-img]

[arch-img]: ./img/architecture.png "Pipeline Architecture"

## Unordered List

- First item with **bold**
- Second item with *italic*
  - Nested item A
  - Nested item B
    - Deep nested
- Third item

## Ordered List

1. Parse Markdown
2. Resolve theme
3. Emit Typst markup
4. Compile to PDF
5. Write output

## Task List

- [x] Markdown parsing
- [x] Theme engine
- [x] Typst emission
- [ ] HTML output
- [ ] Live preview

## Description List

SilkPrint
: Markdown to PDF converter

Typst
: Modern typesetting engine

comrak
: GFM-compatible Markdown parser

## Code Block

```rust
fn main() {
    let theme = Theme::load("silk-light").expect("theme not found");
    let pdf = silkprint::render("# Hello", &theme).expect("render failed");
    std::fs::write("output.pdf", pdf).expect("write failed");
}
```

## Blockquote

> "Any sufficiently advanced technology is indistinguishable from magic."
>
> --- Arthur C. Clarke

## Table

| Theme | Variant | Print-Safe | Description |
|-------|---------|:----------:|-------------|
| silk-light | light | Yes | Clean, warm, professional |
| silk-dark | dark | No | Deep navy-black elegance |
| manuscript | light | Yes | Old-world serif feel |
| monochrome | light | Yes | Pure black on white |

## Horizontal Rule

---

## Footnotes

SilkPrint supports footnotes[^1] for academic and technical writing.
Multiple references[^2] work independently.

[^1]: Footnotes are rendered using Typst's native `#footnote()` function.

[^2]: Each footnote is automatically numbered and placed at the page bottom.

## Alerts

> [!NOTE]
> This is a note callout for supplementary information.

> [!TIP]
> Use `--theme` to switch between 40 built-in themes.

> [!IMPORTANT]
> Always use `serde_yaml_ng` instead of `serde_yml`.

> [!WARNING]
> The `--force` flag overwrites files without confirmation.

> [!CAUTION]
> Do not use `unsafe` code --- it is forbidden in this project.

## Math (Typst-Native)

Inline math: $x^2 + y^2 = z^2$ and $sum_(i=1)^n i = (n(n+1))/2$.

Display math:

$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $

## Emoji Shortcodes

This feature is :sparkles: amazing :rocket: and we :heart: it!

## Highlighted Text

This sentence has ==highlighted words== that should render with a
colored background.

## Wikilinks

See [[getting-started]] for setup instructions, or check the
[[api-reference|API Reference]] for detailed usage.

## Superscript and Subscript

Water is H~2~O. Einstein's equation: E = mc^2^.

---

*End of full feature showcase.*
