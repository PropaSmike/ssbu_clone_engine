use super::*;

#[cfg(feature = "clone_runtime")]
pub(crate) unsafe fn entry_kind_array(entry_id: i32) -> (u32, i32, i32, i32) {
    if entry_id < 0 {
        return (0xffff_fffc, -9, -9, -9);
    }
    let t1 = *((text_base() + 0x52b84f8) as *const usize);
    if t1 == 0 {
        return (0xffff_ffff, -9, -9, -9);
    }
    let t2 = *(t1 as *const usize);
    if t2 == 0 {
        return (0xffff_fffe, -9, -9, -9);
    }
    let per = *((t2 + entry_id as usize * 8 + 0x20) as *const usize);
    if per == 0 {
        return (0xffff_fffd, -9, -9, -9);
    }
    let count = *((per + 0x14) as *const u32);
    let arr = (per + 0x18) as *const i32;
    let rd = |i: usize| {
        if count <= 8 && (i as u32) < count {
            *arr.add(i)
        } else {
            -1
        }
    };
    (count, rd(0), rd(1), rd(2))
}

#[cfg(feature = "clone_runtime")]
pub(crate) static INIT_PROBE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "clone_runtime")]
pub(crate) static INIT_BRIDGE_GUARDS: [core::sync::atomic::AtomicBool; 8] =
    [const { core::sync::atomic::AtomicBool::new(false) }; 8];

