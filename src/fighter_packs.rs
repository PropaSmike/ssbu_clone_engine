use clone_engine_core::fighter_toml::{FighterDeclaration, ParamDeclaration};
use std::sync::{Mutex, OnceLock};

pub(crate) const MOD_ROOT: &str = "sd:/ultimate/mods";

pub(crate) struct LoadedPack {
    pub(crate) directory: &'static str,
    pub(crate) kind: i32,
    pub(crate) declaration: FighterDeclaration,
}

fn loaded() -> &'static Mutex<Vec<LoadedPack>> {
    static LOADED: OnceLock<Mutex<Vec<LoadedPack>>> = OnceLock::new();
    LOADED.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn pack_directory(kind: i32) -> Option<&'static str> {
    let packs = loaded().lock().ok()?;
    packs
        .iter()
        .find(|pack| pack.kind == kind)
        .map(|pack| pack.directory)
}

pub(crate) fn params_for_kind(kind: i32) -> Vec<ParamDeclaration> {
    loaded()
        .lock()
        .ok()
        .and_then(|packs| {
            packs
                .iter()
                .find(|pack| pack.kind == kind)
                .map(|pack| pack.declaration.params.clone())
        })
        .unwrap_or_default()
}

pub(crate) fn loaded_kinds() -> Vec<i32> {
    loaded()
        .lock()
        .map(|packs| packs.iter().map(|pack| pack.kind).collect())
        .unwrap_or_default()
}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
mod live {
    use super::*;
    use crate::{ManifestCss, ManifestRegistration};
    use clone_engine_api::{
        CloneArticleRegistrationV1, CloneCopyMeshV1, CloneCopyModelV1, CloneCopyMotionV1,
        CloneRegistrationV1, API_VERSION_V1, COPY_MOTION_NO_EXTRA, COPY_MOTION_SET_ANIMATION_UNK,
        COPY_MOTION_SET_BLEND_FRAMES, COPY_MOTION_SET_CANCEL_FRAME, COPY_MOTION_SET_FLAGS,
        COPY_MOTION_SET_GAME_SCRIPT, COPY_MOTION_SET_NO_STOP_INTP, COPY_MOTION_SET_SCRIPTS,
        COPY_MOTION_SET_XLU, FLAG_KIRBY_COPY_FULL_MODEL, FLAG_OWNS_PARAM_RESOURCES, KIND_AUTO,
    };
    use clone_engine_core::fighter_toml::{parse_all, ArticleDeclaration, KirbyDeclaration};
    use std::ffi::CString;

