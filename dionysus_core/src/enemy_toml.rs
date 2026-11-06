use crate::manifest::{blocks, strip_comment};

pub const BASES: &[(&str, &str, u32)] = &[
    ("yellow_devil", "yellowdevil", 0),
    ("dark_emperor", "subetenoyaminoou", 1),
    ("flying_man", "flyingman", 2),
    ("metal_face", "metalface", 3),
    ("nabbit", "nabbit", 4),
];
pub const MAX_NAME: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyDeclaration {
    pub name: String,
    pub base: u32,
}

pub fn base_kind(name: &str) -> Option<u32> {
    let name = name.trim().to_ascii_lowercase();
    BASES
        .iter()
        .find(|(plain, folder, _)| *plain == name || *folder == name)
        .map(|(_, _, kind)| *kind)
}

pub fn base_folder(kind: u32) -> Option<&'static str> {
    BASES.iter().find(|(_, _, known)| *known == kind).map(|(_, folder, _)| *folder)
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && base_kind(name).is_none()
        && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn parse_block(text: &str, first_line: usize) -> Result<EnemyDeclaration, String> {
    let (mut name, mut base) = (None, None);
    for (index, raw) in text.lines().enumerate() {
        let line_number = first_line + index;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {line_number}: expected key = value"));
        };
        let value = value.trim().trim_matches('"').trim();
        match key.trim() {
            "name" => {
                if !valid_name(value) {
                    return Err(format!(
                        "line {line_number}: name {value:?} must be lower case letters, digits and _, up to {MAX_NAME}, and not a game enemy's"
                    ));
                }
                name = Some(value.to_string());
            }
            "base" => {
                base = Some(base_kind(value).ok_or_else(|| {
                    format!("line {line_number}: base {value:?} is not yellow_devil, dark_emperor, flying_man, metal_face or nabbit")
                })?)
            }
            other => return Err(format!("line {line_number}: unknown enemy key {other}")),
        }
    }
    Ok(EnemyDeclaration {
        name: name.ok_or_else(|| format!("line {first_line}: an enemy needs name"))?,
        base: base.ok_or_else(|| format!("line {first_line}: an enemy needs base"))?,
    })
}

pub fn parse(text: &str) -> Result<Vec<EnemyDeclaration>, String> {
    let found = blocks(text, "enemy").map_err(|line| format!("line {line}: a key outside any [[enemy]] block"))?;
    found.iter().map(|block| parse_block(&block.text, block.first_line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_enemy_names_itself_and_its_base() {
        let found = parse("name = \"my_enemy\" # files under enemy/my_enemy\nbase = \"nabbit\"\n").unwrap();
        assert_eq!(found, vec![EnemyDeclaration { name: "my_enemy".to_string(), base: 4 }]);
        let two = parse("[[enemy]]\nname = \"a\"\nbase = \"flyingman\"\n\n[[enemy]]\nname = \"b\"\nbase = \"Dark_Emperor\"\n").unwrap();
        assert_eq!((two[0].base, two[1].base), (2, 1));
        assert_eq!(base_folder(1), Some("subetenoyaminoou"));
    }

    #[test]
    fn what_the_engine_cannot_clone_is_refused() {
        assert!(parse("name = \"a\"\n").unwrap_err().contains("needs base"));
        assert!(parse("base = \"nabbit\"\n").unwrap_err().contains("needs name"));
        assert!(parse("name = \"nabbit\"\nbase = \"nabbit\"\n").unwrap_err().contains("not a game enemy's"));
        assert!(parse("name = \"a\"\nbase = \"primid\"\n").unwrap_err().contains("is not"));
        assert!(parse("name = \"a\"\nbase = \"nabbit\"\nspeed = 2\n").unwrap_err().contains("unknown enemy key"));
    }
}