#[cfg(feature = "clone_runtime")]
pub(crate) unsafe fn entry_for_resource_record(record: usize) -> Option<usize> {
    if record == 0 {
        return None;
    }
    let root = *((text_base() + 0x5323680) as *const usize);
    if root == 0 {
        return None;
    }
    for entry in 0..8 {
        if *((root + entry * 8 + 0xe8) as *const usize) == record {
            return Some(entry);
        }
    }
    None
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static SCOPED_RESOURCE_PATH_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(
    feature = "clone_runtime",
    feature = "css_slot",
    feature = "diag_fighter_camera"
))]
pub(crate) static CAMERA_RESOURCE_PATH_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[inline(always)]
pub(crate) fn is_clone_scoped_path_type(path_type: i32, definition: &CloneDefinition) -> bool {
    match path_type {
        12 | 13 => definition.ships_own_param_resources(),
        20 | 22..=31 | 35..=37 => true,
        39 => !CAMERA_ROUTE_DISABLED.load(core::sync::atomic::Ordering::Relaxed),
        _ => false,
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) unsafe fn directory_search_index_is_backed(index: i32) -> bool {
    if index < 0 || index == RESOURCE_INDEX_NOT_FOUND {
        return false;
    }
    let service = *((text_base() + 0x5331f20) as *const usize);
    if service == 0 {
        return false;
    }
    let table = *((service + 0x78) as *const usize);
    if table == 0 {
        return false;
    }
    let root = *((table + 0x8) as *const usize);
    if root == 0 {
        return false;
    }
    let header = *((root + 0x8) as *const usize);
    if header == 0 {
        return false;
    }
    let count = *((header + 0x4) as *const u32);
    if index as u32 >= count {
        return false;
    }
    let redirect = *((root + 0x28) as *const usize);
    if redirect == 0 {
        return true;
    }
    *((redirect + index as usize * 4) as *const u32) != RESOURCE_INDEX_NOT_FOUND as u32
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static CAMERA_FALLBACK_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::from_offset(0x17dd7b0)]
fn resolve_child_in_directory(out: *mut u32, directory: u32, name: u64);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
static CAMERA_NAME_THREADS: [core::sync::atomic::AtomicUsize; 8] =
    [const { core::sync::atomic::AtomicUsize::new(0) }; 8];
#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
static CAMERA_NAME_VALUES: [core::sync::atomic::AtomicU64; 8] =
    [const { core::sync::atomic::AtomicU64::new(0) }; 8];

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn set_pending_camera_name(name: u64) {
    let thread = current_thread_key();
    if thread == 0 {
        return;
    }
    for index in 0..CAMERA_NAME_THREADS.len() {
        let owner = CAMERA_NAME_THREADS[index].load(core::sync::atomic::Ordering::Acquire);
        if owner == thread
            || (owner == 0
                && CAMERA_NAME_THREADS[index]
                    .compare_exchange(
                        0,
                        thread,
                        core::sync::atomic::Ordering::AcqRel,
                        core::sync::atomic::Ordering::Relaxed,
                    )
                    .is_ok())
        {
            CAMERA_NAME_VALUES[index].store(name, core::sync::atomic::Ordering::Release);
            return;
        }
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn pending_camera_name() -> Option<u64> {
    let thread = current_thread_key();
    if thread == 0 {
        return None;
    }
    for index in 0..CAMERA_NAME_THREADS.len() {
        if CAMERA_NAME_THREADS[index].load(core::sync::atomic::Ordering::Acquire) == thread {
            let name = CAMERA_NAME_VALUES[index].load(core::sync::atomic::Ordering::Acquire);
            return (name != 0).then_some(name);
        }
    }
    None
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn clear_pending_camera_name() {
    let thread = current_thread_key();
    if thread == 0 {
        return;
    }
    for index in 0..CAMERA_NAME_THREADS.len() {
        if CAMERA_NAME_THREADS[index].load(core::sync::atomic::Ordering::Acquire) == thread {
            CAMERA_NAME_VALUES[index].store(0, core::sync::atomic::Ordering::Release);
            CAMERA_BASE_DIRECTORIES[index].store(-1, core::sync::atomic::Ordering::Release);
            CAMERA_NAME_THREADS[index].store(0, core::sync::atomic::Ordering::Release);
            return;
        }
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
static CAMERA_BASE_DIRECTORIES: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; 8];

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn set_pending_camera_base_directory(directory: i32) {
    let thread = current_thread_key();
    if thread == 0 {
        return;
    }
    for index in 0..CAMERA_NAME_THREADS.len() {
        if CAMERA_NAME_THREADS[index].load(core::sync::atomic::Ordering::Acquire) == thread {
            CAMERA_BASE_DIRECTORIES[index].store(directory, core::sync::atomic::Ordering::Release);
            return;
        }
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn pending_camera_base_directory() -> Option<i32> {
    let thread = current_thread_key();
    if thread == 0 {
        return None;
    }
    for index in 0..CAMERA_NAME_THREADS.len() {
        if CAMERA_NAME_THREADS[index].load(core::sync::atomic::Ordering::Acquire) == thread {
            let directory =
                CAMERA_BASE_DIRECTORIES[index].load(core::sync::atomic::Ordering::Acquire);
            return (directory >= 0 && directory != RESOURCE_INDEX_NOT_FOUND).then_some(directory);
        }
    }
    None
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
static CAMERA_RESIDENT_KINDS: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; 8];

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
fn camera_is_resident(kind: i32) -> bool {
    CAMERA_RESIDENT_KINDS
        .iter()
        .any(|slot| slot.load(core::sync::atomic::Ordering::Acquire) == kind)
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[allow(dead_code)]
unsafe fn ensure_camera_resident(definition: &CloneDefinition) {
    if camera_is_resident(definition.kind) {
        return;
    }
    let mut loaded = 0usize;
    let mut missing = 0usize;
    for animation in CAMERA_ANIMATIONS {
        for color in 0..8 {
            let path = format!(
                "camera/fighter/{}/c{color:02}/{animation}",
                definition.resource_name
            );
            if fighter_modules::load_file_reporting(&path) == RESOURCE_INDEX_NOT_FOUND as u32 {
                missing += 1;
            } else {
                loaded += 1;
            }
        }
    }
    if loaded == 0 {
        dbg_log!(
            "[camera] {} ships no camera files; base fighter's camera will be used",
            definition.resource_name
        );
        return;
    }
    for slot in CAMERA_RESIDENT_KINDS.iter() {
        if slot
            .compare_exchange(
                -1,
                definition.kind,
                core::sync::atomic::Ordering::AcqRel,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            dbg_log!(
                "[camera] {} camera resident: {loaded} file(s) loaded, {missing} absent",
                definition.resource_name
            );
            return;
        }
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
const CAMERA_ANIMATIONS: [&str; 50] = [
    "d03speciallwcatch.nuanmb",
    "d04final.nuanmb",
    "d04final02.nuanmb",
    "d04final2.nuanmb",
    "d04finalair.nuanmb",
    "d04finalairlockon.nuanmb",
    "d04finalairlockon02.nuanmb",
    "d04finalairstart.nuanmb",
    "d04finalairstart02.nuanmb",
    "d04finalattack.nuanmb",
    "d04finalattack_far.nuanmb",
    "d04finalattackl.nuanmb",
    "d04finalattackl_far.nuanmb",
    "d04finalchange.nuanmb",
    "d04finalcutin.nuanmb",
    "d04finalfinishl.nuanmb",
    "d04finalfinishr.nuanmb",
    "d04finalhit.nuanmb",
    "d04finallockon.nuanmb",
    "d04finallockon02.nuanmb",
    "d04finalmoneyr.nuanmb",
    "d04finalone.nuanmb",
    "d04finalr.nuanmb",
    "d04finalstart.nuanmb",
    "d04finalstart02.nuanmb",
    "d04finalstartl.nuanmb",
    "d04finalstartr.nuanmb",
    "d04finalvisualscene.nuanmb",
    "d04finalvisualscene01.nuanmb",
    "d04finalvisualscene02.nuanmb",
    "d04finalvisualscene03.nuanmb",
    "d04finalvisualsceneattack.nuanmb",
    "d04finalvisualsceneentry.nuanmb",
    "d04visualscene.nuanmb",
    "d04visualscene01.nuanmb",
    "d04visualscene02.nuanmb",
    "d04visualscene03.nuanmb",
    "d04visualscene04.nuanmb",
    "d04visualscene05.nuanmb",
    "e01throwb.nuanmb",
    "e01throwcommand.nuanmb",
    "e01throwf.nuanmb",
    "e01throwhi.nuanmb",
    "e01throwlw.nuanmb",
    "j02win1.nuanmb",
    "j02win1loop.nuanmb",
    "j02win2.nuanmb",
    "j02win2loop.nuanmb",
    "j02win3.nuanmb",
    "j02win3loop.nuanmb",
];

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) fn camera_animation_file_name(hash: u64) -> Option<&'static str> {
    CAMERA_ANIMATIONS
        .into_iter()
        .find(|name| crate::hash40::hash40(name) == hash)
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn camera_animation_resolves(directory: i32, name: u64) -> bool {
    if !directory_search_index_is_backed(directory) {
        return false;
    }
    let mut resolved: u32 = RESOURCE_INDEX_NOT_FOUND as u32;
    resolve_child_in_directory(&mut resolved as *mut u32, directory as u32, name);
    resolved != RESOURCE_INDEX_NOT_FOUND as u32 && resolved != u32::MAX
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = 0x6955d0)]
pub(crate) unsafe fn fighter_camera_set_hook(object: *mut u8, name: u64, holder: *mut u8) -> u64 {
    set_pending_camera_name(name);
    let ret = call_original!(object, name, holder);
    clear_pending_camera_name();
    ret
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = 0x69566c, inline)]
pub(crate) unsafe fn camera_animation_base_fallback(ctx: &mut skyline::hooks::InlineCtx) {
    if ctx.registers[20].x() as u32 != RESOURCE_INDEX_NOT_FOUND as u32 {
        return;
    }
    let (Some(name), Some(base)) = (pending_camera_name(), pending_camera_base_directory()) else {
        return;
    };
    let mut resolved: u32 = RESOURCE_INDEX_NOT_FOUND as u32;
    resolve_child_in_directory(&mut resolved as *mut u32, base as u32, name);
    if resolved == RESOURCE_INDEX_NOT_FOUND as u32 || resolved == u32::MAX {
        return;
    }
    ctx.registers[20].set_x(resolved as u64);
    let n = CAMERA_FALLBACK_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[camfall] #{n} animation={name:#x} missing from clone dir, \
             served from base dir={base:#x} -> {resolved:#x}"
        );
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = OFF_FIGHTER_RESOURCE_PATH_RESOLVE)]
pub(crate) unsafe fn fighter_scoped_resource_path_hook(
    out: *mut i32,
    record: *const u8,
    kind: i32,
    path_type: i32,
) {
    let entry = entry_for_resource_record(record as usize);
    let definition = entry
        .and_then(|entry| entry_custom_kind(entry as u8))
        .and_then(clone_definition)
        .filter(|definition| kind == definition.base_kind);

    if let Some(definition) =
        definition.filter(|definition| is_clone_scoped_path_type(path_type, definition))
    {
        let n = SCOPED_RESOURCE_PATH_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 48 {
            dbg_log!(
                "[clonepath] #{n} entry={entry:?} kind={kind} true_kind={} type={path_type} namespace={}",
                definition.kind,
                definition.resource_name
            );
        }
        let mut base_index: i32 = -1;
        call_original!(&mut base_index as *mut i32, record, kind, path_type);
        if path_type == 20 {
            arm_pending_effect_kind(definition.kind, base_index);
        }
        if path_type == 39 {
            set_pending_camera_base_directory(base_index);
        }
        with_resource_context(definition.kind, || {
            call_original!(out, record, kind, path_type)
        });
        let custom_index = if out.is_null() {
            i32::MIN
        } else {
            core::ptr::read_volatile(out)
        };

        if !out.is_null()
            && (custom_index == RESOURCE_INDEX_NOT_FOUND || custom_index < 0)
            && base_index != RESOURCE_INDEX_NOT_FOUND
            && base_index >= 0
        {
            core::ptr::write_volatile(out, base_index);
        }

        if n < 48 {
            dbg_log!(
                "[effectpath] type={path_type} true_kind={} base={base_index:#x} \
                 custom={custom_index:#x} -> {:#x}",
                definition.kind,
                if out.is_null() {
                    i32::MIN
                } else {
                    core::ptr::read_volatile(out)
                }
            );
        }
        return;
    }

    call_original!(out, record, kind, path_type);
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static MODEL_PATH_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = OFF_MODEL_PATH_RESOLVE)]
pub(crate) unsafe fn model_path_namespace_hook(
    out: *mut i32,
    record: *const u8,
    kind: i32,
    mode: i32,
) {
    let entry = entry_for_resource_record(record as usize);
    let selected_kind = entry.and_then(|entry| entry_custom_kind(entry as u8));
    let definition = selected_kind.and_then(clone_definition);
    let is_custom_entry = definition.is_some();
    let effective_kind = match definition {
        Some(definition) if kind == definition.base_kind => definition.kind,
        _ => kind,
    };

    if let Some(definition) = definition {
        with_resource_context(definition.kind, || {
            call_original!(out, record, effective_kind, mode)
        });
    } else {
        call_original!(out, record, effective_kind, mode);
    }

    if !is_custom_entry && kind != 0 {
        return;
    }
    let n = MODEL_PATH_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n >= 32 {
        return;
    }
    let result = if out.is_null() {
        i32::MIN
    } else {
        core::ptr::read_volatile(out)
    };
    let namespace = definition
        .map(|definition| definition.resource_name)
        .unwrap_or("vanilla");
    dbg_log!(
        "[modelpath54] #{n} entry={entry:?} custom={is_custom_entry} kind={kind}->{effective_kind} namespace={namespace} \
         mode={mode} record={:#x} result={result:#x}",
        record as usize
    );
}

#[cfg(all(feature = "clone_runtime", not(feature = "diag_article_initspoof")))]
#[skyline::hook(offset = OFF_FIGHTER_INIT_OBJ)]
pub(crate) unsafe fn fighter_init_kind_bridge(
    object: *mut u8,
    id: u32,
    kind: i32,
    entry_id: i32,
    name_hash: u64,
) {
    let n = INIT_PROBE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        let (cnt, k0, k1, k2) = entry_kind_array(entry_id);
        dbg_log!(
            "[initprobe] #{n} kind={kind} entry={entry_id} id={id:#x} name={name_hash:#x} \
             arr[{cnt}]={k0},{k1},{k2}"
        );
    }
    if let Some(base) = clone_base(kind) {
        let spoof_hash = base_name_hash(base).unwrap_or(name_hash);
        let guard = if (0..8).contains(&entry_id) {
            let guard = &INIT_BRIDGE_GUARDS[entry_id as usize];
            while guard
                .compare_exchange_weak(
                    false,
                    true,
                    core::sync::atomic::Ordering::Acquire,
                    core::sync::atomic::Ordering::Relaxed,
                )
                .is_err()
            {
                core::hint::spin_loop();
            }
            Some(guard)
        } else {
            None
        };
        let patched = entry_kind_slot(entry_id, kind);
        if let Some((slot, idx, count)) = patched {
            dbg_log!(
                "[initbridge] #{n} kind {kind}->{base} name {name_hash:#x}->{spoof_hash:#x} \
                 arr entry={entry_id} count={count} idx={idx}"
            );
            *slot = base;
        } else {
            dbg_log!(
                "[initbridge] #{n} WARNING kind {kind} absent from entry={entry_id}; \
                 forwarding without array bridge"
            );
        }

        with_construction_context(kind, || {
            call_original!(object, id, base, entry_id, spoof_hash)
        });
        if let Some((slot, _, _)) = patched {
            *slot = kind;
        }
        if let Some(guard) = guard {
            guard.store(false, core::sync::atomic::Ordering::Release);
        }
        dbg_log!("[initbridge] #{n} EXIT restored entry={entry_id} kind={kind}");
    } else {
        call_original!(object, id, kind, entry_id, name_hash);
    }
}

#[cfg(feature = "true_kind")]
pub(crate) const OFF_ENTRY_BLOCK_INSTALL: usize = 0x653240;

#[cfg(feature = "true_kind")]
pub(crate) static ENTRY118_LOG_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "true_kind")]
#[skyline::hook(offset = OFF_ENTRY_BLOCK_INSTALL)]
pub(crate) unsafe fn entry_block_install_hook(obj: *mut u8, block: *mut u8, aux: *mut u8) -> u64 {
    let count = *((block as usize + 0x14) as *const i32);
    let kinds = (block as usize + 0x18) as *mut i32;
    let n = ENTRY118_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        let entry = *((obj as usize + 0xe0) as *const i32);
        dbg_log!(
            "[entry118] #{n} install obj={:#x} entry={entry} count={count} kinds=[{},{},{}]",
            obj as usize,
            *kinds,
            *kinds.add(1),
            *kinds.add(2)
        );
    }
    if count == 1 && *kinds == 1 {
        *kinds = FIRST_CUSTOM_KIND;
        dbg_log!(
            "[entry118] #{n} FORCED entry kind 1 (donkey) -> {FIRST_CUSTOM_KIND} (true new kind, clone base 0)"
        );
    }
    call_original!(obj, block, aux)
}

#[cfg(feature = "true_kind")]
pub(crate) const OFF_ENTRY_LIFECYCLE: usize = 0x653490;

#[cfg(feature = "true_kind")]
pub(crate) static ENTRY118_TICK_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "true_kind")]
#[skyline::hook(offset = OFF_ENTRY_LIFECYCLE)]
pub(crate) unsafe fn entry_lifecycle_hook(obj: *mut u8) -> u64 {
    let n = ENTRY118_TICK_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        let entry = *((obj as usize + 0xe0) as *const i32);
        let state = *((obj as usize + 0x5920) as *const u8);
        let a_count = *((obj as usize + 0x14) as *const i32);
        let a0 = *((obj as usize + 0x18) as *const i32);
        let s_count = *((obj as usize + 0x84) as *const i32);
        let s0 = *((obj as usize + 0x88) as *const i32);
        dbg_log!(
            "[entry118] tick #{n} obj={:#x} entry={entry} state={state} active=({a_count},{a0}) staged=({s_count},{s0})",
            obj as usize
        );
    }
    let s_count = *((obj as usize + 0x84) as *const i32);
    let s_kinds = (obj as usize + 0x88) as *mut i32;
    if s_count == 1 && *s_kinds == 1 {
        *s_kinds = FIRST_CUSTOM_KIND;
        let entry = *((obj as usize + 0xe0) as *const i32);
        dbg_log!(
            "[entry118] STAGED entry={entry} kind 1 (donkey) -> {FIRST_CUSTOM_KIND} obj={:#x}",
            obj as usize
        );
    }
    let a_count = *((obj as usize + 0x14) as *const i32);
    let a_kinds = (obj as usize + 0x18) as *mut i32;
    if a_count == 1 && *a_kinds == 1 {
        *a_kinds = FIRST_CUSTOM_KIND;
        let entry = *((obj as usize + 0xe0) as *const i32);
        dbg_log!(
            "[entry118] ACTIVE entry={entry} kind 1 (donkey) -> {FIRST_CUSTOM_KIND} obj={:#x}",
            obj as usize
        );
    }
    call_original!(obj)
}

