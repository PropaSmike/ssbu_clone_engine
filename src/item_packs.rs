#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemPackDeclaration {
    pub base_kind: i32,
    pub base_item: Option<String>,
    pub resource_name: String,
    pub agent_name: Option<String>,
    pub ui_id: Option<String>,
    pub training_order: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ItemPackError {
    Malformed { line: usize },
    BadValue { line: usize, key: String },
    MissingResourceName,
    MissingBaseKind,
}

impl ItemPackDeclaration {
    pub fn agent(&self) -> &str {
        self.agent_name.as_deref().unwrap_or(&self.resource_name)
    }

    pub fn ui(&self) -> String {
        self.ui_id
            .clone()
            .unwrap_or_else(|| format!("ui_item_{}", self.resource_name))
    }
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

pub fn parse(text: &str) -> Result<ItemPackDeclaration, ItemPackError> {
    let mut declaration = ItemPackDeclaration::default();
    let mut saw_base = false;
    for (index, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ItemPackError::Malformed { line: index + 1 });
        };
        let key = key.trim();
        let value = value.trim();
        let unquoted = value.trim_matches('"');
        let bad = || ItemPackError::BadValue {
            line: index + 1,
            key: key.to_string(),
        };
        match key {
            "base_kind" => {
                declaration.base_kind = value.parse::<i32>().map_err(|_| bad())?;
                saw_base = true;
            }
            "base_item" => declaration.base_item = Some(unquoted.to_string()),
            "resource_name" => declaration.resource_name = unquoted.to_string(),
            "agent_name" => declaration.agent_name = Some(unquoted.to_string()),
            "ui_id" => declaration.ui_id = Some(unquoted.to_string()),
            "training_order" => {
                declaration.training_order = value.parse::<i32>().map_err(|_| bad())?
            }
            _ => {}
        }
    }
    if declaration.resource_name.is_empty() {
        return Err(ItemPackError::MissingResourceName);
    }
    if !saw_base {
        return Err(ItemPackError::MissingBaseKind);
    }
    Ok(declaration)
}

#[cfg(all(not(test), feature = "item_clone_backend"))]
mod live {
    use super::*;
    use clone_engine_api::{
        CloneItemRegistrationV1, CloneItemUiRegistrationV1, API_VERSION_V1, ITEM_UI_FLAG_TRAINING,
    };
    use std::ffi::CString;

    const MOD_ROOT: &str = "sd:/ultimate/mods";

    const MAX_KIND_PROBES: usize = 8;

    enum RegisterOutcome {
        Registered,
        KindTaken,
        Refused,
    }

    pub fn load_all() {
        let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
            return;
        };
        let mut declarations: Vec<(String, String)> = Vec::new();
        for entry in entries.flatten() {
            let directory = entry.path();
            if !directory.is_dir() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(directory.join("item.toml")) else {
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
            "[itempack] {} item.toml pack(s) under {MOD_ROOT}",
            declarations.len()
        );
        let mut next_kind = crate::item_clones::FIRST_SPARSE_ITEM_KIND;
        for (directory, text) in declarations {
            let declaration = match parse(&text) {
                Ok(declaration) => declaration,
                Err(error) => {
                    skyline::println!(
                        "[itempack] {directory}/item.toml is not readable: {error:?}; skipped"
                    );
                    continue;
                }
            };
            for attempt in 0..MAX_KIND_PROBES {
                let kind = next_kind;
                next_kind += 1;
                match register(&directory, &declaration, kind) {
                    RegisterOutcome::Registered => break,
                    RegisterOutcome::KindTaken if attempt + 1 < MAX_KIND_PROBES => continue,
                    RegisterOutcome::KindTaken => skyline::println!(
                        "[itempack] {directory}: no free item kind in {MAX_KIND_PROBES} tries; skipped"
                    ),
                    RegisterOutcome::Refused => break,
                }
            }
        }
    }

    fn register(
        directory: &str,
        declaration: &ItemPackDeclaration,
        public_kind: i32,
    ) -> RegisterOutcome {
        let (Ok(resource), Ok(agent), Ok(ui)) = (
            CString::new(declaration.resource_name.as_str()),
            CString::new(declaration.agent()),
            CString::new(declaration.ui()),
        ) else {
            skyline::println!("[itempack] {directory}: names must not contain NUL; skipped");
            return RegisterOutcome::Refused;
        };

        let registration = CloneItemRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneItemRegistrationV1>() as u32,
            item_kind: public_kind,
            base_item_kind: declaration.base_kind,
            resource_name: resource.as_ptr(),
            agent_name: agent.as_ptr(),
            flags: 0,
            reserved_u32: 0,
            reserved: [0; 4],
        };
        let result = unsafe { crate::item_clones::clone_engine_register_item_v1(&registration) };
        if result != 0 {
            skyline::println!(
                "[itempack] {directory}: REFUSED public={public_kind:#x} base={} result={result}",
                declaration.base_kind
            );
            return if result == clone_engine_api::ERROR_DUPLICATE {
                RegisterOutcome::KindTaken
            } else {
                RegisterOutcome::Refused
            };
        }

