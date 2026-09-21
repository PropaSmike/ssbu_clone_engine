#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    Float(f64),
    Int(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamOp {
    Set,
    Mul,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDeclaration {
    pub name: String,
    pub slot: Option<i32>,
    pub op: ParamOp,
    pub value: ParamValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArticleDeclaration {
    pub name: String,
    pub from: Option<(String, String)>,
    pub base_article: Option<String>,
    pub kirby: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MotionDeclaration {
    pub name: String,
    pub animation: String,
    pub template: Option<String>,
    pub game: Option<String>,
    pub sound: Option<String>,
    pub effect: Option<String>,
    pub expression: Option<String>,
    pub flags: Vec<String>,
    pub blend_frames: Option<u8>,
    pub cancel_frame: Option<u8>,
    pub xlu: Option<(u8, u8)>,
    pub no_stop_intp: Option<bool>,
    pub animation_unk: Option<u8>,
    pub no_extra: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KirbyDeclaration {
    pub statuses: i32,
    pub first: Option<i32>,
    pub full_model: bool,
    pub model: Option<String>,
    pub motions: Vec<MotionDeclaration>,
    pub meshes: Vec<(String, bool)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FighterDeclaration {
    pub name: String,
    pub base: String,
    pub display_name: Option<String>,
    pub color_start: u32,
    pub color_count: u32,
    pub ui_chara: Option<String>,
    pub fighter_kind_name: Option<String>,
    pub resource_name: Option<String>,
    pub base_resource_name: Option<String>,
    pub series: Option<String>,
    pub disp_order: Option<i8>,
    pub save_no: Option<i8>,
    pub exhibit_year: Option<i16>,
    pub narration: Option<String>,
    pub staffroll: bool,
    pub own_css: bool,
    pub jingle: Option<String>,
    pub owns_param_resources: bool,
    pub effect_namespace: u32,
    pub article_namespace: u32,
    pub articles: Vec<ArticleDeclaration>,
    pub kirby: Option<KirbyDeclaration>,
    pub params: Vec<ParamDeclaration>,
}

pub const DEFAULT_COLOR_COUNT: u32 = 8;

impl FighterDeclaration {
    pub fn ui_chara(&self) -> String {
        self.ui_chara
            .clone()
            .unwrap_or_else(|| format!("ui_chara_{}", self.name))
    }

    pub fn fighter_kind_name(&self) -> String {
        self.fighter_kind_name
            .clone()
            .unwrap_or_else(|| format!("fighter_kind_{}", self.name))
    }

    pub fn resource_name(&self) -> String {
        self.resource_name.clone().unwrap_or_else(|| self.name.clone())
    }

    pub fn base_resource_name(&self) -> String {
        self.base_resource_name
            .clone()
            .unwrap_or_else(|| self.base.clone())
    }

    pub fn display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.name.clone())
    }

    pub fn series(&self) -> String {
        let series = self.series.as_deref().unwrap_or("");
        if series.starts_with("ui_series_") {
            series.to_string()
        } else {
            format!("ui_series_{series}")
        }
    }
}

impl MotionDeclaration {
    pub fn script(&self, prefix: &str, stem: Option<&str>) -> Option<String> {
        let explicit = match prefix {
            "game" => &self.game,
            "sound" => &self.sound,
            "effect" => &self.effect,
            _ => &self.expression,
        };
        explicit
            .clone()
            .or_else(|| stem.map(|stem| format!("{prefix}_{stem}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Fighter,
    Article,
    Kirby,
    Motion,
    Mesh,
    Params { slot: Option<i32>, op: ParamOp },
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

fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

fn unquote(value: &str, line: usize) -> Result<String, ParseError> {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return Ok(value[1..value.len() - 1].to_string());
    }
    if value.starts_with('"') || value.ends_with('"') {
        return Err(error(line, "unterminated string"));
    }
    Err(error(line, "expected a quoted string"))
}

fn parse_bool(value: &str, line: usize, key: &str) -> Result<bool, ParseError> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error(line, format!("{key} must be true or false"))),
    }
}

fn parse_int<T: core::str::FromStr>(value: &str, line: usize, key: &str) -> Result<T, ParseError> {
    let value = value.trim();
    let parsed = if let Some(hex) = value.strip_prefix("0x") {
        i64::from_str_radix(hex, 16)
            .ok()
            .and_then(|number| number.to_string().parse::<T>().ok())
    } else {
        value.parse::<T>().ok()
    };
    parsed.ok_or_else(|| error(line, format!("{key} must be a whole number")))
}

fn parse_list(value: &str, line: usize, key: &str) -> Result<Vec<String>, ParseError> {
    let value = value.trim();
    let Some(inner) = value.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) else {
        return Err(error(line, format!("{key} must be a [list]")));
    };
    inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            if item.starts_with('"') {
                unquote(item, line)
            } else {
                Ok(item.to_string())
            }
        })
        .collect()
}

fn header(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if let Some(inner) = trimmed
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
    {
        return Some((inner.trim().to_string(), true));
    }
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        return Some((inner.trim().to_string(), false));
    }
    None
}

fn section_for(name: &str, array: bool, line: usize) -> Result<Section, ParseError> {
    let name = name.strip_prefix("fighter.").unwrap_or(name);
    match (name, array) {
        ("fighter", false) => Ok(Section::Fighter),
        ("article", true) => Ok(Section::Article),
        ("kirby", false) => Ok(Section::Kirby),
        ("kirby.motion", true) => Ok(Section::Motion),
        ("kirby.mesh", true) => Ok(Section::Mesh),
        ("params", false) => Ok(Section::Params {
            slot: None,
            op: ParamOp::Set,
        }),
        (rest, false) if rest.starts_with("params.") => {
            let mut slot = None;
            let mut op = ParamOp::Set;
            for part in rest["params.".len()..].split('.') {
                if part == "mul" {
                    op = ParamOp::Mul;
                } else if let Some(number) = part.strip_prefix('c') {
                    slot = Some(parse_int::<i32>(number, line, "costume")?);
                } else {
                    return Err(error(line, format!("unknown params table [{rest}]")));
                }
            }
            Ok(Section::Params { slot, op })
        }
        _ => Err(error(
            line,
            format!("unknown table [{}{name}{}]", if array { "[" } else { "" }, if array { "]" } else { "" }),
        )),
    }
}

pub fn parse(text: &str) -> Result<FighterDeclaration, ParseError> {
    let mut declaration = FighterDeclaration {
        color_count: DEFAULT_COLOR_COUNT,
        ..FighterDeclaration::default()
    };
    let mut section = Section::Fighter;
    let mut saw_costumes = false;
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some((name, array)) = header(line) {
            section = section_for(&name, array, line_number)?;
            match section {
                Section::Article => declaration.articles.push(ArticleDeclaration::default()),
                Section::Kirby => {
                    declaration.kirby.get_or_insert_with(KirbyDeclaration::default);
                }
                Section::Motion => declaration
                    .kirby
                    .get_or_insert_with(KirbyDeclaration::default)
                    .motions
                    .push(MotionDeclaration::default()),
                Section::Mesh => declaration
                    .kirby
                    .get_or_insert_with(KirbyDeclaration::default)
                    .meshes
                    .push((String::new(), true)),
                _ => {}
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(error(line_number, "expected key = value"));
        };
        let key = key.trim();
        let value = value.trim();
        match section {
            Section::Fighter => match key {
                "name" => declaration.name = unquote(value, line_number)?,
                "base" => declaration.base = unquote(value, line_number)?,
                "display_name" => declaration.display_name = Some(unquote(value, line_number)?),
                "costumes" | "color_count" => {
                    declaration.color_count = parse_int(value, line_number, key)?;
                    saw_costumes = true;
                }
                "color_start" => declaration.color_start = parse_int(value, line_number, key)?,
                "ui_chara" => declaration.ui_chara = Some(unquote(value, line_number)?),
                "fighter_kind_name" => {
                    declaration.fighter_kind_name = Some(unquote(value, line_number)?)
                }
                "resource_name" => declaration.resource_name = Some(unquote(value, line_number)?),
                "base_resource_name" => {
                    declaration.base_resource_name = Some(unquote(value, line_number)?)
                }
                "series" => declaration.series = Some(unquote(value, line_number)?),
                "disp_order" => declaration.disp_order = Some(parse_int(value, line_number, key)?),
                "save_no" => declaration.save_no = Some(parse_int(value, line_number, key)?),
                "exhibit_year" => {
                    declaration.exhibit_year = Some(parse_int(value, line_number, key)?)
                }
                "narration" => declaration.narration = Some(unquote(value, line_number)?),
                "staffroll" => declaration.staffroll = parse_bool(value, line_number, key)?,
                "css" => declaration.own_css = !parse_bool(value, line_number, key)?,
                "jingle" => declaration.jingle = Some(unquote(value, line_number)?),
                "owns_param_resources" => {
                    declaration.owns_param_resources = parse_bool(value, line_number, key)?
                }
                "effect_namespace" => {
                    declaration.effect_namespace = parse_int(value, line_number, key)?
                }
                "article_namespace" => {
                    declaration.article_namespace = parse_int(value, line_number, key)?
                }
                _ => return Err(error(line_number, format!("unknown key {key}"))),
            },
            Section::Article => {
                let article = declaration.articles.last_mut().expect("pushed at header");
                match key {
                    "name" => article.name = unquote(value, line_number)?,
                    "from" => {
                        let from = unquote(value, line_number)?;
                        let Some((owner, weapon)) = from.split_once('/') else {
                            return Err(error(line_number, "from must be \"fighter/weapon\""));
                        };
                        if owner.is_empty() || weapon.is_empty() {
                            return Err(error(line_number, "from must be \"fighter/weapon\""));
                        }
                        article.from = Some((owner.to_string(), weapon.to_string()));
                    }
                    "base_article" => article.base_article = Some(unquote(value, line_number)?),
                    "kirby" => article.kirby = parse_bool(value, line_number, key)?,
                    _ => return Err(error(line_number, format!("unknown article key {key}"))),
                }
            }
            Section::Kirby => {
                let kirby = declaration.kirby.as_mut().expect("inserted at header");
                match key {
                    "statuses" => kirby.statuses = parse_int(value, line_number, key)?,
                    "first" => kirby.first = Some(parse_int(value, line_number, key)?),
                    "full_model" => kirby.full_model = parse_bool(value, line_number, key)?,
                    "model" => kirby.model = Some(unquote(value, line_number)?),
                    _ => return Err(error(line_number, format!("unknown kirby key {key}"))),
                }
            }
            Section::Motion => {
                let motion = declaration
                    .kirby
                    .as_mut()
                    .and_then(|kirby| kirby.motions.last_mut())
                    .expect("pushed at header");
                match key {
                    "name" => motion.name = unquote(value, line_number)?,
                    "animation" => motion.animation = unquote(value, line_number)?,
                    "template" => motion.template = Some(unquote(value, line_number)?),
                    "scripts" => {
                        let stem = unquote(value, line_number)?;
                        motion.game.get_or_insert_with(|| format!("game_{stem}"));
                        motion.sound.get_or_insert_with(|| format!("sound_{stem}"));
                        motion.effect.get_or_insert_with(|| format!("effect_{stem}"));
                        motion
                            .expression
                            .get_or_insert_with(|| format!("expression_{stem}"));
                    }
                    "game" => motion.game = Some(unquote(value, line_number)?),
                    "sound" => motion.sound = Some(unquote(value, line_number)?),
                    "effect" => motion.effect = Some(unquote(value, line_number)?),
                    "expression" => motion.expression = Some(unquote(value, line_number)?),
                    "flags" => motion.flags = parse_list(value, line_number, key)?,
                    "loop" => {
                        if parse_bool(value, line_number, key)? {
                            motion.flags.push("loop".to_string());
                        }
                    }
                    "blend_frames" => {
                        motion.blend_frames = Some(parse_int(value, line_number, key)?)
                    }
                    "cancel_frame" => {
                        motion.cancel_frame = Some(parse_int(value, line_number, key)?)
                    }
                    "xlu" => {
                        let pair = parse_list(value, line_number, key)?;
                        if pair.len() != 2 {
                            return Err(error(line_number, "xlu must be [start, end]"));
                        }
                        motion.xlu = Some((
                            parse_int(&pair[0], line_number, key)?,
                            parse_int(&pair[1], line_number, key)?,
                        ));
                    }
                    "no_stop_intp" => {
                        motion.no_stop_intp = Some(parse_bool(value, line_number, key)?)
                    }
                    "animation_unk" => {
                        motion.animation_unk = Some(parse_int(value, line_number, key)?)
                    }
                    "no_extra" => motion.no_extra = parse_bool(value, line_number, key)?,
                    _ => return Err(error(line_number, format!("unknown motion key {key}"))),
                }
            }
            Section::Mesh => {
                let mesh = declaration
                    .kirby
                    .as_mut()
                    .and_then(|kirby| kirby.meshes.last_mut())
                    .expect("pushed at header");
                match key {
                    "name" => mesh.0 = unquote(value, line_number)?,
                    "visible" => mesh.1 = parse_bool(value, line_number, key)?,
                    _ => return Err(error(line_number, format!("unknown mesh key {key}"))),
                }
            }
            Section::Params { slot, op } => {
                let value = value.trim();
                let parsed = if value.contains('.') || value.contains('e') {
                    value
                        .parse::<f64>()
                        .map(ParamValue::Float)
                        .map_err(|_| error(line_number, format!("{key} must be a number")))?
                } else {
                    ParamValue::Int(parse_int(value, line_number, key)?)
                };
                if op == ParamOp::Mul && matches!(parsed, ParamValue::Int(_)) {
                    return Err(error(line_number, format!("{key}: a multiplier must be a decimal")));
                }
                declaration.params.push(ParamDeclaration {
                    name: key.to_string(),
                    slot,
                    op,
                    value: parsed,
                });
            }
        }
    }
    finish(declaration, saw_costumes)
}

fn finish(declaration: FighterDeclaration, saw_costumes: bool) -> Result<FighterDeclaration, ParseError> {
    if declaration.name.is_empty() {
        return Err(error(0, "name is required"));
    }
    if declaration.base.is_empty() {
        return Err(error(0, "base is required"));
    }
    if !valid_name(&declaration.name) {
        return Err(error(0, "name must be lowercase letters, digits and underscores"));
    }
    if declaration.color_count == 0 || declaration.color_start + declaration.color_count > 256 {
        return Err(error(0, "costumes must be 1 to 256 including color_start"));
    }
    let _ = saw_costumes;
    for article in &declaration.articles {
        if article.name.is_empty() {
            return Err(error(0, "an article needs a name"));
        }
        if article.from.is_none() && article.base_article.is_none() {
            return Err(error(
                0,
                format!("article {} needs from = \"fighter/weapon\" or base_article", article.name),
            ));
        }
        if article.from.is_some() && article.base_article.is_some() {
            return Err(error(
                0,
                format!("article {} has both from and base_article", article.name),
            ));
        }
    }
    if let Some(kirby) = &declaration.kirby {
        if kirby.statuses < 0 {
            return Err(error(0, "kirby statuses must not be negative"));
        }
        if kirby.first.is_some_and(|first| first <= 0) {
            return Err(error(0, "kirby first must be a positive status kind"));
        }
        for motion in &kirby.motions {
            if motion.name.is_empty() || motion.animation.is_empty() {
                return Err(error(0, "a kirby motion needs name and animation"));
            }
        }
        for (mesh, _) in &kirby.meshes {
            if mesh.is_empty() {
                return Err(error(0, "a kirby mesh needs a name"));
            }
        }
    }
    Ok(declaration)
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub fn parse_all(text: &str) -> Result<Vec<FighterDeclaration>, ParseError> {
    let blocks = crate::manifest::blocks(text, "fighter")
        .map_err(|line| error(line, "a key before the first [[fighter]] belongs to nothing"))?;
    blocks
        .iter()
        .map(|block| {
            parse(&block.text).map_err(|mut failure| {
                if failure.line > 0 {
                    failure.line += block.first_line - 1;
                }
                failure
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KAWASHIMA: &str = r#"
[fighter]
name = "kawashima"          # files under fighter/kawashima
base = "marth"
display_name = "Kawashima"
costumes = 1
series = "tekken"
disp_order = 40
narration = "vc_narration_characall_kawashima"

[[article]]
name = "coin"
from = "simon/cross"

[[article]]
name = "cshot"
base_article = "cshot"

[kirby]
statuses = 4
full_model = true

[[kirby.motion]]
name = "specialnstart"
animation = "kawashima_specialnstart"
scripts = "kawashima_specialnstart"

[[kirby.motion]]
name = "specialnloop"
animation = "kawashima_specialnloop"
scripts = "kawashima_specialnloop"
loop = true
xlu = [3, 9]

[[kirby.mesh]]
name = "hat_visor"
visible = false

[params]
run_speed_max = 1.645
attack_combo_max = 3

[params.mul]
jump_y = 1.1

[params.c07]
scale = 0.95
"#;

    #[test]
    fn the_worked_example_parses_into_every_table() {
        let fighter = parse(KAWASHIMA).unwrap();
        assert_eq!(fighter.name, "kawashima");
        assert_eq!(fighter.base, "marth");
        assert_eq!(fighter.color_count, 1);
        assert_eq!(fighter.color_start, 0);
        assert_eq!(fighter.ui_chara(), "ui_chara_kawashima");
        assert_eq!(fighter.fighter_kind_name(), "fighter_kind_kawashima");
        assert_eq!(fighter.resource_name(), "kawashima");
        assert_eq!(fighter.base_resource_name(), "marth");
        assert_eq!(fighter.series(), "ui_series_tekken");
        assert_eq!(fighter.disp_order, Some(40));
        assert_eq!(fighter.narration.as_deref(), Some("vc_narration_characall_kawashima"));
        assert_eq!(fighter.articles.len(), 2);
        assert_eq!(
            fighter.articles[0].from,
            Some(("simon".to_string(), "cross".to_string()))
        );
        assert_eq!(fighter.articles[1].base_article.as_deref(), Some("cshot"));
        let kirby = fighter.kirby.as_ref().unwrap();
        assert_eq!(kirby.statuses, 4);
        assert!(kirby.full_model);
        assert_eq!(kirby.motions.len(), 2);
        assert_eq!(
            kirby.motions[0].script("game", None).as_deref(),
            Some("game_kawashima_specialnstart")
        );
        assert_eq!(
            kirby.motions[0].script("expression", None).as_deref(),
            Some("expression_kawashima_specialnstart")
        );
        assert_eq!(kirby.motions[1].flags, vec!["loop".to_string()]);
        assert_eq!(kirby.motions[1].xlu, Some((3, 9)));
        assert_eq!(kirby.meshes, vec![("hat_visor".to_string(), false)]);
        assert_eq!(fighter.params.len(), 4);
        assert_eq!(fighter.params[0].value, ParamValue::Float(1.645));
        assert_eq!(fighter.params[1].value, ParamValue::Int(3));
        assert_eq!(fighter.params[2].op, ParamOp::Mul);
        assert_eq!(fighter.params[3].slot, Some(7));
    }

    #[test]
    fn the_flat_form_needs_no_fighter_header_and_defaults_to_eight_costumes() {
        let fighter = parse("name = \"wawa\"\nbase = \"samus\"\n").unwrap();
        assert_eq!(fighter.color_count, DEFAULT_COLOR_COUNT);
        assert_eq!(fighter.series(), "ui_series_");
        assert!(fighter.kirby.is_none());
        assert!(fighter.articles.is_empty());
    }

    #[test]
    fn a_missing_name_or_base_is_refused() {
        assert_eq!(parse("base = \"samus\"\n").unwrap_err().message, "name is required");
        assert_eq!(parse("name = \"wawa\"\n").unwrap_err().message, "base is required");
    }

    #[test]
    fn an_unknown_key_names_its_line() {
        let failure = parse("name = \"wawa\"\nbase = \"samus\"\ncolour = 3\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert_eq!(failure.message, "unknown key colour");
    }

    #[test]
    fn an_unknown_table_is_refused() {
        let failure = parse("name = \"wawa\"\nbase = \"samus\"\n[sound]\nbank = 1\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert!(failure.message.starts_with("unknown table"));
    }

    #[test]
    fn an_article_needs_a_source_or_a_base_article_but_not_both() {
        let neither = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"x\"\n";
        assert!(parse(neither).unwrap_err().message.contains("needs from"));
        let both = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\nbase_article = \"cshot\"\n";
        assert!(parse(both).unwrap_err().message.contains("both"));
        let bad_from = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"x\"\nfrom = \"fireball\"\n";
        assert_eq!(parse(bad_from).unwrap_err().line, 5);
    }

    #[test]
    fn costume_range_is_bounded_like_the_engine() {
        assert!(parse("name = \"a\"\nbase = \"b\"\ncostumes = 0\n").is_err());
        assert!(parse("name = \"a\"\nbase = \"b\"\ncostumes = 256\n").is_ok());
        assert!(parse("name = \"a\"\nbase = \"b\"\ncolor_start = 8\ncostumes = 249\n").is_err());
        assert!(parse("name = \"a\"\nbase = \"b\"\ncolor_start = 8\ncostumes = 248\n").is_ok());
    }

    #[test]
    fn a_multiplier_must_be_a_decimal_and_hex_is_a_whole_number() {
        let failure = parse("name = \"a\"\nbase = \"b\"\n[params.mul]\njump_y = 2\n").unwrap_err();
        assert_eq!(failure.line, 4);
        let fighter = parse("name = \"a\"\nbase = \"b\"\n[kirby]\nfirst = 0x400\nstatuses = 6\n").unwrap();
        assert_eq!(fighter.kirby.unwrap().first, Some(0x400));
    }

    #[test]
    fn names_are_folder_names() {
        assert!(parse("name = \"Wawa\"\nbase = \"samus\"\n").is_err());
        assert!(parse("name = \"wawa-2\"\nbase = \"samus\"\n").is_err());
        assert!(parse("name = \"wawa_2\"\nbase = \"samus\"\n").is_ok());
    }

    #[test]
    fn several_fighters_are_blocks_with_prefixed_tables_and_absolute_lines() {
        let text = "[[fighter]]\nname = \"a\"\nbase = \"mario\"\n[[fighter.article]]\nname = \"ball\"\nfrom = \"mario/fireball\"\n[fighter.kirby]\nstatuses = 2\n\n[[fighter]]\nname = \"b\"\nbase = \"luigi\"\nbogus = 1\n";
        let failure = parse_all(text).unwrap_err();
        assert_eq!(failure.line, 13);
        let fixed = text.replace("bogus = 1\n", "");
        let fighters = parse_all(&fixed).unwrap();
        assert_eq!(fighters.len(), 2);
        assert_eq!(fighters[0].articles[0].name, "ball");
        assert_eq!(fighters[0].kirby.as_ref().unwrap().statuses, 2);
        assert_eq!(fighters[1].base, "luigi");
    }

    #[test]
    fn a_key_before_the_first_block_is_refused_at_its_line() {
        let failure = parse_all("name = \"lost\"\n[[fighter]]\nname = \"a\"\nbase = \"mario\"\n").unwrap_err();
        assert_eq!(failure.line, 1);
    }
}