#[cfg(feature = "true_kind")]
pub(crate) const OFF_KIND_EXPANDER: usize = 0x65dbc0;

#[cfg(feature = "true_kind")]
pub(crate) static ENTRY118_EXPAND_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "true_kind", not(feature = "css_slot")))]
pub(crate) const IN_MATCH_CALLER_MIN: usize = 0x100_0000;

#[cfg(feature = "css_slot")]
pub(crate) const IN_MATCH_RESOURCE_LOADER_CALLER: usize = 0x17e_56d8;

#[cfg(all(feature = "true_kind", not(feature = "css_slot")))]
pub(crate) const MATCH_ENTRY_CALLER_A: usize = 0x66_dd18;
#[cfg(all(feature = "true_kind", not(feature = "css_slot")))]
pub(crate) const MATCH_ENTRY_CALLER_B: usize = 0x66_db88;
#[cfg(feature = "true_kind")]
pub(crate) const MATCH_ROSTER_SETUP_CALLER: usize = 0x14e_c24c;

#[cfg(feature = "diag_load_barrier")]
pub(crate) const OFF_LOAD_BARRIER_POLL: usize = 0x14e_94d4;

#[cfg(feature = "diag_load_barrier")]
pub(crate) const OFF_MATCH_SETUP_ENTRY: usize = 0x14e_58f0;

