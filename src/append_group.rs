use core::sync::atomic::{AtomicU32, Ordering};

use crate::finalsmash_residency::{
    directory_index_for, directory_load_flags, fiber_note, resource_service,
};

const OFF_LOAD_DIRECTORY: usize = 0x3543ad0;
const OFF_LOAD_CONTEXT_REGISTER: usize = 0x17e2bf0;
const APPEND_OWNER_KIND: i32 = 82;
const CTX_COLOR: usize = 0x64;
const CTX_NAME_FLAG: usize = 0x7d;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const LOG_BUDGET: u32 = 8;

static LOG: AtomicU32 = AtomicU32::new(0);

#[skyline::from_offset(OFF_LOAD_DIRECTORY)]
unsafe fn load_directory(service: u64, directory: u32) -> usize;

#[skyline::from_offset(OFF_LOAD_CONTEXT_REGISTER)]
unsafe fn register_directory(context: u64, key: u64, directory: *const u32);

unsafe fn directory_at(service: usize, path: &str) -> Option<(u32, u64)> {
    let hash = crate::hash40::hash40(path);
    directory_index_for(service, hash).map(|index| (index, hash))
}

unsafe fn directory_named(service: usize, name: &str, color: u32) -> Option<(u32, u64)> {
    directory_at(service, &format!("fighter/{name}/append/c{color:02}"))
}

unsafe fn request(service: usize, context: usize, directory: Option<(u32, u64)>) -> u64 {
    let Some((directory, hash)) = directory else {
        return 0xdead;
    };
    load_directory(service as u64, directory);
    let mut flags = directory_load_flags(service, directory);
    if flags & 1 == 0 {
        load_directory(service as u64, directory);
        flags = directory_load_flags(service, directory);
    }
    let index: u32 = directory;
    register_directory(context as u64, hash, &index);
    flags
}

pub(crate) unsafe fn request_append_group(context: usize, kind: i32) {
    if context < LOWEST_PLAUSIBLE_POINTER
        || core::ptr::read_volatile((context + CTX_NAME_FLAG) as *const u8) != 0
    {
        return;
    }
    let Some(definition) = crate::clone_definition(kind) else {
        return;
    };
    let service = resource_service();
    if service < LOWEST_PLAUSIBLE_POINTER {
        return;
    }
    let color = core::ptr::read_volatile((context + CTX_COLOR) as *const u32);
    let movie = directory_at(
        service,
        &format!("fighter/{}/movie/c{color:02}", definition.base_resource_name),
    );
    let movie_flags = request(service, context, movie);
    if definition.base_kind != APPEND_OWNER_KIND {
        if movie.is_some() && LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
            fiber_note(
                b"[movie]",
                &[
                    (b"kind", kind as u32 as u64),
                    (b"color", color as u64),
                    (b"group", movie.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                    (b"group_flags", movie_flags),
                ],
            );
        }
        return;
    }
    let own = directory_named(service, definition.resource_name, color);
    let base = directory_named(service, definition.base_resource_name, color);
    let own_flags = request(service, context, own);
    let base_flags = request(service, context, base);
    let prebuilt = directory_at(
        service,
        &format!("prebuilt:/movie/fighter/{}/c{color:02}", definition.base_resource_name),
    );
    let prebuilt_flags = request(service, context, prebuilt);
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        fiber_note(
            b"[append]",
            &[
                (b"kind", kind as u32 as u64),
                (b"color", color as u64),
                (b"own", own.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"own_flags", own_flags),
                (b"base", base.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"base_flags", base_flags),
                (b"movie", movie.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"movie_flags", movie_flags),
                (b"prebuilt", prebuilt.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"prebuilt_flags", prebuilt_flags),
            ],
        );
    }
}

pub(crate) fn install() {
    crate::dbg_log_public("[append] armed: a clone's base movie group, and a Joker clone's append groups, are requested from the load pass");
}
