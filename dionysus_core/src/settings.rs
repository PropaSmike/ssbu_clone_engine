pub const DEFAULT_TEXT: &str = "gliding = true\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub gliding: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { gliding: true }
    }
}

fn flag(value: &str) -> Option<bool> {
    let value = value.trim_matches('"').trim();
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

pub fn parse(text: &str) -> (Settings, Vec<String>) {
    let mut settings = Settings::default();
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!("line {}: not key = value; ignored", index + 1));
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "gliding" => match flag(value) {
                Some(on) => settings.gliding = on,
                None => problems.push(format!("line {}: gliding = {value} is not true or false; ignored", index + 1)),
            },
            _ => problems.push(format!("line {}: unknown setting {key}; ignored", index + 1)),
        }
    }
    (settings, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_file_turns_gliding_on() {
        assert_eq!(parse(DEFAULT_TEXT), (Settings { gliding: true }, vec![]));
        assert_eq!(parse(""), (Settings::default(), vec![]));
    }

    #[test]
    fn gliding_false_turns_it_off() {
        assert_eq!(parse("gliding = false\n").0.gliding, false);
        assert_eq!(parse("gliding=false # another mod glides\r\n").0.gliding, false);
        assert_eq!(parse("gliding = \"FALSE\"").0.gliding, false);
        assert_eq!(parse("# gliding = false\ngliding = true").0.gliding, true);
    }

    #[test]
    fn a_bad_line_is_reported_and_leaves_the_rest() {
        let (settings, problems) = parse("gliding = no\nspeed = 2\nhello\n");
        assert!(settings.gliding);
        assert_eq!(problems.len(), 3);
        assert!(problems[0].starts_with("line 1: gliding = no"));
        assert!(problems[1].contains("unknown setting speed"));
        assert!(problems[2].starts_with("line 3"));
        let (settings, problems) = parse("gliding = false\ngliding = maybe\n");
        assert!(!settings.gliding);
        assert_eq!(problems.len(), 1);
    }
}
