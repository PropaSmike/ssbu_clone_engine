#![allow(dead_code)]

use crate::stage_ledger::{CloneStage, Form};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackDeclaration {
    pub place: String,
    pub display_name: String,
    pub id_name: Option<String>,
    pub forms: Vec<Form>,
    pub ships_battle_tree: bool,
    pub series: String,
    pub disp_order: i32,
    pub donor: Option<String>,
    pub resource_place: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Malformed { line: usize },
    BadValue { line: usize, key: String },
    MissingPlace,
    UnknownForm { line: usize, form: String },
}

enum Value {
    Text(String),
    Number(i64),
    Flag(bool),
    List(Vec<String>),
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

fn parse_value(raw: &str) -> Option<Value> {
    let raw = raw.trim();
    if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let mut items = Vec::new();
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let text = item.strip_prefix('"')?.strip_suffix('"')?;
            items.push(text.to_string());
        }
        return Some(Value::List(items));
    }
    if let Some(text) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return Some(Value::Text(text.to_string()));
    }
    match raw {
        "true" => Some(Value::Flag(true)),
        "false" => Some(Value::Flag(false)),
        _ => raw.parse::<i64>().ok().map(Value::Number),
    }
}

pub fn parse(text: &str) -> Result<PackDeclaration, ParseError> {
    let mut out = PackDeclaration {
        forms: vec![Form::Normal],
        ..Default::default()
    };
    let mut saw_forms = false;

    for (index, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let number = index + 1;
        let Some((key, value)) = line.split_once('=') else {
            return Err(ParseError::Malformed { line: number });
        };
        let key = key.trim();
        let Some(value) = parse_value(value) else {
            return Err(ParseError::BadValue {
                line: number,
                key: key.to_string(),
            });
        };
        let bad = || ParseError::BadValue {
            line: number,
            key: key.to_string(),
        };
        match (key, value) {
            ("place", Value::Text(text)) => out.place = text.to_ascii_lowercase(),
            ("display_name", Value::Text(text)) => out.display_name = text,
            ("id_name", Value::Text(text)) => out.id_name = Some(text),
            ("series", Value::Text(text)) => out.series = text.to_ascii_lowercase(),
            ("donor", Value::Text(text)) => out.donor = Some(text.to_ascii_lowercase()),
            ("resource_place", Value::Text(text)) => {
                out.resource_place = Some(text.to_ascii_lowercase())
            }
            ("ships_battle_tree", Value::Flag(flag)) => out.ships_battle_tree = flag,
            ("disp_order", Value::Number(value)) => out.disp_order = value as i32,
            ("forms", Value::List(items)) => {
                saw_forms = true;
                out.forms.clear();
                for item in items {
                    out.forms.push(match item.to_ascii_lowercase().as_str() {
                        "normal" => Form::Normal,
                        "omega" | "end" => Form::Omega,
                        "battlefield" | "battle" => Form::Battlefield,
                        _ => {
                            return Err(ParseError::UnknownForm {
                                line: number,
                                form: item,
                            })
                        }
                    });
                }
            }
            (
                "place" | "display_name" | "id_name" | "series" | "donor" | "resource_place"
                | "ships_battle_tree" | "disp_order" | "forms",
                _,
            ) => return Err(bad()),
            _ => {}
        }
    }

    if out.place.is_empty() {
        return Err(ParseError::MissingPlace);
    }
    if saw_forms && out.forms.is_empty() {
        out.forms.push(Form::Normal);
    }
    if out.display_name.is_empty() {
        out.display_name = out.place.clone();
    }
    Ok(out)
}

impl PackDeclaration {
    pub fn to_clone_stage(&self) -> CloneStage {
        let mut stage = CloneStage::new(&self.place);
        stage.ships_battle_tree = self.ships_battle_tree;
        stage.forms = self.forms.clone();
        stage.ui_name_id = self.id_name.clone();
        stage.resource_place = self.resource_place.clone();
        stage
    }
}

const MOD_ROOT: &str = "sd:/ultimate/mods";

#[cfg(all(not(test), feature = "stage_mint"))]
pub fn load_all() {
    let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
        skyline::println!("[stagepack] no {MOD_ROOT}; no data-only stage packs to load");
        return;
    };
    let mut declarations: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let manifest = directory.join("stage.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        declarations.push((
            directory
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            text,
        ));
    }
    declarations.sort_by(|a, b| a.0.cmp(&b.0));
    if declarations.is_empty() {
        return;
    }
    skyline::println!(
        "[stagepack] {} stage.toml pack(s) under {MOD_ROOT}",
        declarations.len()
    );
    for (directory, text) in declarations {
        match parse(&text) {
            Ok(declaration) => mint(&directory, &declaration),
            Err(error) => skyline::println!(
                "[stagepack] {directory}/stage.toml is not readable: {error:?}; skipped"
            ),
        }
    }
}

