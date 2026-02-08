---
title: Code Block Showcase
author: SilkPrint Test Suite
---

# Code Blocks

## Rust

```rust
use std::collections::HashMap;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let mut cache: HashMap<String, Vec<u8>> = HashMap::new();
    cache.insert("hello".to_string(), vec![1, 2, 3]);
    println!("fib(10) = {}", fibonacci(10));
}
```

## Python

```python
from dataclasses import dataclass
from typing import Optional

@dataclass
class Document:
    title: str
    content: str
    theme: Optional[str] = None

    def render(self) -> str:
        return f"# {self.title}\n\n{self.content}"

docs = [Document("Hello", "World"), Document("Test", "Content", "silk-dark")]
for doc in docs:
    print(doc.render())
```

## JavaScript

```javascript
const renderMarkdown = async (input, options = {}) => {
  const { theme = "silk-light", toc = false } = options;

  const ast = parse(input);
  const typst = emit(ast, { theme, toc });

  return await compile(typst);
};

// Arrow functions and template literals
const greet = (name) => `Hello, ${name}!`;
console.log(greet("SilkPrint"));
```

## Bash

```bash
#!/usr/bin/env bash
set -euo pipefail

THEMES_DIR="./themes"
OUTPUT_DIR="/tmp/silkprint-output"

for theme in "$THEMES_DIR"/*.toml; do
    name=$(basename "$theme" .toml)
    echo "Rendering with theme: $name"
    silkprint input.md -t "$name" -o "$OUTPUT_DIR/${name}.pdf"
done

echo "Done! Rendered ${#themes[@]} PDFs."
```

## JSON

```json
{
  "name": "silkprint",
  "version": "0.1.0",
  "themes": [
    { "id": "silk-light", "variant": "light", "print_safe": true },
    { "id": "silk-dark", "variant": "dark", "print_safe": false }
  ],
  "config": {
    "default_theme": "silk-light",
    "font_embedding": true,
    "pdf_version": "1.7"
  }
}
```

## YAML

```yaml
meta:
  name: Silk Light
  version: "1"
  variant: light

colors:
  primary: "#4a5dbd"
  secondary: "#555570"
  background: "#ffffff"

fonts:
  heading: Inter
  body: Source Serif 4
  mono: JetBrains Mono
```

## TOML

```toml
[meta]
name = "Custom Theme"
variant = "light"

[colors]
accent = "#e135ff"
background = "#1a1a2e"

[fonts]
heading = "Inter"
body = "Source Serif 4"
```

## Go

```go
package main

import (
	"fmt"
	"strings"
)

type Theme struct {
	Name    string
	Variant string
	Colors  map[string]string
}

func (t *Theme) Resolve(key string) string {
	if val, ok := t.Colors[key]; ok {
		return val
	}
	return "#000000"
}

func main() {
	theme := &Theme{
		Name:    "silk-light",
		Variant: "light",
		Colors:  map[string]string{"primary": "#4a5dbd"},
	}
	fmt.Println(strings.ToUpper(theme.Name))
}
```

## TypeScript

```typescript
interface ThemeConfig {
  meta: { name: string; variant: "light" | "dark" };
  colors: Record<string, string>;
  fonts: {
    heading: string;
    body: string;
    mono: string;
  };
}

async function loadTheme(name: string): Promise<ThemeConfig> {
  const response = await fetch(`/themes/${name}.toml`);
  const toml = await response.text();
  return parseToml<ThemeConfig>(toml);
}
```

## Inline Code

You can use `inline code` in the middle of a sentence. Variables like `font_size` and functions like `resolve_color()` render in monospace. A longer inline code span: `HashMap<String, Vec<Box<dyn Trait + Send + 'static>>>`.

## Long Lines (Wrapping Test)

```rust
fn this_function_has_a_very_long_signature_that_should_test_line_wrapping_behavior(first_parameter: &str, second_parameter: u64, third_parameter: Option<Vec<String>>, fourth_parameter: HashMap<String, Box<dyn std::fmt::Display>>) -> Result<String, Box<dyn std::error::Error>> {
    todo!("This line is intentionally very long to test how code blocks handle horizontal overflow and whether soft-wrapping or clipping is applied correctly by the theme engine")
}
```

## Empty Code Block

```
```

## No Language Specified

```
This is a code block with no language identifier.
It should render as plain preformatted text.
No syntax highlighting should be applied.
```
