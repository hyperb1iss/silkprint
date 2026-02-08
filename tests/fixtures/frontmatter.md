---
title: Front Matter Showcase
subtitle: Testing All YAML Metadata Fields
author: Stefanie Jane
date: 2025-06-15
theme: silk-light
toc: true
title_page: true
---

# Front Matter

This document tests the full YAML front matter support. All metadata fields above
should be parsed and applied correctly by SilkPrint.

## Expected Behavior

- The **title** and **subtitle** appear on the title page
- The **author** name is displayed below the title
- The **date** is formatted and shown on the title page
- The **theme** overrides the default `silk-light` (same here, but validates the path)
- The **toc** flag generates a table of contents
- The **title_page** flag enables the dedicated title page

## Section One

Some content to give the table of contents entries to display.

### Subsection 1.1

Nested content here.

### Subsection 1.2

More nested content.

## Section Two

Another top-level section for TOC purposes.

---

The remainder of this file tests a document with minimal front matter.
In practice this would be a separate file, but we document the expected
minimal format here:

```yaml
---
title: Just a Title
---
```

Only the `title` field is required. All other fields have sensible defaults:
- `theme` defaults to `silk-light`
- `toc` defaults to `false`
- `title_page` defaults to `true` when title is present
