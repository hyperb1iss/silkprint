---
title: Description List Showcase
author: SilkPrint Test Suite
---

# Description Lists

## Basic Description List

SilkPrint
: A Rust CLI tool that converts Markdown into beautifully typeset PDFs.

Typst
: A modern typesetting system designed as an alternative to LaTeX,
  providing professional typography with a simpler syntax.

comrak
: A CommonMark and GFM compatible Markdown parser written in Rust,
  supporting extensions like footnotes, math, and alerts.

## Multiple Definitions per Term

Theme Variant
: **Light** --- Designed for print and bright displays. Uses dark text on light backgrounds.
: **Dark** --- Designed for screen reading. Uses light text on dark backgrounds.

Font Style
: *Regular* (weight 400)
: *Medium* (weight 500)
: *SemiBold* (weight 600)
: *Bold* (weight 700)

## Terms with Inline Formatting

**Bold Term**
: A term that is bold to draw attention.

*Italic Term*
: A term rendered in italic style.

`Code Term`
: A term rendered as inline code, useful for API references.

## Long Definitions

Pipeline Architecture
: SilkPrint processes documents through a multi-stage pipeline. First, the
  Markdown source is parsed into an AST using comrak with all extensions enabled.
  Next, the theme is resolved from either a built-in name or a custom TOML file.
  The AST is then walked to emit Typst markup, applying theme styling as set/show
  rules. Finally, the Typst source is compiled through a custom World trait
  implementation into a PDF with embedded fonts and metadata.

## Nested Content in Definitions

Error Handling
: SilkPrint uses `thiserror` for typed error definitions and `miette` for
  rich diagnostic output. The error types include:

  - `IoError` --- File read/write failures
  - `ParseError` --- Markdown or TOML parse failures
  - `ThemeError` --- Invalid theme configuration
  - `CompileError` --- Typst compilation failures

## Single-Item Description List

Monochrome Theme
: Pure black on white, zero color, maximum ink efficiency. Ideal for
  formal documents and professional printing.