macro_rules! setup_trace_probe {
    ($name:ident, $ord:expr, $off:expr, $target:expr) => {
        #[cfg(feature = "diag_load_barrier")]
        #[skyline::hook(offset = $off, inline)]
        pub(crate) unsafe fn $name(_ctx: &mut skyline::hooks::InlineCtx) {
            use core::sync::atomic::{AtomicU32, Ordering};
            static HITS: AtomicU32 = AtomicU32::new(0);
            let n = HITS.fetch_add(1, Ordering::Relaxed);
            if n < 3 {
                dbg_log!(
                    "[setuptrace] step {} @{:#x} -> {} (hit {n})",
                    $ord,
                    $off as usize,
                    $target
                );
            }
        }
    };
}

setup_trace_probe!(setup_trace_1, 1, 0x14e_5a40, "0x1754d80");
setup_trace_probe!(setup_trace_2, 2, 0x14e_642c, "0x353d480");
setup_trace_probe!(setup_trace_3, 3, 0x14e_7434, "0x510ff0");
setup_trace_probe!(setup_trace_4, 4, 0x14e_84bc, "0x66ee40");
setup_trace_probe!(
    setup_trace_5,
    5,
    0x14e_8f68,
    "0x138a5a0 (per-slot loop, 1st of 8)"
);
setup_trace_probe!(
    setup_trace_6,
    6,
    0x14e_93e0,
    "0x3255580 (last before 0x14e94d4)"
);

#[cfg(feature = "diag_load_barrier")]
#[skyline::hook(offset = OFF_MATCH_SETUP_ENTRY, inline)]
pub(crate) unsafe fn match_setup_entry_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::{AtomicU32, Ordering};
    static CALLS: AtomicU32 = AtomicU32::new(0);
    let n = CALLS.fetch_add(1, Ordering::Relaxed);
    if n >= 24 {
        return;
    }
    let caller = (ctx.registers[30].x() as usize).wrapping_sub(text_base());
    let mut custom = [-1i32; 8];
    for (entry, out) in custom.iter_mut().enumerate() {
        if let Some(kind) = crate::css_registration::entry_custom_kind(entry as u8) {
            *out = kind;
        }
    }
    dbg_log!("[setupentry] #{n} ENTER caller=@{caller:#x} custom={custom:?}");
}

#[cfg(feature = "diag_load_barrier")]
#[skyline::hook(offset = OFF_LOAD_BARRIER_POLL, inline)]
pub(crate) unsafe fn load_barrier_poll_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    const SLOTS: usize = 8;
    const FIRST: usize = 0xa;
    const STRIDE: usize = 0x260;
    const MAX_LINES: u32 = 96;
    const HEARTBEAT: u64 = 200_000;

    static ITER: AtomicU64 = AtomicU64::new(0);
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    static LINES: AtomicU32 = AtomicU32::new(0);

    let base = ctx.registers[12].x() as usize;
    if base == 0 {
        return;
    }
    let mut slots = [0u8; SLOTS];
    for (index, slot) in slots.iter_mut().enumerate() {
        *slot = *((base + FIRST + index * STRIDE) as *const u8);
    }
    let packed = u64::from_le_bytes(slots);
    let n = ITER.fetch_add(1, Ordering::Relaxed);
    let changed = LAST.swap(packed, Ordering::Relaxed) != packed;
    if !(changed || n < 4 || n % HEARTBEAT == 0) {
        return;
    }
    if LINES.fetch_add(1, Ordering::Relaxed) >= MAX_LINES {
        return;
    }
    let mut custom = [-1i32; SLOTS];
    for (entry, out) in custom.iter_mut().enumerate() {
        if let Some(kind) = crate::css_registration::entry_custom_kind(entry as u8) {
            *out = kind;
        }
    }
    dbg_log!("[loadbar] #{n} slots={slots:?} custom={custom:?} base={base:#x} changed={changed}");
}

#[cfg(feature = "true_kind")]
#[skyline::hook(offset = OFF_KIND_EXPANDER)]
pub(crate) unsafe fn kind_expander_hook(kind: i32, out: *mut i32) -> i32 {
    close_registration("match roster expansion");
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller_off = lr.wrapping_sub(text_base());

    let n = ENTRY118_EXPAND_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let mut k = kind;

    #[cfg(feature = "css_slot")]
    if let Some(definition) = clone_definition(k) {
        if caller_off == MATCH_ROSTER_SETUP_CALLER {
            dbg_log!(
                "[entrykind] expand #{n}: kind {} retained (CONSTRUCTION roster caller @{caller_off:#x})",
                definition.kind
            );
        } else if caller_off == IN_MATCH_RESOURCE_LOADER_CALLER {
            k = definition.base_kind;
            dbg_log!(
                "[entrykind] expand #{n}: kind {} -> {} ({}, RESOURCE-LOADER caller @{caller_off:#x})",
                definition.kind,
                definition.base_kind,
                definition.base_resource_name
            );
        } else {
            dbg_log!(
                "[entrykind] expand #{n}: kind {} retained (CSS/match-entry caller @{caller_off:#x})",
                definition.kind
            );
        }
    } else if n < 40 {
        dbg_log!("[entry118] expand #{n} kind={kind} passthrough (caller @{caller_off:#x})");
    }

    #[cfg(not(feature = "css_slot"))]
    if k == 1 {
        if caller_off == MATCH_ENTRY_CALLER_A || caller_off == MATCH_ENTRY_CALLER_B {
            k = FIRST_CUSTOM_KIND;
            dbg_log!(
                "[entry118] expand #{n}: kind 1 -> {FIRST_CUSTOM_KIND} (match-entry caller @{caller_off:#x})"
            );
        } else if caller_off == MATCH_ROSTER_SETUP_CALLER {
            k = FIRST_CUSTOM_KIND;
            dbg_log!(
                "[entry118] expand #{n}: kind 1 -> {FIRST_CUSTOM_KIND} (CONSTRUCTION roster caller @{caller_off:#x})"
            );
        } else if caller_off >= IN_MATCH_CALLER_MIN {
            k = 0;
            dbg_log!(
                "[entry118] expand #{n}: kind 1 -> 0 (mario, IN-MATCH caller @{caller_off:#x})"
            );
        } else {
            k = 0;
            dbg_log!("[entry118] expand #{n}: kind 1 -> 0 (CSS enum caller @{caller_off:#x})");
        }
    } else if n < 40 {
        dbg_log!("[entry118] expand #{n} kind={kind} passthrough (caller @{caller_off:#x})");
    }
    let count = call_original!(k, out);
    if n < 24 {
        dbg_log!(
            "[entry118] expand #{n} kind={kind}->{k} count={count} out=[{},{},{}]",
            *out,
            *out.add(1),
            *out.add(2)
        );
    }
    count
}