    fn leak(text: &str) -> &'static str {
        Box::leak(text.to_string().into_boxed_str())
    }

    fn cstring(directory: &str, what: &str, text: &str) -> Option<CString> {
        match CString::new(text) {
            Ok(value) => Some(value),
            Err(_) => {
                skyline::println!("[fighterpack] {directory}: {what} must not contain NUL");
                None
            }
        }
    }

    pub fn load_all() {
        let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
            return;
        };
        let mut manifests: Vec<(String, String)> = Vec::new();
        for entry in entries.flatten() {
            let directory = entry.path();
            if !directory.is_dir() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(directory.join("fighter.toml")) else {
                continue;
            };
            manifests.push((
                directory
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                text,
            ));
        }
        manifests.sort_by(|a, b| a.0.cmp(&b.0));
        if manifests.is_empty() {
            return;
        }
        skyline::println!(
            "[fighterpack] {} fighter.toml pack(s) under {MOD_ROOT}",
            manifests.len()
        );
        for (directory, text) in manifests {
            let parsed = match parse_all(&text) {
                Ok(parsed) => parsed,
                Err(error) => {
                    skyline::println!(
                        "[fighterpack] {directory}/fighter.toml line {}: {}; the pack is not registered",
                        error.line,
                        error.message
                    );
                    continue;
                }
            };
            if parsed.len() > 1 {
                skyline::println!("[fighterpack] {directory}/fighter.toml: {} fighters", parsed.len());
            }
            let directory: &'static str = leak(&directory);
            for declaration in parsed {
                register(directory, declaration);
            }
        }
    }

    fn directory_shipping(resource: &str) -> Option<&'static str> {
        let entries = std::fs::read_dir(MOD_ROOT).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("fighter").join(resource).is_dir() {
                return Some(leak(&entry.file_name().to_string_lossy()));
            }
        }
        None
    }

    pub fn register_text(label: &str, text: &str) -> i32 {
        let parsed = match parse_all(text) {
            Ok(parsed) => parsed,
            Err(error) => {
                skyline::println!(
                    "[fighterpack] {label} (plugin manifest) line {}: {}; not registered",
                    error.line,
                    error.message
                );
                return clone_engine_api::ERROR_MANIFEST;
            }
        };
        if parsed.is_empty() {
            return clone_engine_api::ERROR_MANIFEST;
        }
        let mut first = clone_engine_api::ERROR_MANIFEST;
        for declaration in parsed {
            let directory = directory_shipping(&declaration.resource_name()).unwrap_or_else(|| leak(label));
            if let Some(kind) = register(directory, declaration) {
                if first < 0 {
                    first = kind;
                }
            }
        }
        first
    }

    fn register(directory: &'static str, declaration: FighterDeclaration) -> Option<i32> {
        let Some(base_kind) = crate::custom_articles::fighter_kind_from_str(&declaration.base) else {
            skyline::println!(
                "[fighterpack] {directory}: base {:?} is not a fighter name; the pack is not registered",
                declaration.base
            );
            return None;
        };
        let name = declaration.name.clone();
        let (Some(ui_chara), Some(fighter_kind_name), Some(resource_name), Some(base_resource_name)) = (
            cstring(directory, "ui_chara", &declaration.ui_chara()),
            cstring(directory, "fighter_kind_name", &declaration.fighter_kind_name()),
            cstring(directory, "name", &declaration.resource_name()),
            cstring(directory, "base", &declaration.base_resource_name()),
        ) else {
            return None;
        };

        let mut base_article_names: Vec<CString> = Vec::new();
        let mut base_articles: Vec<CloneArticleRegistrationV1> = Vec::new();
        for article in declaration
            .articles
            .iter()
            .filter(|article| article.base_article.is_some())
        {
            let weapon = article.base_article.as_deref().unwrap_or_default();
            let Some(weapon_kind) =
                crate::custom_articles::weapon_kind_from_names(&declaration.base, weapon)
            else {
                skyline::println!(
                    "[fighterpack] {directory}: {} has no article {weapon:?}; the pack is not registered",
                    declaration.base
                );
                return None;
            };
            let Some(file_name) = cstring(directory, "article name", &article.name) else {
                return None;
            };
            base_article_names.push(file_name);
            base_articles.push(CloneArticleRegistrationV1 {
                base_weapon_kind: weapon_kind,
                reserved: 0,
                file_name: base_article_names.last().map(|name| name.as_ptr()).unwrap(),
            });
        }

        let mut flags = 0;
        if declaration.owns_param_resources {
            flags |= FLAG_OWNS_PARAM_RESOURCES;
        }
        let kirby = declaration.kirby.clone();
        if kirby.as_ref().is_some_and(|kirby| kirby.full_model) {
            flags |= FLAG_KIRBY_COPY_FULL_MODEL;
        }
        let (copy_status_first, copy_status_count) = match &kirby {
            Some(kirby) if kirby.statuses > 0 => (kirby.first.unwrap_or(0), kirby.statuses),
            _ => (-1, 0),
        };

        let registration = CloneRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneRegistrationV1>() as u32,
            custom_kind: KIND_AUTO,
            base_kind,
            ui_chara: ui_chara.as_ptr(),
            fighter_kind_name: fighter_kind_name.as_ptr(),
            resource_name: resource_name.as_ptr(),
            base_resource_name: base_resource_name.as_ptr(),
            color_start: declaration.color_start,
            color_count: declaration.color_count,
            copy_status_first,
            copy_status_count,
            article_namespace: declaration.article_namespace,
            effect_namespace: declaration.effect_namespace,
            articles: if base_articles.is_empty() {
                core::ptr::null()
            } else {
                base_articles.as_ptr()
            },
            article_count: base_articles.len() as u32,
            flags,
            reserved: [0; 4],
        };
        let manifest = ManifestRegistration {
            css: (!declaration.own_css).then(|| ManifestCss {
                ui_name: leak(&declaration.display_name()),
                ui_series: leak(&declaration.series()),
                disp_order: declaration.disp_order.unwrap_or(-1),
                save_no: declaration.save_no.unwrap_or(0),
                exhibit_year: declaration.exhibit_year.unwrap_or(0),
                narration: declaration.narration.as_deref().map(leak),
            }),
        };
        let result = unsafe { crate::register_v1_with(&registration, Some(manifest)) };
        if result < 0 {
            skyline::println!(
                "[fighterpack] {directory}: {name} REFUSED by the engine (result {result}); the pack is not registered"
            );
            return None;
        }
        let kind = result;
        skyline::println!(
            "[fighterpack] {directory}: {name} is kind {kind} on {} ({base_kind}), costumes {}..{}, {} base article(s){}",
            declaration.base,
            declaration.color_start,
            declaration.color_start + declaration.color_count - 1,
            base_articles.len(),
            if declaration.own_css {
                "; the pack publishes its own CSS row"
            } else {
                ""
            }
        );

        for article in declaration
            .articles
            .iter()
            .filter(|article| article.from.is_some())
        {
            mint_article(directory, kind, &declaration, article);
        }
        if let Some(kirby) = &kirby {
            register_kirby(directory, kind, kirby);
        }
        if declaration.jingle.is_some() {
            skyline::println!(
                "[fighterpack] {directory}: jingle is accepted but not served by this engine yet"
            );
        }
        if declaration.staffroll {
            crate::staffroll::install(
                kind,
                directory,
                &declaration.resource_name(),
                &declaration.base_resource_name(),
            );
        }
        if let Ok(mut packs) = loaded().lock() {
            packs.push(LoadedPack {
                directory,
                kind,
                declaration,
            });
        }
        Some(kind)
    }

    fn mint_article(
        directory: &str,
        kind: i32,
        declaration: &FighterDeclaration,
        article: &ArticleDeclaration,
    ) {
        let Some((owner, weapon)) = article.from.as_ref() else {
            return;
        };
        let Some(weapon_kind) = crate::custom_articles::weapon_kind_from_names(owner, weapon) else {
            skyline::println!(
                "[fighterpack] {directory}: no weapon {owner}/{weapon} for article {}; skipped",
                article.name
            );
            return;
        };
        let resource_owner = if article.kirby {
            "kirby".to_string()
        } else {
            declaration.resource_name()
        };
        let (Some(owner_c), Some(clone_c), Some(name_c)) = (
            cstring(directory, "article owner", owner),
            cstring(directory, "name", &resource_owner),
            cstring(directory, "article name", &article.name),
        ) else {
            return;
        };
        let minted = if article.kirby {
            unsafe {
                crate::custom_articles::register_kirby_copy(
                    kind,
                    owner_c.as_ptr(),
                    weapon_kind,
                    clone_c.as_ptr(),
                    name_c.as_ptr(),
                )
            }
        } else {
            unsafe {
                crate::custom_articles::register(
                    owner_c.as_ptr(),
                    weapon_kind,
                    clone_c.as_ptr(),
                    core::ptr::null(),
                    name_c.as_ptr(),
                )
            }
        };
        if minted < 0 {
            skyline::println!(
                "[fighterpack] {directory}: article {} from {owner}/{weapon} REFUSED (result {minted})",
                article.name
            );
        } else {
            skyline::println!(
                "[fighterpack] {directory}: article {} from {owner}/{weapon} is weapon kind {minted}{}",
                article.name,
                if article.kirby { " (Kirby copy)" } else { "" }
            );
        }
    }

    fn register_kirby(directory: &str, kind: i32, kirby: &KirbyDeclaration) {
        if let Some(model) = &kirby.model {
            if let Some(model_c) = cstring(directory, "kirby model", model) {
                let registration = CloneCopyModelV1 {
                    api_version: API_VERSION_V1,
                    struct_size: core::mem::size_of::<CloneCopyModelV1>() as u32,
                    fighter_kind: kind,
                    reserved0: 0,
                    directory: model_c.as_ptr(),
                    reserved: [0; 4],
                };
                let result =
                    unsafe { crate::kirby_copy::clone_engine_clone_copy_model_v1(&registration) };
                skyline::println!("[fighterpack] {directory}: kirby model {model} result={result}");
            }
        }
        for (mesh, visible) in &kirby.meshes {
            let Some(mesh_c) = cstring(directory, "kirby mesh", mesh) else {
                continue;
            };
            let registration = CloneCopyMeshV1 {
                api_version: API_VERSION_V1,
                struct_size: core::mem::size_of::<CloneCopyMeshV1>() as u32,
                fighter_kind: kind,
                visible: u32::from(*visible),
                mesh: mesh_c.as_ptr(),
                reserved: [0; 4],
            };
            let result =
                unsafe { crate::kirby_copy::clone_engine_clone_copy_mesh_default_v1(&registration) };
            skyline::println!("[fighterpack] {directory}: kirby mesh {mesh} result={result}");
        }
        for motion in &kirby.motions {
            let owned = |value: &str| cstring(directory, "kirby motion", value);
            let (Some(name), Some(animation)) = (owned(&motion.name), owned(&motion.animation)) else {
                continue;
            };
            let template = motion.template.as_deref().and_then(owned);
            let game = motion.script("game", None).as_deref().and_then(owned);
            let sound = motion.script("sound", None).as_deref().and_then(owned);
            let effect = motion.script("effect", None).as_deref().and_then(owned);
            let expression = motion.script("expression", None).as_deref().and_then(owned);
            let mut present = 0u32;
            let mut motion_flags = 0u16;
            if game.is_some() {
                present |= COPY_MOTION_SET_GAME_SCRIPT;
            }
            if sound.is_some() && effect.is_some() && expression.is_some() {
                present |= COPY_MOTION_SET_SCRIPTS;
            }
            if !motion.flags.is_empty() {
                present |= COPY_MOTION_SET_FLAGS;
                for flag in &motion.flags {
                    motion_flags |= match flag.as_str() {
                        "turn" => 1 << 0,
                        "loop" => 1 << 1,
                        "move" => 1 << 2,
                        "fix_trans" => 1 << 3,
                        "fix_rot" => 1 << 4,
                        "fix_scale" => 1 << 5,
                        other => {
                            skyline::println!(
                                "[fighterpack] {directory}: kirby motion {} has unknown flag {other:?}",
                                motion.name
                            );
                            0
                        }
                    };
                }
            }
            if motion.blend_frames.is_some() {
                present |= COPY_MOTION_SET_BLEND_FRAMES;
            }
            if motion.xlu.is_some() {
                present |= COPY_MOTION_SET_XLU;
            }
            if motion.cancel_frame.is_some() {
                present |= COPY_MOTION_SET_CANCEL_FRAME;
            }
            if motion.no_stop_intp.is_some() {
                present |= COPY_MOTION_SET_NO_STOP_INTP;
            }
            if motion.animation_unk.is_some() {
                present |= COPY_MOTION_SET_ANIMATION_UNK;
            }
            if motion.no_extra {
                present |= COPY_MOTION_NO_EXTRA;
            }
            let null = core::ptr::null();
            let pointer = |value: &Option<CString>| value.as_ref().map_or(null, |value| value.as_ptr());
            let (xlu_start, xlu_end) = motion.xlu.unwrap_or((0, 0));
            let registration = CloneCopyMotionV1 {
                api_version: API_VERSION_V1,
                struct_size: core::mem::size_of::<CloneCopyMotionV1>() as u32,
                fighter_kind: kind,
                present,
                name: name.as_ptr(),
                template: pointer(&template),
                animation: animation.as_ptr(),
                game_script: pointer(&game),
                script_expression: pointer(&expression),
                script_sound: pointer(&sound),
                script_effect: pointer(&effect),
                motion_flags,
                blend_frames: motion.blend_frames.unwrap_or(0),
                xlu_start,
                xlu_end,
                cancel_frame: motion.cancel_frame.unwrap_or(0),
                no_stop_intp: u8::from(motion.no_stop_intp.unwrap_or(false)),
                animation_unk: motion.animation_unk.unwrap_or(0),
                reserved: [0; 4],
            };
            let result =
                unsafe { crate::kirby_motions::clone_engine_clone_copy_motion_v1(&registration) };
            skyline::println!(
                "[fighterpack] {directory}: kirby motion {} result={result}",
                motion.name
            );
        }
    }

    pub fn apply_params() {
        for kind in loaded_kinds() {
            let params = params_for_kind(kind);
            if params.is_empty() {
                continue;
            }
            if !crate::param_overrides::available() {
                skyline::println!(
                    "[fighterpack] kind {kind}: {} [params] entries need ParamConfig, which is not loaded",
                    params.len()
                );
                continue;
            }
            let mut applied = 0;
            for param in &params {
                let (name, sub) = match param.name.split_once('.') {
                    Some((name, sub)) => (name, clone_engine_core::hash40(sub)),
                    None => (param.name.as_str(), 0),
                };
                let slot = param.slot.unwrap_or(crate::param_overrides::ANY_SLOT);
                let result = match (&param.value, param.op) {
                    (clone_engine_core::fighter_toml::ParamValue::Int(value), _) => {
                        crate::clone_engine_param_int_override_v1(
                            kind,
                            slot,
                            clone_engine_core::hash40(name),
                            sub,
                            *value,
                        )
                    }
                    (clone_engine_core::fighter_toml::ParamValue::Float(value), op) => {
                        crate::clone_engine_param_override_v1(
                            kind,
                            slot,
                            clone_engine_core::hash40(name),
                            sub,
                            match op {
                                clone_engine_core::fighter_toml::ParamOp::Set => {
                                    crate::param_overrides::OP_SET
                                }
                                clone_engine_core::fighter_toml::ParamOp::Mul => {
                                    crate::param_overrides::OP_MUL
                                }
                            },
                            *value,
                        )
                    }
                };
                if result == 0 {
                    applied += 1;
                } else {
                    skyline::println!(
                        "[fighterpack] kind {kind}: [params] {} was declined (result {result})",
                        param.name
                    );
                }
            }
            skyline::println!("[fighterpack] kind {kind}: {applied} of {} [params] applied", params.len());
        }
    }
}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
pub(crate) fn load_all() {
    live::load_all();
}

#[cfg(not(all(not(test), feature = "native_relocate", feature = "css_slot")))]
pub(crate) fn load_all() {}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
pub(crate) fn apply_params() {
    live::apply_params();
}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
pub(crate) fn register_text(label: &str, text: &str) -> i32 {
    live::register_text(label, text)
}

#[cfg(not(all(not(test), feature = "native_relocate", feature = "css_slot")))]
pub(crate) fn register_text(_label: &str, _text: &str) -> i32 {
    clone_engine_api::ERROR_UNSUPPORTED
}

#[cfg(not(all(not(test), feature = "native_relocate", feature = "css_slot")))]
pub(crate) fn apply_params() {}
