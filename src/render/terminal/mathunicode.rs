//! Tiny Typst-math to Unicode renderer for inline terminal math.

#[derive(Clone, Copy)]
enum Script {
    Super,
    Sub,
}

/// Render inline Typst-flavored math as Unicode when it can be done honestly.
///
/// Unsupported constructs return the original source unchanged.
pub fn render_inline(source: &str) -> String {
    let mut parser = Parser::new(source);
    match parser.render() {
        Some(output) => output,
        None => source.to_string(),
    }
}

struct Parser<'a> {
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    fn render(&mut self) -> Option<String> {
        let mut out = String::with_capacity(self.source.len());
        while let Some(ch) = self.peek() {
            match ch {
                '^' | '_' => {
                    self.pos += 1;
                    let script = if ch == '^' {
                        Script::Super
                    } else {
                        Script::Sub
                    };
                    let atom = self.script_atom()?;
                    out.push_str(&script_text(&atom, script)?);
                }
                'a'..='z' | 'A'..='Z' => {
                    let ident = self.ident();
                    out.push_str(symbol(&ident).unwrap_or(&ident));
                }
                '-' if self.peek_next() == Some('>') => {
                    self.pos += 2;
                    out.push('\u{2192}');
                }
                '<' if self.peek_next() == Some('-') => {
                    self.pos += 2;
                    out.push('\u{2190}');
                }
                _ => {
                    self.pos += 1;
                    out.push(ch);
                }
            }
        }
        Some(out)
    }

    fn script_atom(&mut self) -> Option<String> {
        self.skip_ws();
        match self.peek()? {
            '(' => self.group('(', ')'),
            '{' => self.group('{', '}'),
            'a'..='z' | 'A'..='Z' => {
                let ident = self.ident();
                Some(symbol(&ident).unwrap_or(&ident).to_string())
            }
            ch if ch.is_ascii_digit() || matches!(ch, '+' | '-' | '=' | '(' | ')') => {
                self.pos += 1;
                Some(ch.to_string())
            }
            _ => None,
        }
    }

    fn group(&mut self, open: char, close: char) -> Option<String> {
        if self.peek()? != open {
            return None;
        }
        self.pos += 1;
        let start = self.pos;
        let mut depth = 1usize;
        while let Some(ch) = self.peek() {
            self.pos += 1;
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let end = self.pos - 1;
                    return Some(self.chars[start..end].iter().collect());
                }
            }
        }
        None
    }

    fn ident(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.chars[start..self.pos].iter().collect()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }
}

fn symbol(ident: &str) -> Option<&'static str> {
    Some(match ident {
        "alpha" => "\u{03b1}",
        "beta" => "\u{03b2}",
        "gamma" => "\u{03b3}",
        "delta" => "\u{03b4}",
        "epsilon" => "\u{03b5}",
        "theta" => "\u{03b8}",
        "lambda" => "\u{03bb}",
        "mu" => "\u{03bc}",
        "pi" => "\u{03c0}",
        "sigma" => "\u{03c3}",
        "omega" => "\u{03c9}",
        "phi" => "\u{03c6}",
        "sqrt" => "\u{221a}",
        "sum" => "\u{2211}",
        "integral" => "\u{222b}",
        "infinity" => "\u{221e}",
        "times" => "\u{00d7}",
        "div" => "\u{00f7}",
        "plus.minus" => "\u{00b1}",
        "eq.not" => "\u{2260}",
        "lt.eq" => "\u{2264}",
        "gt.eq" => "\u{2265}",
        "approx" => "\u{2248}",
        "prop" => "\u{221d}",
        "arrow.r" => "\u{2192}",
        "arrow.l" => "\u{2190}",
        "arrow.r.double" => "\u{21d2}",
        "arrow.l.r" => "\u{2194}",
        "in" => "\u{2208}",
        "in.not" => "\u{2209}",
        "subset" => "\u{2282}",
        "union" => "\u{222a}",
        "inter" => "\u{2229}",
        "emptyset" => "\u{2205}",
        "dif" => "d",
        _ => return None,
    })
}

fn script_text(text: &str, script: Script) -> Option<String> {
    text.chars()
        .filter(|ch| !ch.is_whitespace())
        .map(|ch| script_char(ch, script))
        .collect()
}

fn script_char(ch: char, script: Script) -> Option<char> {
    match script {
        Script::Super => super_char(ch),
        Script::Sub => sub_char(ch),
    }
}

fn super_char(ch: char) -> Option<char> {
    Some(match ch {
        '0' => '\u{2070}',
        '1' => '\u{00b9}',
        '2' => '\u{00b2}',
        '3' => '\u{00b3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        '+' => '\u{207a}',
        '-' => '\u{207b}',
        '=' => '\u{207c}',
        '(' => '\u{207d}',
        ')' => '\u{207e}',
        'i' => '\u{2071}',
        'n' => '\u{207f}',
        _ => return None,
    })
}

fn sub_char(ch: char) -> Option<char> {
    Some(match ch {
        '0' => '\u{2080}',
        '1' => '\u{2081}',
        '2' => '\u{2082}',
        '3' => '\u{2083}',
        '4' => '\u{2084}',
        '5' => '\u{2085}',
        '6' => '\u{2086}',
        '7' => '\u{2087}',
        '8' => '\u{2088}',
        '9' => '\u{2089}',
        '+' => '\u{208a}',
        '-' => '\u{208b}',
        '=' => '\u{208c}',
        '(' => '\u{208d}',
        ')' => '\u{208e}',
        'a' => '\u{2090}',
        'e' => '\u{2091}',
        'h' => '\u{2095}',
        'i' => '\u{1d62}',
        'j' => '\u{2c7c}',
        'k' => '\u{2096}',
        'l' => '\u{2097}',
        'm' => '\u{2098}',
        'n' => '\u{2099}',
        'o' => '\u{2092}',
        'p' => '\u{209a}',
        'r' => '\u{1d63}',
        's' => '\u{209b}',
        't' => '\u{209c}',
        'u' => '\u{1d64}',
        'v' => '\u{1d65}',
        'x' => '\u{2093}',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::render_inline;

    #[test]
    fn renders_symbols_and_scripts() {
        assert_eq!(
            render_inline("alpha^2 + beta_1"),
            "\u{03b1}\u{00b2} + \u{03b2}\u{2081}"
        );
        assert_eq!(
            render_inline("sum_(i=1)^n i"),
            "\u{2211}\u{1d62}\u{208c}\u{2081}\u{207f} i"
        );
    }

    #[test]
    fn renders_common_typst_symbols() {
        assert_eq!(
            render_inline("plus.minus sqrt(5) approx phi"),
            "\u{00b1} \u{221a}(5) \u{2248} \u{03c6}"
        );
    }

    #[test]
    fn falls_back_for_unrepresentable_scripts() {
        assert_eq!(render_inline("x^abc"), "x^abc");
    }
}