#[cfg(all(feature = "diag_article_initspoof", feature = "true_kind"))]
pub(crate) unsafe fn log_entry_installer_vt(n: u32, entry_id: i32) {
    let tb = text_base();
    let t1 = *((tb + 0x52b84f8) as *const usize);
    if t1 == 0 {
        return;
    }
    let t2 = *(t1 as *const usize);
    if t2 == 0 {
        return;
    }
    let per = *((t2 + entry_id as usize * 8 + 0x20) as *const usize);
    if per == 0 {
        return;
    }
    let sub = *((per + 0xf0) as *const usize);
    if sub == 0 {
        dbg_log!("[entry118] vt #{n} entry={entry_id} per={per:#x} sub(+0xf0)=NULL");
        return;
    }
    let vt = *(sub as *const usize);
    if vt == 0 {
        dbg_log!("[entry118] vt #{n} entry={entry_id} sub={sub:#x} vt=NULL");
        return;
    }
    let f18 = *((vt + 0x18) as *const usize);
    let f28 = *((vt + 0x28) as *const usize);
    dbg_log!(
        "[entry118] vt #{n} entry={entry_id} sub={sub:#x} vt=text+{:#x} \
         install(vt+0x28)=text+{:#x} post(vt+0x18)=text+{:#x}",
        vt.wrapping_sub(tb),
        f28.wrapping_sub(tb),
        f18.wrapping_sub(tb)
    );
}

#[cfg(feature = "clone_runtime")]
pub(crate) const OFF_KIND_VALIDITY_GATE: usize = 0x65dd70;

#[cfg(feature = "clone_runtime")]
pub(crate) static GATE118_118_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "clone_runtime")]
pub(crate) static GATE118_OTHER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "clone_runtime")]
#[skyline::hook(offset = OFF_KIND_VALIDITY_GATE)]
pub(crate) unsafe extern "C" fn kind_validity_gate_hook(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let kind = a0 as u32;
    if let Some(base) = clone_base(kind as i32) {
        let n = GATE118_118_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            let real_ret = call_original!(a0, a1, a2, a3, a4, a5, a6, a7);
            dbg_log!("[gatekind] #{n} kind={kind} REAL ret={real_ret:#x}");
            let spoof_ret = call_original!(base as u64, a1, a2, a3, a4, a5, a6, a7);
            dbg_log!("[gatekind] #{n} kind={kind} SPOOF kind->{base} ret={spoof_ret:#x}");
            return spoof_ret;
        }
        return call_original!(base as u64, a1, a2, a3, a4, a5, a6, a7);
    }
    if kind > 0x75 {
        let n = GATE118_OTHER_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 32 {
            let ret = call_original!(a0, a1, a2, a3, a4, a5, a6, a7);
            dbg_log!("[gate118] #{n} kind={kind} ret={ret:#x}");
            return ret;
        }
        return call_original!(a0, a1, a2, a3, a4, a5, a6, a7);
    }
    call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
}

#[cfg(any(feature = "diag_pathtrace", feature = "clone_runtime"))]
pub(crate) const OFF_PATH_BUILDER: usize = 0x17df460;

#[cfg(feature = "diag_pathtrace")]
pub(crate) static PATHB_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "diag_pathtrace")]
pub(crate) static PATHB_LOW_LOG_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(any(feature = "diag_pathtrace", feature = "css_slot"))]
pub(crate) unsafe fn read_cstr_capped(ptr: *const u8, cap: usize) -> String {
    if ptr.is_null() {
        return String::from("<null>");
    }
    let mut buf: Vec<u8> = Vec::with_capacity(cap);
    for i in 0..cap {
        let b = *ptr.add(i);
        if b == 0 {
            break;
        }
        buf.push(b);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

#[cfg(all(feature = "diag_pathtrace", not(feature = "true_kind")))]
#[skyline::hook(offset = OFF_PATH_BUILDER)]
pub(crate) unsafe extern "C" fn path_builder_trace_hook(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let ret = call_original!(a0, a1, a2, a3, a4, a5, a6, a7);
    let kind = a1 as u32;
    let n = PATHB_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let should_log = if kind > 0x75 {
        true
    } else {
        PATHB_LOW_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 24
    };
    if should_log {
        let s = read_cstr_capped(a0 as *const u8, 96);
        dbg_log!("[pathb] #{n} kind={kind} type={} out={s}", a2 as u32);
    }
    ret
}

#[cfg(feature = "clone_runtime")]
pub(crate) static PATHB_HI: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "clone_runtime")]
#[skyline::hook(offset = OFF_PATH_BUILDER)]
pub(crate) unsafe extern "C" fn path_builder_remap_hook(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    if let Some(definition) = clone_definition(a1 as i32) {
        let n = PATHB_HI.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        #[cfg(feature = "css_slot")]
        {
            let result = with_resource_context(definition.kind, || {
                call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
            });
            if n < 48 {
                let path = read_cstr_capped(a0 as *const u8, 96);
                dbg_log!(
                    "[pathb] #{n} CUSTOM kind={} resource={} type={} out={path}",
                    definition.kind,
                    definition.resource_name,
                    a2 as u32
                );
            }
            return result;
        }

        #[cfg(not(feature = "css_slot"))]
        {
            if n < 48 {
                dbg_log!(
                "[pathb] #{n} REMAP kind=118->0 (Fork A', mario path; [obj+0x58] untouched) a0={a0:#x} type={}",
                a2 as u32
            );
            }
            return call_original!(a0, 0u64, a2, a3, a4, a5, a6, a7);
        }
    }
    call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
}

#[cfg(feature = "true_kind")]
pub(crate) const OFF_RESMGR_INSERT: usize = 0x17d1d70;

#[cfg(feature = "true_kind")]
pub(crate) static RESMGR_INS_HI_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "true_kind")]
pub(crate) static RESMGR_INS_LO_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "true_kind")]
pub(crate) unsafe fn resmgr_active_byte(this: u64, kind: u32) -> i32 {
    if this == 0 {
        return -1;
    }
    let base = *(this as *const u64);
    if base == 0 {
        return -2;
    }
    let slot = base.wrapping_add(kind as u64 * 0xc88).wrapping_add(0xd00);
    *(slot as *const u8) as i32
}

