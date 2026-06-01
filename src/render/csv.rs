pub fn parse_rows(source: &str) -> Option<Vec<Vec<String>>> {
    let rows: Vec<Vec<String>> = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_line)
        .collect::<Option<_>>()?;

    (!rows.is_empty()
        && rows
            .iter()
            .any(|row| row.iter().any(|cell| !cell.trim().is_empty())))
    .then_some(rows)
}

fn parse_line(line: &str) -> Option<Vec<String>> {
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    let mut after_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' if !quoted && cell.trim().is_empty() => {
                quoted = true;
                after_quote = false;
            }
            '"' if quoted => {
                quoted = false;
                after_quote = true;
            }
            ',' if !quoted => {
                row.push(cell.trim().to_string());
                cell.clear();
                after_quote = false;
            }
            _ if after_quote && !ch.is_whitespace() => return None,
            _ => cell.push(ch),
        }
    }

    if quoted {
        return None;
    }
    row.push(cell.trim().to_string());
    Some(row)
}

#[cfg(test)]
mod tests {
    use super::parse_rows;

    #[test]
    fn parses_basic_rows() {
        let rows = parse_rows("name,count\nalpha,1\nbeta,2\n").expect("rows");

        assert_eq!(rows[0], ["name", "count"]);
        assert_eq!(rows[2], ["beta", "2"]);
    }

    #[test]
    fn parses_quoted_cells() {
        let rows = parse_rows("name,note\nalpha,\"hello, \"\"world\"\"\"\n").expect("rows");

        assert_eq!(rows[1], ["alpha", "hello, \"world\""]);
    }

    #[test]
    fn rejects_unbalanced_quotes() {
        assert!(parse_rows("a,b\n\"oops,b\n").is_none());
    }
}
