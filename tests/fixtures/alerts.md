---
title: Alert Showcase
author: SilkPrint Test Suite
---

# Alerts

GitHub-style alert callouts using the `> [!TYPE]` syntax.

## Note

> [!NOTE]
> This is a **note** alert. Notes are used to highlight information that
> users should take into account, even when skimming.
>
> They can contain *multiple paragraphs* of text.

## Tip

> [!TIP]
> Use the `--theme` flag to switch between built-in themes:
>
> ```bash
> silkprint input.md --theme silk-dark -o output.pdf
> ```
>
> There are **40 built-in themes** across 8 aesthetic families.

## Important

> [!IMPORTANT]
> The `comrak` extension field is called `alerts`, **not** `admonitions`.
> Using the wrong field name will silently disable alert parsing.
>
> Always verify your extension configuration with:
>
> ```rust
> let mut options = Options::default();
> options.extension.alerts = true;
> ```

## Warning

> [!WARNING]
> Do *not* use `serde_yml` for YAML front matter parsing.
> It has an active **RustSec advisory** (RUSTSEC-2025-0068).
>
> Use `serde_yaml_ng` instead --- it's a maintained fork with
> the same API surface.

## Caution

> [!CAUTION]
> Running `silkprint` with `--force` will **overwrite** existing output
> files without confirmation. This action is *irreversible*.
>
> Always double-check your `-o` path before using `--force`.

## Alert with Complex Content

> [!NOTE]
> Alerts can contain various formatting:
>
> - **Bold** items in a list
> - *Italic* items
> - `Code spans` inline
> - [Links](https://example.com) too
>
> And even tables:
>
> | Feature | Supported |
> |---------|-----------|
> | Bold    | Yes       |
> | Code    | Yes       |