#[cfg(feature = "true_kind")]
#[skyline::hook(offset = OFF_RESMGR_INSERT)]
pub(crate) unsafe extern "C" fn resmgr_insert_hook(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller_off = lr.wrapping_sub(text_base());
    let kind = a1 as u32;
    let ret = call_original!(a0, a1, a2, a3, a4, a5, a6, a7);

    #[cfg(all(feature = "native_relocate", not(feature = "native_patch_probe")))]
    resource_relocation::on_resource_manager_insert();

    #[cfg(feature = "stage_relocate")]
    crate::stage_transaction::try_relocate();

    crate::stage_db_rows::apply_pending();

    #[cfg(feature = "stage_mint_places")]
    crate::stage_transaction::verify_canaries();
    #[cfg(feature = "stage_mint")]
    crate::stage_registry::install_pending();

    let log_probe = |n: u32, label: &str| {
        let act = resmgr_active_byte(a0, kind);
        let (count, first_key, first_value) = if a2 != 0 {
            let count = *((a2 as usize + 0x10) as *const u64);
            let root = *((a2 as usize + 0x08) as *const u64);
            if count != 0 && root != 0 && root != a2 {
                (
                    count,
                    *((root as usize + 0x20) as *const u64),
                    *((root as usize + 0x28) as *const u32),
                )
            } else {
                (count, 0, 0)
            }
        } else {
            (u64::MAX, 0, 0)
        };
        dbg_log!(
            "[resmgr49] #{n} {label} kind={kind} caller=@{caller_off:#x} ret={ret} act={act} out_count={count} first={first_key:#x}/{first_value:#x}"
        );
    };

    if kind >= 0x76 && kind <= 0x77 {
        let n = RESMGR_INS_HI_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 64 {
            log_probe(n, "custom");
        }
    } else if kind == 0 {
        let n = RESMGR_INS_LO_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            log_probe(n, "mario");
        }
    }
    ret
}

#[cfg(feature = "true_kind")]
pub(crate) const OFF_RESMGR_ACTIVATE_BATCH_CALL: usize = 0x184a360;
#[cfg(feature = "true_kind")]
pub(crate) const OFF_RESMGR_ACTIVATE_DYNAMIC_CALL: usize = 0x197489c;

#[cfg(feature = "true_kind")]
#[skyline::from_offset(0x17d1f40)]
pub(crate) unsafe fn resmgr_activate_native(this: u64, kind: u32, source_tree: u64);

#[cfg(feature = "true_kind")]
pub(crate) static RESMGR_MIRROR_BATCH_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "true_kind")]
pub(crate) static RESMGR_MIRROR_DYNAMIC_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "true_kind")]
pub(crate) unsafe fn resmgr_activate_and_mirror(
    ctx: &skyline::hooks::InlineCtx,
    site: &'static str,
    counter: &core::sync::atomic::AtomicU32,
) {
    let this = ctx.registers[0].x();
    let kind = ctx.registers[1].x() as u32;
    let source_tree = ctx.registers[2].x();

    if kind != 0 {
        return;
    }

    let before = resmgr_active_byte(this, FIRST_CUSTOM_KIND as u32);
    resmgr_activate_native(this, FIRST_CUSTOM_KIND as u32, source_tree);
    let after = resmgr_active_byte(this, FIRST_CUSTOM_KIND as u32);

    let n = counter.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        let mario = resmgr_active_byte(this, 0);
        dbg_log!(
            "[resmirror] {site} #{n} manager={this:#x} tree={source_tree:#x} \
             kind0_act={mario} kind118_act={before}->{after}"
        );
    }
}

#[cfg(feature = "true_kind")]
#[allow(dead_code)]
#[skyline::hook(offset = OFF_RESMGR_ACTIVATE_BATCH_CALL, inline)]
pub(crate) unsafe fn resmgr_activate_batch_call_hook(ctx: &skyline::hooks::InlineCtx) {
    resmgr_activate_and_mirror(ctx, "batch@184a360", &RESMGR_MIRROR_BATCH_COUNT);
}

#[cfg(feature = "true_kind")]
#[allow(dead_code)]
#[skyline::hook(offset = OFF_RESMGR_ACTIVATE_DYNAMIC_CALL, inline)]
pub(crate) unsafe fn resmgr_activate_dynamic_call_hook(ctx: &skyline::hooks::InlineCtx) {
    resmgr_activate_and_mirror(ctx, "dynamic@197489c", &RESMGR_MIRROR_DYNAMIC_COUNT);
}

pub(crate) const OFF_LOAD_DISPATCH: usize = 0x17e5c00;

#[cfg(feature = "clone_runtime")]
pub(crate) const OFF_LOAD_FINAL_REGISTER_CALL: usize = 0x17e7480;

