---
title: SilkPrint Showcase
subtitle: Beautiful PDFs from Markdown
author: Stefanie Jane
date: 2025-06-15
toc: false
title_page: false
---

# SilkPrint

Transform your Markdown into stunning, publication-ready PDFs with **40 built-in themes** across 8 aesthetic families.

## Syntax Highlighting

SilkPrint renders code blocks with rich, theme-aware syntax highlighting powered by TextMate grammars.

```rust
use silkprint::Theme;

#[derive(Debug, Clone)]
struct Document {
    title: String,
    content: String,
    theme: Theme,
}

impl Document {
    fn render(&self) -> Result<Vec<u8>, Error> {
        let typst = emit_typst(&self.content, &self.theme)?;
        compile_pdf(typst)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = Document {
        title: "Hello, World".into(),
        content: std::fs::read_to_string("input.md")?,
        theme: Theme::load("silk-light")?,
    };
    std::fs::write("output.pdf", doc.render()?)?;
    Ok(())
}
```

```python
from dataclasses import dataclass, field
from pathlib import Path

@dataclass
class ThemeConfig:
    name: str
    variant: str = "light"
    colors: dict[str, str] = field(default_factory=dict)
    print_safe: bool = True

    def resolve_color(self, key: str) -> str:
        """Two-level color resolution within [colors] table."""
        value = self.colors.get(key, "#000000")
        return self.colors.get(value, value)

themes = [
    ThemeConfig("silk-light", colors={"primary": "#4a5dbd"}),
    ThemeConfig("nord", colors={"primary": "#5e81ac"}),
    ThemeConfig("dracula", variant="dark", print_safe=False),
]

for theme in themes:
    print(f"{theme.name}: {theme.resolve_color('primary')}")
```

## Tables

| Theme Family | Themes | Variants | Description |
|:------------|:------:|:--------:|:------------|
| **Signature** | 4 | Light + Dark | Clean, professional defaults |
| **SilkCircuit** | 5 | Dark + Dawn | Electric meets elegant |
| **Developer** | 10 | Mixed | Nord, Dracula, Solarized, and more |
| **Classic** | 4 | Light | Academic, typewriter, newspaper |
| **Nature** | 5 | Mixed | Forest, ocean, sunset, arctic, sakura |
| **Futuristic** | 5 | Dark | Cyberpunk, terminal, matrix |
| **Artistic** | 4 | Mixed | Noir, candy, blueprint, witch |
| **Greyscale** | 3 | Light | Warm, cool, high-contrast |

## Alerts

> [!NOTE]
> SilkPrint supports all 5 GitHub-style alert types with custom theming per alert level.

> [!TIP]
> Use `silkprint input.md --theme nord -o output.pdf` to render with any of 40 built-in themes.

> [!WARNING]
> Dark themes are not print-safe. Use `--list-themes` to check which themes support printing.

## Mathematics

Typst-native math rendering with full equation support:

The quadratic formula: $x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a)$

Display equations render beautifully:

$$ integral_0^infinity e^(-x) dif x = 1 $$

$$ sum_(k=0)^infinity x^k / k! = e^x $$

## Blockquotes

> "Any sufficiently advanced technology is indistinguishable from magic."
>
> --- Arthur C. Clarke

## Features at a Glance

- **40 built-in themes** across 8 aesthetic families
- Rich **syntax highlighting** for 20+ languages
- **GitHub-style alerts** (note, tip, important, warning, caution)
- **Typst-native math** with inline and display equations
- **Tables** with alignment, striping, and formatting
- **YAML front matter** for document metadata
- **Task lists**, footnotes, and description lists
- Print-safe themes validated with **WCAG contrast checks**
- **Custom themes** via TOML with 24 configurable sections
