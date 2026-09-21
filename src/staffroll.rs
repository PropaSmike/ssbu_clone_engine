use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

struct Served {
    base_path_hash: u64,
    kind: i32,
    file: String,
    size: usize,
}

fn served() -> &'static Mutex<Vec<Served>> {
    static SERVED: OnceLock<Mutex<Vec<Served>>> = OnceLock::new();
    SERVED.get_or_init(|| Mutex::new(Vec::new()))
}

fn used() -> &'static Mutex<Vec<i32>> {
    static USED: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();
    USED.get_or_init(|| Mutex::new(Vec::new()))
}

static INSTALLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn mark_used(kind: i32) {
    if let Ok(mut list) = used().lock() {
        if !list.contains(&kind) {
            list.push(kind);
        }
    }
}

fn was_used(kind: i32) -> bool {
    used().lock().map(|list| list.contains(&kind)).unwrap_or(false)
}

pub(crate) fn texture_path(name: &str) -> String {
    format!("standard/staffroll/texture/standard_staffroll_{name}.nutexb")
}

extern "C" fn serve(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    if buffer.is_null() || length == 0 {
        return false;
    }
    let chosen = served().lock().ok().and_then(|entries| {
        entries
            .iter()
            .find(|entry| entry.base_path_hash == hash && was_used(entry.kind))
            .map(|entry| (entry.kind, entry.file.clone()))
    });
    let Some((kind, file)) = chosen else {
        return false;
    };
    let Ok(bytes) = std::fs::read(&file) else {
        skyline::println!("[staffroll] kind {kind}: {file} could not be read; the base's texture plays");
        return false;
    };
    if bytes.len() > length {
        skyline::println!(
            "[staffroll] kind {kind}: {file} is {} bytes but {length} were reserved; the base's texture plays",
            bytes.len()
        );
        return false;
    }
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    *out_size = bytes.len();
    true
}

pub(crate) fn install(kind: i32, directory: &str, resource_name: &str, base_resource_name: &str) {
    let file = format!(
        "{}/{directory}/{}",
        crate::fighter_packs::MOD_ROOT,
        texture_path(resource_name)
    );
    let Ok(metadata) = std::fs::metadata(&file) else {
        skyline::println!(
            "[staffroll] kind {kind}: staffroll = true but {file} is missing; nothing is served"
        );
        return;
    };
    let size = metadata.len() as usize;
    let base_path = texture_path(base_resource_name);
    let base_path_hash = clone_engine_core::hash40(&base_path);
    let register = served()
        .lock()
        .map(|entries| !entries.iter().any(|entry| entry.base_path_hash == base_path_hash))
        .unwrap_or(false);
    if let Ok(mut entries) = served().lock() {
        entries.push(Served {
            base_path_hash,
            kind,
            file: file.clone(),
            size,
        });
    }
    if register {
        let capacity = served()
            .lock()
            .map(|entries| {
                entries
                    .iter()
                    .filter(|entry| entry.base_path_hash == base_path_hash)
                    .map(|entry| entry.size)
                    .max()
                    .unwrap_or(size)
            })
            .unwrap_or(size)
            .max(size);
        arcropolis_api::register_callback(base_path.as_str(), capacity.max(4 << 20), serve);
        INSTALLED.store(true, Ordering::Release);
    }
    skyline::println!(
        "[staffroll] kind {kind}: {file} ({size} bytes) replaces {base_path} after the clone was picked"
    );
}
