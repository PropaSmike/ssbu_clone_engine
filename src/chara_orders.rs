use clone_engine_core::csk_database::chara_disp_orders;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const MODS_DATABASE: &str = "mods:/database";

static ORDERS: RwLock<Option<HashMap<u64, i8>>> = RwLock::new(None);

fn json_files(directory: &Path) -> Option<Vec<PathBuf>> {
    let entries = std::fs::read_dir(directory).ok()?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect();
    files.sort();
    Some(files)
}

fn read_orders(files: &[PathBuf]) -> HashMap<u64, i8> {
    let mut orders = HashMap::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(file) else {
            crate::dbg_log_public(&format!("[cskorder] {} could not be read", file.display()));
            continue;
        };
        match chara_disp_orders(&text) {
            Ok(found) => {
                for refused in &found.refused {
                    crate::dbg_log_public(&format!("[cskorder] {}: {refused}", file.display()));
                }
                for (ui_chara, order) in found.orders {
                    orders.insert(ui_chara, order);
                }
            }
            Err(error) => crate::dbg_log_public(&format!(
                "[cskorder] {}: not JSON at byte {} ({}); no order taken from it",
                file.display(),
                error.offset,
                error.message
            )),
        }
    }
    crate::dbg_log_public(&format!(
        "[cskorder] {MODS_DATABASE}: {} file(s), {} character disp_order(s)",
        files.len(),
        orders.len()
    ));
    orders
}

fn load() -> HashMap<u64, i8> {
    match json_files(Path::new(MODS_DATABASE)) {
        Some(files) => read_orders(&files),
        None => HashMap::new(),
    }
}

pub(crate) fn player_disp_order(ui_chara: u64) -> Option<i8> {
    if let Some(orders) = ORDERS.read().ok()?.as_ref() {
        return orders.get(&ui_chara).copied();
    }
    if !crate::mount_events::mods_fs_mounted() {
        return None;
    }
    let orders = load();
    let found = orders.get(&ui_chara).copied();
    if let Ok(mut held) = ORDERS.write() {
        held.get_or_insert(orders);
    }
    found
}