#[cfg(all(not(test), feature = "stage_mint"))]
fn mint(directory: &str, declaration: &PackDeclaration) {
    let stage = declaration.to_clone_stage();
    let place = {
        let Ok(mut registry) = crate::stage_registry::registry().lock() else {
            skyline::println!("[stagepack] {directory}: registry unavailable");
            return;
        };
        if registry.by_name(&stage.place_name).is_some() {
            skyline::println!(
                "[stagepack] {directory}: {} is already minted; leaving it alone",
                stage.place_name
            );
            return;
        }
        match registry.register(&stage) {
            Ok(minted) => minted.place,
            Err(error) => {
                skyline::println!("[stagepack] {directory}: refused: {error:?}");
                return;
            }
        }
    };

    if let Some(donor) = declaration.donor.as_deref() {
        if let Ok(mut registry) = crate::stage_registry::registry().lock() {
            registry.set_behaviour(&stage.place_name, donor);
        }
    }

    skyline::println!(
        "[stagepack] {directory}: minted {} at place {place}, behaviour {}, disp_order {}",
        stage.place_name,
        declaration.donor.as_deref().unwrap_or("(its own)"),
        declaration.disp_order,
    );

    if !crate::stage_registry::registry()
        .lock()
        .map(|mut registry| registry.claim_row(&stage.place_name))
        .unwrap_or(false)
    {
        return;
    }

    match crate::stage_registration::plan(
        &stage,
        crate::stage_ledger::hash40(&declaration.series),
        declaration.disp_order,
    ) {
        Ok(_registration) => {
            #[cfg(feature = "stage_slot")]
            {
                crate::stage_registration::register(&_registration);
                if let Some(wanted) = _registration.deferred_disp_order {
                    crate::stage_db_rows::request_disp_order(_registration.stage_hash, wanted);
                    skyline::println!(
                        "[stagepack] {}: registered hidden; disp_order {wanted} queued for the \
                         row backend (CSK's SignedByteType stops at 127)",
                        stage.place_name
                    );
                }
            }
        }
        Err(error) => skyline::println!(
            "[stagepack] {}: has an identity but no grid row: {error:?}",
            stage.place_name
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUMPKIN: &str = r#"
# Pumpkin Hill, rehoused from `!Knuckles Moveset` onto a MINTED place.
place        = "pumpkin_hill"
display_name = "Pumpkin Hill"
id_name      = "PumpkinHill"
forms        = ["normal"]
ships_battle_tree = false
series       = "sonic"
disp_order   = 119
donor        = "photostage"
"#;

    #[test]
    fn reads_the_pack_that_already_ships() {
        let pack = parse(PUMPKIN).unwrap();
        assert_eq!(pack.place, "pumpkin_hill");
        assert_eq!(pack.display_name, "Pumpkin Hill");
        assert_eq!(pack.id_name.as_deref(), Some("PumpkinHill"));
        assert_eq!(pack.forms, vec![Form::Normal]);
        assert!(!pack.ships_battle_tree);
        assert_eq!(pack.series, "sonic");
        assert_eq!(pack.disp_order, 119);
        assert_eq!(pack.donor.as_deref(), Some("photostage"));
        assert_eq!(pack.resource_place, None);
    }

    #[test]
    fn a_declaration_becomes_the_same_stage_a_plugin_would_build() {
        let stage = parse(PUMPKIN).unwrap().to_clone_stage();
        assert_eq!(stage.place_name, "pumpkin_hill");
        assert_eq!(stage.ui_name_id.as_deref(), Some("PumpkinHill"));
        assert_eq!(stage.forms, vec![Form::Normal]);
        assert_eq!(stage.resource_place, None);
    }

    #[test]
    fn all_three_forms_parse() {
        let pack =
            parse("place = \"x\"\nforms = [\"normal\", \"omega\", \"battlefield\"]").unwrap();
        assert_eq!(
            pack.forms,
            vec![Form::Normal, Form::Omega, Form::Battlefield]
        );
    }

    #[test]
    fn a_hash_inside_a_string_is_not_a_comment() {
        let pack = parse("place = \"x\"\ndisplay_name = \"Stage #2\"").unwrap();
        assert_eq!(pack.display_name, "Stage #2");
    }

    #[test]
    fn a_missing_display_name_falls_back_to_the_place() {
        assert_eq!(parse("place = \"tetris\"").unwrap().display_name, "tetris");
    }

    #[test]
    fn an_unknown_key_is_ignored_but_a_mistyped_known_one_is_not() {
        assert!(parse("place = \"x\"\nfuture_key = \"whatever\"").is_ok());
        assert_eq!(
            parse("place = \"x\"\ndisp_order = \"119\""),
            Err(ParseError::BadValue {
                line: 2,
                key: "disp_order".to_string()
            })
        );
    }

    #[test]
    fn a_pack_without_a_place_is_refused() {
        assert_eq!(
            parse("display_name = \"Nameless\""),
            Err(ParseError::MissingPlace)
        );
    }

    #[test]
    fn a_bad_form_names_itself() {
        assert_eq!(
            parse("place = \"x\"\nforms = [\"normal\", \"omga\"]"),
            Err(ParseError::UnknownForm {
                line: 2,
                form: "omga".to_string()
            })
        );
    }

    #[test]
    fn negative_and_deferred_disp_orders_survive_the_round_trip() {
        assert_eq!(
            parse("place=\"x\"\ndisp_order = -1").unwrap().disp_order,
            -1
        );
        assert_eq!(
            parse("place=\"x\"\ndisp_order = 128").unwrap().disp_order,
            128
        );
    }
}

#[cfg(all(test, feature = "stage_mint"))]
pub fn load_all() {}