#[cfg(feature = "clone_runtime")]
pub(crate) static LOAD_FINAL_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static CUSTOM_LOAD_OBJECT: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "clone_runtime")]
#[skyline::hook(offset = OFF_LOAD_FINAL_REGISTER_CALL, inline)]
pub(crate) unsafe fn load_final_register_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let load_obj = ctx.registers[0].x();
    let kind = ctx.registers[1].x() as u32;

    if clone_definition(kind as i32).is_some() {
        let n = LOAD_FINAL_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        #[cfg(feature = "css_slot")]
        {
            CUSTOM_LOAD_OBJECT.store(load_obj, core::sync::atomic::Ordering::Release);
            if n < 32 {
                dbg_log!(
                    "[loadfinal] #{n} CUSTOM single register keeps w1={kind} obj={load_obj:#x}; trampoline executes once"
                );
            }
        }

        #[cfg(not(feature = "css_slot"))]
        {
            ctx.registers[1].set_x(0);
            if n < 32 {
                dbg_log!(
                "[loadfinal] #{n} single register w1 118->0 obj={load_obj:#x}; trampoline executes once"
            );
            }
        }
    }
}

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) const OFF_GROUP_READY: usize = 0x17eb180;
#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) const OFF_ENTRY_READY: usize = 0x17e4790;
#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) const OFF_BASE_READY: usize = 0x17e28c0;

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) static GROUP_READY_118_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) static ENTRY_READY_118_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) static BASE_READY_118_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) const OFF_NATIVE_KIND_REGISTER: usize = 0x17e4940;
#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) static NATIVE_REGISTER_118_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
pub(crate) unsafe fn registered_kind_hash(object: u64, kind: u32) -> u64 {
    if object == 0 {
        return 0;
    }
    let key = kind as u64 * 42;
    let sentinel = object + 0x4b8;
    let mut node = *((object + 0x4b8) as *const u64);
    let mut candidate = sentinel;
    while node != 0 {
        let node_key = *((node + 0x20) as *const u64);
        if node_key < key {
            node = *((node + 0x8) as *const u64);
        } else {
            candidate = node;
            node = *(node as *const u64);
        }
    }
    if candidate != sentinel && *((candidate + 0x20) as *const u64) == key {
        *((candidate + 0x28) as *const u64) & 0xffffffffff
    } else {
        0
    }
}

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
#[skyline::hook(offset = OFF_NATIVE_KIND_REGISTER)]
pub(crate) unsafe fn custom_native_register_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let kind = x1 as i32;
    let Some(definition) = clone_definition(kind) else {
        return call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    };

    let n = NATIVE_REGISTER_118_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let hash_before = registered_kind_hash(x0, kind as u32);
    let ret = call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    let hash_after = registered_kind_hash(x0, kind as u32);
    if n < 32 {
        dbg_log!(
            "[regkind] #{n} MODULE obj={x0:#x} key={kind} hash={hash_before:#x}->{hash_after:#x} expected_base={}({:#x}); asset paths stay {}",
            definition.base_resource_name,
            hash40(definition.base_resource_name),
            definition.resource_name
        );
    }
    ret
}

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
#[skyline::hook(offset = OFF_BASE_READY)]
pub(crate) unsafe fn custom_base_ready_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let custom = CUSTOM_LOAD_OBJECT.load(core::sync::atomic::Ordering::Acquire);
    if custom == 0 || x0 != custom {
        return call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    }

    let n = BASE_READY_118_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let root = *((x0 + 0x8) as *const u64);
    let sentinel = x0 + 0x10;
    let count = *((x0 + 0x18) as *const u64);
    let cached = *((x0 + 0x20) as *const u8);
    let (key, node_type, node_index) = if root != 0 && root != sentinel {
        (
            *((root + 0x20) as *const u64),
            *((root + 0x2c) as *const u32),
            *((root + 0x30) as *const u32),
        )
    } else {
        (0, u32::MAX, u32::MAX)
    };

    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
    let fs = *((text + 0x5331f20) as *const u64);
    let mut fs_count = 0u32;
    let mut fs_valid = 0u8;
    let mut fs_state = 0xffu8;
    let mut fs_aux = 0u64;
    if fs != 0 {
        if node_type == 0x00ff_ffff && node_index != 0x00ff_ffff {
            fs_count = *((fs + 0x48) as *const u32);
            let slots = *((fs + 0x40) as *const u64);
            if slots != 0 && node_index < fs_count {
                let slot = slots + node_index as u64 * 0x48;
                fs_valid = *((slot + 0x8) as *const u8);
                fs_state = *(slot as *const u8);
                fs_aux = slot;
            }
        } else if node_type != 0x00ff_ffff {
            fs_count = *((fs + 0x18) as *const u32);
            let index_map = *((fs + 0x8) as *const u64);
            if index_map != 0 && node_type < fs_count {
                let map_entry = index_map + node_type as u64 * 8;
                let mapped = *(map_entry as *const u32);
                fs_valid = *((map_entry + 4) as *const u8);
                let record_count = *((fs + 0x1c) as *const u32);
                let records = *((fs + 0x10) as *const u64);
                if fs_valid != 0 && records != 0 && mapped < record_count {
                    let record = records + mapped as u64 * 0x18;
                    fs_state = *((record + 0xd) as *const u8);
                    fs_aux = record;
                }
            }
        }
    }

    let ret = call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    if n < 12 {
        dbg_log!(
            "[ready118] base #{n} ret={ret} obj={x0:#x} cached={cached} nodes={count} root={root:#x} key={key:#x} type={node_type:#x} index={node_index:#x} fs_count={fs_count} fs_valid={fs_valid:#x} fs_state={fs_state:#x} fs_aux={fs_aux:#x}"
        );
    }
    ret
}

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
#[skyline::hook(offset = OFF_ENTRY_READY)]
pub(crate) unsafe fn custom_entry_ready_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let custom = CUSTOM_LOAD_OBJECT.load(core::sync::atomic::Ordering::Acquire);
    if custom == 0 || x0 != custom {
        return call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    }

    let n = ENTRY_READY_118_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let pending = *((x0 + 0xa8) as *const u64);
    let pending_count = if pending != 0 {
        *((pending + 0x10) as *const u32)
    } else {
        0
    };
    let record_head = *((x0 + 0x4b0) as *const u64);
    let record_sentinel = x0 + 0x4b8;
    let record_count = *((x0 + 0x4c0) as *const u64);
    let first_hash = if record_head != 0 && record_head != record_sentinel {
        *((record_head + 0x28) as *const u64) & 0xffffffffff
    } else {
        0
    };
    let base_before = BASE_READY_118_COUNT.load(core::sync::atomic::Ordering::Relaxed);
    let ret = call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    let base_after = BASE_READY_118_COUNT.load(core::sync::atomic::Ordering::Relaxed);
    if n < 12 {
        let layer = if pending_count != 0 {
            "pending"
        } else if base_after == base_before {
            "record"
        } else {
            "base"
        };
        dbg_log!(
            "[ready118] entry #{n} ret={ret} layer={layer} obj={x0:#x} pending={pending:#x}/{pending_count} records={record_count} first_hash={first_hash:#x} base_calls={base_before}->{base_after}"
        );
    }
    ret
}

#[cfg(all(feature = "true_kind", feature = "css_slot"))]
#[skyline::hook(offset = OFF_GROUP_READY)]
pub(crate) unsafe fn custom_group_ready_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let ret = call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    let custom = CUSTOM_LOAD_OBJECT.load(core::sync::atomic::Ordering::Acquire);
    if custom != 0 && ret == 0 {
        let n = GROUP_READY_118_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            let mut slot_found = -1i32;
            for slot in 0..25u64 {
                if *((x0 + slot * 8) as *const u64) == custom {
                    slot_found = slot as i32;
                    break;
                }
            }
            dbg_log!(
                "[ready118] group #{n} ret=0 group={x0:#x} custom={custom:#x} slot={slot_found} entry_calls={} base_calls={}",
                ENTRY_READY_118_COUNT.load(core::sync::atomic::Ordering::Relaxed),
                BASE_READY_118_COUNT.load(core::sync::atomic::Ordering::Relaxed)
            );
        }
    }
    ret
}

#[cfg(feature = "clone_runtime")]
pub(crate) static LOAD_DISP_HI: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "clone_runtime")]
pub(crate) static LOAD_DISP_LO: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "clone_runtime")]
#[skyline::hook(offset = OFF_LOAD_DISPATCH)]
pub(crate) unsafe extern "C" fn load_dispatch_kind_hook(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller_off = lr.wrapping_sub(text_base());

    let kind = if a0 != 0 {
        *((a0 as usize + 0x58) as *const u32)
    } else {
        0xffff_ffff
    };

    if let Some(definition) = clone_definition(kind as i32) {
        let n = LOAD_DISP_HI.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 48 {
            dbg_log!(
                "[loaddisp] #{n} kind={} resource={} PASSTHRU obj={a0:#x} lr=@{caller_off:#x}",
                definition.kind,
                definition.resource_name
            );
        }
        #[cfg(feature = "css_slot")]
        return with_resource_context(definition.kind, || {
            call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
        });
        #[cfg(not(feature = "css_slot"))]
        return call_original!(a0, a1, a2, a3, a4, a5, a6, a7);
    }

    if kind != 0xffff_ffff && kind >= 0x5c {
        let n = LOAD_DISP_HI.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 48 {
            dbg_log!("[loaddisp] #{n} kind={kind} (unremapped) obj={a0:#x} lr=@{caller_off:#x}");
        }
    } else if kind == 0 {
        let n = LOAD_DISP_LO.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 6 {
            dbg_log!("[loaddisp] #{n} kind=0(baseline) obj={a0:#x} lr=@{caller_off:#x}");
        }
    }
    call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static CAMERA_ROUTE_DISABLED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = 0x36de390)]
