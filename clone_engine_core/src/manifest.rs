#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub first_line: usize,
    pub text: String,
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

fn is_header(line: &str, table: &str) -> bool {
    let line = strip_comment(line).trim();
    line.len() == table.len() + 4
        && line.starts_with("[[")
        && line.ends_with("]]")
        && line[2..line.len() - 2].trim() == table
}

pub fn blocks(text: &str, table: &str) -> Result<Vec<Block>, usize> {
    let lines: Vec<&str> = text.lines().collect();
    let headers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| is_header(line, table))
        .map(|(index, _)| index)
        .collect();
    if headers.is_empty() {
        return Ok(vec![Block {
            first_line: 1,
            text: text.to_string(),
        }]);
    }
    if let Some(stray) = lines[..headers[0]]
        .iter()
        .position(|line| !strip_comment(line).trim().is_empty())
    {
        return Err(stray + 1);
    }
    let mut out = Vec::with_capacity(headers.len());
    for (position, &start) in headers.iter().enumerate() {
        let end = headers.get(position + 1).copied().unwrap_or(lines.len());
        out.push(Block {
            first_line: start + 2,
            text: lines[start + 1..end].join("\n"),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_manifest_is_one_block_from_line_one() {
        let text = "base_kind = 63\nresource_name = \"wawa\"\n";
        assert_eq!(
            blocks(text, "item").unwrap(),
            vec![Block {
                first_line: 1,
                text: text.to_string()
            }]
        );
    }

    #[test]
    fn headers_split_the_text_and_remember_where_each_block_starts() {
        let text = "# two items\n\n[[item]]\nbase_kind = 63\nresource_name = \"a\"\n\n[[item]] # second\nbase_kind = 50\nresource_name = \"b\"\n";
        let found = blocks(text, "item").unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].first_line, 4);
        assert_eq!(found[0].text, "base_kind = 63\nresource_name = \"a\"\n");
        assert_eq!(found[1].first_line, 8);
        assert_eq!(found[1].text, "base_kind = 50\nresource_name = \"b\"");
    }

    #[test]
    fn a_key_before_the_first_header_belongs_to_nothing_and_is_refused() {
        let text = "place = \"lost\"\n[[stage]]\nplace = \"a\"\n";
        assert_eq!(blocks(text, "stage"), Err(1));
    }

    #[test]
    fn another_tables_header_is_an_ordinary_line() {
        let text = "[[stage]]\nplace = \"a\"\n[[item]]\nplace = \"b\"\n";
        let found = blocks(text, "stage").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "place = \"a\"\n[[item]]\nplace = \"b\"");
    }

    #[test]
    fn an_empty_file_is_one_empty_block() {
        assert_eq!(blocks("", "item").unwrap().len(), 1);
    }
}