        let ui_registration = CloneItemUiRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneItemUiRegistrationV1>() as u32,
            item_kind: public_kind,
            flags: ITEM_UI_FLAG_TRAINING,
            ui_id: ui.as_ptr(),
            training_order: declaration.training_order,
            rules_order: 0,
            reserved: [0; 4],
        };
        #[cfg(feature = "item_ui_backend")]
        let ui_result =
            unsafe { crate::item_ui::clone_engine_register_item_ui_v1(&ui_registration) };
        #[cfg(not(feature = "item_ui_backend"))]
        let ui_result = {
            let _ = &ui_registration;
            i32::MIN
        };
        skyline::println!(
            "[itempack] {directory}: public={public_kind:#x} base={} ({}) resource={} ui={} ui_result={ui_result}",
            declaration.base_kind,
            declaration.base_item.as_deref().unwrap_or("?"),
            declaration.resource_name,
            declaration.ui()
        );
        RegisterOutcome::Registered
    }
}

#[cfg(all(not(test), feature = "item_clone_backend"))]
pub(crate) fn load_all() {
    live::load_all();
}

#[cfg(not(all(not(test), feature = "item_clone_backend")))]
pub(crate) fn load_all() {}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_PACK: &str = r#"
# Example, built on Killing Edge.
base_kind     = 63
base_item     = "killsword"
resource_name = "wawa"
"#;

    #[test]
    fn reads_a_minimal_pack() {
        let declaration = parse(EXAMPLE_PACK).unwrap();
        assert_eq!(declaration.base_kind, 63);
        assert_eq!(declaration.resource_name, "wawa");
        assert_eq!(declaration.base_item.as_deref(), Some("killsword"));
    }

    #[test]
    fn the_agent_defaults_to_the_resource_namespace() {
        assert_eq!(parse(EXAMPLE_PACK).unwrap().agent(), "wawa");
    }

    #[test]
    fn the_ui_id_defaults_to_the_shipped_convention() {
        assert_eq!(parse(EXAMPLE_PACK).unwrap().ui(), "ui_item_wawa");
    }

    #[test]
    fn an_explicit_agent_and_ui_id_win() {
        let declaration = parse(
            "base_kind = 64\nresource_name = \"bonk\"\nagent_name = \"other\"\nui_id = \"ui_item_custom\"\n",
        )
        .unwrap();
        assert_eq!(declaration.agent(), "other");
        assert_eq!(declaration.ui(), "ui_item_custom");
    }

    #[test]
    fn a_pack_without_a_base_is_refused() {
        assert_eq!(
            parse("resource_name = \"wawa\"\n"),
            Err(ItemPackError::MissingBaseKind)
        );
    }

    #[test]
    fn a_pack_without_a_namespace_is_refused() {
        assert_eq!(
            parse("base_kind = 63\n"),
            Err(ItemPackError::MissingResourceName)
        );
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let declaration =
            parse("base_kind = 63\nresource_name = \"wawa\"\nfuture_key = \"whatever\"\n").unwrap();
        assert_eq!(declaration.resource_name, "wawa");
    }

    #[test]
    fn a_comment_inside_a_quoted_value_is_data() {
        let declaration =
            parse("base_kind = 63\nresource_name = \"wawa\"\nbase_item = \"a # b\"\n").unwrap();
        assert_eq!(declaration.base_item.as_deref(), Some("a # b"));
    }

    #[test]
    fn a_non_numeric_base_kind_names_its_line() {
        assert_eq!(
            parse("base_kind = \"killsword\"\nresource_name = \"wawa\"\n"),
            Err(ItemPackError::BadValue {
                line: 1,
                key: "base_kind".to_string()
            })
        );
    }

    #[test]
    fn a_line_that_is_not_a_pair_is_malformed() {
        assert_eq!(
            parse("base_kind = 63\nresource_name\n"),
            Err(ItemPackError::Malformed { line: 2 })
        );
    }
}