pub(crate) unsafe fn camera_record_value_guard(object: *mut u8) -> f32 {
    if !object.is_null() {
        let record = core::ptr::read_volatile(object.add(0x38) as *const u64);
        let read = |a: u64| -> u64 {
            if a == 0 {
                0
            } else {
                core::ptr::read_volatile(a as *const u64)
            }
        };
        let blob = if record == 0 { 0 } else { read(record + 0x20) };
        let src = if record == 0 { 0 } else { read(record + 8) };

        if record != 0 && blob == 0 {
            if !CAMERA_ROUTE_DISABLED.swap(true, core::sync::atomic::Ordering::Relaxed) {
                dbg_log!(
                    "[camaccess] clone camera has no curves; type 39 routing DISABLED for the                      rest of the session - later cameras fall back to the base fighter's, which                      is the configuration that has always worked"
                );
            }
            return 0.0;
        }
    }
    call_original!(object)
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static VICTORY_CAMERA_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = 0x60d6ac, inline)]
pub(crate) unsafe fn victory_camera_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let record = ctx.registers[20].x() as usize;
    let kind = ctx.registers[1].x() as i32;

    let Some(definition) = entry_for_resource_record(record)
        .and_then(|entry| entry_custom_kind(entry as u8))
        .and_then(clone_definition)
        .filter(|definition| kind == definition.base_kind)
    else {
        return;
    };

    ctx.registers[1].set_x(definition.kind as u64);
    let n = VICTORY_CAMERA_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 16 {
        dbg_log!(
            "[vcamera] #{n} type=36 kind={kind}->{} namespace={} (fighter/{}/cNN/camera)",
            definition.kind,
            definition.resource_name,
            definition.resource_name
        );
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
pub(crate) static VICTORY_NAME_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
unsafe fn victory_camera_name(ctx: &mut skyline::hooks::InlineCtx, site: &str) {
    let entry = ctx.registers[8].x() as i64;
    let kind = ctx.registers[9].x() as i32;
    if !(0..8).contains(&entry) {
        return;
    }
    let Some(definition) = entry_custom_kind(entry as u8)
        .and_then(clone_definition)
        .filter(|definition| kind == definition.base_kind)
    else {
        return;
    };
    ctx.registers[2].set_x(definition.resource_name_cstr.as_ptr() as u64);
    let n = VICTORY_NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 12 {
        dbg_log!(
            "[vcamname] #{n} {site} entry={entry} kind={kind}->{} name={} \
             (camera/fighter/{}/cNN/J02WinN.nuanmb)",
            definition.kind,
            definition.resource_name,
            definition.resource_name
        );
    }
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = 0x1492968, inline)]
pub(crate) unsafe fn victory_camera_name_hook_a(ctx: &mut skyline::hooks::InlineCtx) {
    victory_camera_name(ctx, "0x1492968");
}

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
#[skyline::hook(offset = 0x14977ec, inline)]
pub(crate) unsafe fn victory_camera_name_hook_b(ctx: &mut skyline::hooks::InlineCtx) {
    victory_camera_name(ctx, "0x14977ec");
}

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
pub(crate) static TRAIL_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
#[skyline::hook(offset = 0x356092c, inline)]
pub(crate) unsafe fn trail_nutexb_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let data = ctx.registers[26].x() as *const u8;
    let size = ctx.registers[2].x() as usize;
    if data.is_null() || size < 0x70 {
        return;
    }
    let footer = core::slice::from_raw_parts(data.add(size - 0x70), 0x70);
    let Some(start) = footer.windows(4).position(|w| w == b" XNT") else {
        return;
    };
    let name = &footer[start + 4..];
    let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
    let Ok(name) = core::str::from_utf8(&name[..end]) else {
        return;
    };
    let n = TRAIL_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n >= 96 {
        return;
    }
    let owner = crate::active_resource_kind_for_diagnostics();
    record_trail_hash(name);
    dbg_log!("[trail] #{n} name={name} size={size:#x} context={owner:?}");
}

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
pub(crate) static TRAIL_DIR_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
#[skyline::hook(offset = 0x355fc30, inline)]
pub(crate) unsafe fn trail_directory_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let built = ctx.registers[8].x();
    let base = ctx.registers[19].x();
    let n = TRAIL_DIR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        let owner = crate::active_resource_kind_for_diagnostics();
        dbg_log!("[traildir] #{n} trail dir hash={built:#x} base={base:#x} context={owner:?}");
    }
}

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
static TRAIL_NAME_HASHES: [core::sync::atomic::AtomicU64; 64] =
    [const { core::sync::atomic::AtomicU64::new(0) }; 64];

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
fn record_trail_hash(name: &str) {
    for candidate in [
        crate::hash40::hash40(&name.to_lowercase()),
        crate::hash40::hash40(name),
    ] {
        for slot in TRAIL_NAME_HASHES.iter() {
            let seen = slot.load(core::sync::atomic::Ordering::Relaxed);
            if seen == candidate {
                break;
            }
            if seen == 0
                && slot
                    .compare_exchange(
                        0,
                        candidate,
                        core::sync::atomic::Ordering::AcqRel,
                        core::sync::atomic::Ordering::Relaxed,
                    )
                    .is_ok()
            {
                break;
            }
        }
    }
}

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
fn trail_hash_is_known(hash: u64) -> bool {
    hash != 0
        && TRAIL_NAME_HASHES
            .iter()
            .any(|slot| slot.load(core::sync::atomic::Ordering::Relaxed) == hash)
}

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
pub(crate) static TRAIL_REQUEST_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
pub(crate) static TRAIL_REQUEST_SEEN: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
#[skyline::hook(offset = 0x355b308, inline)]
pub(crate) unsafe fn trail_request_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let hash = ctx.registers[1].x();
    let seen = TRAIL_REQUEST_SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if seen < 12 {
        dbg_log!("[trailreq] control #{seen} lookup hash={hash:#x}");
    } else if seen == 4096 {
        dbg_log!("[trailreq] control: 4096 lookups seen, still no trail texture");
    }
    if !trail_hash_is_known(hash) {
        return;
    }
    let n = TRAIL_REQUEST_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!("[trailreq] #{n} effect lookup asked for a TRAIL texture hash={hash:#x}");
    }
}
