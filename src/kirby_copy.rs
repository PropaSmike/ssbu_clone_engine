use super::*;

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_HEADER_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_MODEL_SITE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_HAT_FLOW_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_CONTEXT: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("kirby_copy_context");

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_REMOVAL_CONTEXT_THREAD: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_REMOVAL_CONTEXT_KIND: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);

#[cfg(feature = "css_slot")]
pub(crate) fn active_kirby_copy_kind() -> Option<i32> {
    let kind = KIRBY_COPY_CONTEXT.active(unsafe { current_thread_key() })?;
    clone_definition(kind).map(|_| kind)
}

#[cfg(feature = "css_slot")]
pub(crate) fn active_kirby_removal_kind() -> Option<i32> {
    let thread = unsafe { current_thread_key() };
    if thread != 0
        && KIRBY_REMOVAL_CONTEXT_THREAD.load(core::sync::atomic::Ordering::Acquire) == thread
    {
        let kind = KIRBY_REMOVAL_CONTEXT_KIND.load(core::sync::atomic::Ordering::Acquire);
        return clone_definition(kind).map(|_| kind);
    }
    None
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn clear_kirby_removal_context() {
    let thread = current_thread_key();
    if thread != 0
        && KIRBY_REMOVAL_CONTEXT_THREAD.load(core::sync::atomic::Ordering::Acquire) == thread
    {
        KIRBY_REMOVAL_CONTEXT_THREAD.store(0, core::sync::atomic::Ordering::Release);
        KIRBY_REMOVAL_CONTEXT_KIND.store(-1, core::sync::atomic::Ordering::Release);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn with_kirby_copy_context<R>(kind: i32, callback: impl FnOnce() -> R) -> R {
    if clone_definition(kind).is_none() {
        return callback();
    }
    KIRBY_COPY_CONTEXT.scope(current_thread_key(), kind, callback)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn kirby_copy_work_snapshot(boma: u64) -> Option<(u64, i32, i32, Option<i32>)> {
    if boma == 0 {
        return None;
    }
    let work = *((boma as usize + 0x50) as *const u64);
    if work == 0 {
        return None;
    }
    let vt = *(work as *const u64);
    if vt == 0 {
        return None;
    }
    let is_flag: extern "C" fn(u64, u32) -> u64 =
        core::mem::transmute(*((vt as usize + 0x108) as *const u64));
    let get_int: extern "C" fn(u64, u32) -> u64 =
        core::mem::transmute(*((vt as usize + 0x98) as *const u64));
    let flag = is_flag(work, 0x2000_0102) & 1;
    let copy_kind = get_int(work, 0x1000_00FC) as u32 as i32;
    let target_entry = get_int(work, 0x1000_00FD) as u32 as i32;
    let target_kind = if (0..8).contains(&target_entry) {
        entry_custom_kind(target_entry as u8)
    } else {
        None
    };
    Some((flag, copy_kind, target_entry, target_kind))
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn kirby_hat_flow_log_index() -> Option<u32> {
    let n = KIRBY_HAT_FLOW_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    (n < 96).then_some(n)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_MODEL_CHANGER_ENTRY, inline)]
pub(crate) unsafe fn kirby_copy_model_changer_entry_probe(ctx: &mut skyline::hooks::InlineCtx) {
    clear_kirby_removal_context();
    let link = ctx.registers[0].x();
    let requested = ctx.registers[1].x() as u32 as i32;
    let active = active_kirby_copy_kind();
    let pending_visual = active.filter(|kind| {
        clone_definition(*kind)
            .map(|definition| requested == definition.base_kind)
            .unwrap_or(false)
    });
    let previous = if link == 0 {
        i32::MIN
    } else {
        *((link as usize + 0x17398) as *const i32)
    };
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=change-enter link={link:#x} requested={requested}->{requested} previous={previous} context={active:?} pending_visual={pending_visual:?}"
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_VISUAL_KIND_PROMOTE, inline)]
pub(crate) unsafe fn kirby_copy_visual_kind_promote(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(kind) = active_kirby_copy_kind() else {
        return;
    };
    let definition = clone_definition(kind).unwrap();
    if !definition.kirby_copy_full_model {
        return;
    }
    let before = ctx.registers[20].x() as u32 as i32;
    if before != definition.base_kind {
        return;
    }

    let link = ctx.registers[19].x();
    let row = if link == 0 {
        0
    } else {
        *((link as usize + 0x250) as *const u64)
    };
    let row_kind = if row == 0 {
        i32::MIN
    } else {
        *(row as *const i32)
    };
    if row == 0 || row_kind != definition.base_kind {
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!(
                "[kirbyhat] #{n} site=post-row-promote fallback kind={before} target={kind} link={link:#x} row={row:#x} row_kind={row_kind}"
            );
        }
        return;
    }

    ctx.registers[20].set_x(kind as u32 as u64);
    *((link as usize + 0x17398) as *mut i32) = kind;
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!(
            "[kirbyhat] #{n} site=post-row-promote kind={before}->{kind} link={link:#x} row={row:#x} row_kind={row_kind} previous_store={kind}"
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_FULL_MODEL_REMOVE_GATE, inline)]
pub(crate) unsafe fn kirby_copy_full_model_remove_gate(ctx: &mut skyline::hooks::InlineCtx) {
    if ctx.registers[20].x() as u32 as i32 != -1 {
        return;
    }
    let previous = ctx.registers[27].x() as u32 as i32;
    if !clone_definition(previous).is_some_and(|definition| definition.kirby_copy_full_model) {
        return;
    }

    let fighter = ctx.registers[24].x();
    let resource = if fighter == 0 {
        0
    } else {
        *((fighter as usize + 0xf2c8) as *const u64)
    };
    let pending = if resource == 0 {
        0
    } else {
        *((resource as usize + 0x160) as *const u64)
    };
    if pending == 0 {
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!(
                "[kirbyhat] #{n} site=teardown-gate fallback previous={previous} resource={resource:#x} pending=0"
            );
        }
        return;
    }

    let thread = current_thread_key();
    if thread == 0 {
        return;
    }
    KIRBY_REMOVAL_CONTEXT_KIND.store(previous, core::sync::atomic::Ordering::Relaxed);
    KIRBY_REMOVAL_CONTEXT_THREAD.store(thread, core::sync::atomic::Ordering::Release);
    ctx.registers[27].set_x(KIRBY_FULL_MODEL_BRANCH_KIND as u32 as u64);
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!(
            "[kirbyhat] #{n} site=teardown-gate previous={previous}->{} resource={resource:#x} pending={pending:#x}",
            KIRBY_FULL_MODEL_BRANCH_KIND
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_FULL_MODEL_GATE, inline)]
pub(crate) unsafe fn kirby_copy_full_model_gate(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(kind) = active_kirby_copy_kind() else {
        return;
    };
    if !clone_definition(kind).is_some_and(|definition| definition.kirby_copy_full_model) {
        return;
    }
    if ctx.registers[20].x() as u32 as i32 != kind {
        return;
    }

    let fighter = ctx.registers[24].x();
    let resource = if fighter == 0 {
        0
    } else {
        *((fighter as usize + 0xf2c8) as *const u64)
    };
    let pending = if resource == 0 {
        0
    } else {
        *((resource as usize + 0x160) as *const u64)
    };
    let source = if resource == 0 {
        0
    } else {
        *((resource as usize + 0x10) as *const u64)
    };
    if pending == 0 && source == 0 {
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!(
                "[kirbyhat] #{n} site=full-model-gate fallback kind={kind} fighter={fighter:#x} resource={resource:#x} pending=0 source=0"
            );
        }
        return;
    }

    ctx.registers[20].set_x(KIRBY_FULL_MODEL_BRANCH_KIND as u32 as u64);
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!(
            "[kirbyhat] #{n} site=full-model-gate kind={kind}->{} resource={resource:#x} pending={pending:#x} source={source:#x}",
            KIRBY_FULL_MODEL_BRANCH_KIND
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_MODEL_REMOVE_RESULT, inline)]
pub(crate) unsafe fn kirby_copy_model_remove_result_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=remove requested={} article={:#x} context={:?}",
        ctx.registers[20].x() as u32 as i32,
        ctx.registers[0].x(),
        active_kirby_copy_kind()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_SPECIAL_INSTALL, inline)]
pub(crate) unsafe fn kirby_copy_special_install_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let routed = ctx.registers[20].x() as u32 as i32;
    let active = active_kirby_copy_kind();
    let previous_routed = ctx.registers[27].x() as u32 as i32;
    let removal = active_kirby_removal_kind();
    let previous_restored = if routed == -1 && previous_routed == KIRBY_FULL_MODEL_BRANCH_KIND {
        if let Some(kind) = removal {
            ctx.registers[27].set_x(kind as u32 as u64);
            kind
        } else {
            previous_routed
        }
    } else {
        previous_routed
    };
    if removal.is_some() {
        clear_kirby_removal_context();
    }
    let restored = if routed == KIRBY_FULL_MODEL_BRANCH_KIND {
        if let Some(kind) = active.filter(|kind| {
            clone_definition(*kind).is_some_and(|definition| definition.kirby_copy_full_model)
        }) {
            ctx.registers[20].set_x(kind as u32 as u64);
            kind
        } else {
            routed
        }
    } else {
        routed
    };
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=special requested={routed}->{restored} previous={previous_routed}->{previous_restored} context={active:?} removal={removal:?}",
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_DIR_NAME_MERGE, inline)]
pub(crate) unsafe fn kirby_copy_dir_name_merge_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[20].x() as u32 as i32;
    if kind != 3 && kind < FIRST_CUSTOM_KIND {
        return;
    }
    let active = active_kirby_copy_kind();
    let mut selected = false;
    if active == Some(kind) {
        if let Some(definition) = clone_definition(kind) {
            ctx.registers[2].set_x(definition.resource_name_cstr.as_ptr() as u64);
            selected = true;
        }
    }
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=registrar-name kind={kind} color={} active={active:?} selected={selected} name_ptr={:#x}",
        ctx.registers[21].x() as u32,
        ctx.registers[2].x()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_RECORD_LOOKUP_KIND, inline)]
pub(crate) unsafe fn kirby_copy_record_lookup_kind(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(kind) = active_kirby_copy_kind() else {
        return;
    };
    let before = ctx.registers[26].x() as u32 as i32;
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    if before != definition.base_kind {
        return;
    }
    ctx.registers[26].set_x(kind as u32 as u64);
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!("[kirbyhat] #{n} site=record-lookup target_kind={kind} lookup_kind={before}->{kind}");
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn kirby_copy_resource_slot_probe(
    ctx: &mut skyline::hooks::InlineCtx,
    site: u32,
) {
    let Some(kind) = active_kirby_copy_kind() else {
        return;
    };
    let before = ctx.registers[2].x() as u32 as i32;
    let definition = clone_definition(kind).unwrap();
    if before == definition.base_kind {
        ctx.registers[2].set_x(kind as u32 as u64);
    }
    let after = ctx.registers[2].x() as u32 as i32;
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=resource-{site} target_kind={kind} lookup_kind={before}->{after} member_type={:#x} record={:#x}",
        ctx.registers[3].x() as u32,
        ctx.registers[1].x()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_RESOURCE_SLOT_0, inline)]
pub(crate) unsafe fn kirby_copy_resource_slot_0_probe(ctx: &mut skyline::hooks::InlineCtx) {
    kirby_copy_resource_slot_probe(ctx, 0);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_RESOURCE_SLOT_1, inline)]
pub(crate) unsafe fn kirby_copy_resource_slot_1_probe(ctx: &mut skyline::hooks::InlineCtx) {
    kirby_copy_resource_slot_probe(ctx, 1);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_RESOURCE_SLOT_2, inline)]
pub(crate) unsafe fn kirby_copy_resource_slot_2_probe(ctx: &mut skyline::hooks::InlineCtx) {
    kirby_copy_resource_slot_probe(ctx, 2);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_HAT_SYNC, inline)]
pub(crate) unsafe fn kirby_copy_hat_sync_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let fighter = ctx.registers[1].x();
    let boma = if fighter == 0 {
        0
    } else {
        *((fighter as usize + 0x20) as *const u64)
    };
    let Some((flag, copy_kind, target_entry, target_kind)) = kirby_copy_work_snapshot(boma) else {
        return;
    };
    if target_kind.is_none() {
        return;
    }
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=sync fighter={fighter:#x} boma={boma:#x} flag={flag} copy_kind={copy_kind} target_entry={target_entry} target_kind={target_kind:?} event={:#x}",
        ctx.registers[2].x()
    );
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn bridged_clone_kind(kind: i32) -> i32 {
    clone_definition(kind)
        .map(|definition| definition.base_kind)
        .unwrap_or(kind)
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn add_table_bias(address: u64, kind: i32) -> u64 {
    let base_kind = bridged_clone_kind(kind);
    let bias = i64::from(base_kind - kind) * 8;
    (address as i64).wrapping_add(bias) as u64
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_STATIC_SETUP_TABLE, inline)]
pub(crate) unsafe fn kirby_copy_static_setup_table(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[20].x() as i32;
    let before = ctx.registers[8].x();
    let after = add_table_bias(before, kind);
    ctx.registers[8].set_x(after);
    if clone_definition(kind).is_some() {
        let n = KIRBY_COPY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[kirbycopy_early] #{n} site=static kind={kind}->{} table={before:#x}->{after:#x}",
                bridged_clone_kind(kind)
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_ARTICLE_HEADER_ASSIGN, inline)]
pub(crate) unsafe fn kirby_copy_article_header_assign(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(target_kind) = active_kirby_copy_kind() else {
        return;
    };
    let Some((header, count)) = custom_articles::kirby_copy_header(target_kind) else {
        return;
    };
    let native_header = ctx.registers[23].x();
    ctx.registers[23].set_x(header as u64);
    let n = KIRBY_COPY_ARTICLE_HEADER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        dbg_log!(
            "[copyarticle] #{n} publish target={target_kind} native={native_header:#x} custom={header:#x} count={count}"
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn load_bridged_kirby_callback_kind(ctx: &mut skyline::hooks::InlineCtx) {
    let raw_kind = ctx.registers[9].x() as u32 as i32;
    let owner = ctx.registers[2].x();
    let target = active_kirby_copy_kind()
        .map(|kind| (kind, "copy-setup"))
        .or_else(|| {
            if tracked_kirby_article_owner(owner) {
                let kind = KIRBY_CLONE_COPY_KIND.load(core::sync::atomic::Ordering::Acquire);
                clone_definition(kind).map(|_| (kind, "persistent-owner"))
            } else {
                None
            }
        });
    if let Some((target_kind, source)) = target {
        if let Some(definition) = clone_definition(target_kind) {
            if raw_kind == target_kind || raw_kind == definition.base_kind {
                if let Some(slot) = custom_articles::kirby_copy_header_slot(target_kind) {
                    let biased = (slot as u64).wrapping_sub((raw_kind as u32 as u64) * 8);
                    ctx.registers[10].set_x(biased);
                    static LOG: core::sync::atomic::AtomicU32 =
                        core::sync::atomic::AtomicU32::new(0);
                    let n = LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                    if n < 24 {
                        dbg_log!(
                            "[copyarticle] callback table target={target_kind} raw_kind={raw_kind} owner={owner:#x} slot={slot:#x} source={source}"
                        );
                    }
                    return;
                }
            }
        }
    }
    ctx.registers[9].set_x(i64::from(bridged_clone_kind(raw_kind)) as u64);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_CALLBACK_KIND_1, inline)]
pub(crate) unsafe fn kirby_copy_callback_kind_1(ctx: &mut skyline::hooks::InlineCtx) {
    load_bridged_kirby_callback_kind(ctx);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_CALLBACK_KIND_2, inline)]
pub(crate) unsafe fn kirby_copy_callback_kind_2(ctx: &mut skyline::hooks::InlineCtx) {
    load_bridged_kirby_callback_kind(ctx);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_CALLBACK_KIND_3, inline)]
pub(crate) unsafe fn kirby_copy_callback_kind_3(ctx: &mut skyline::hooks::InlineCtx) {
    load_bridged_kirby_callback_kind(ctx);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_RESOURCE_KIND, inline)]
pub(crate) unsafe fn kirby_copy_resource_kind(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[20].x() as i32;
    let active = active_kirby_copy_kind();
    let resource = ctx.registers[0].x();
    if active == Some(kind)
        && clone_definition(kind).is_some_and(|definition| definition.kirby_copy_full_model)
        && resource != 0
    {
        let pending_resource = *((resource as usize + 0x160) as *const u64);
        let pending_owner = *((resource as usize + 0x168) as *const u64);
        let source_resource = *((resource as usize + 0x10) as *const u64);
        let source_owner = *((resource as usize + 0x18) as *const u64);
        let n = KIRBY_COPY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[kirbycopy] #{n} full-model transfer prepared kind={kind} target={active:?} resource={resource:#x} pending={pending_resource:#x}/{pending_owner:#x} source={source_resource:#x}/{source_owner:#x}"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_RECORD_SUB: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_RECORD_PAIR_COLORS: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_RECORD_BUILD_LOCK: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_RECORD_BUILD_OWNER: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_SLOT_WALKER_ACTIVE: crate::thread_context::ThreadReentrancyFlag =
    crate::thread_context::ThreadReentrancyFlag::new("kirby_slot_walker");

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn kirby_record_mutex_held_here() -> bool {
    KIRBY_SLOT_WALKER_ACTIVE.is_active(current_thread_key())
}

#[cfg(feature = "css_slot")]
unsafe fn kirby_record_build_acquire() -> Option<bool> {
    use core::sync::atomic::Ordering;

    let thread = current_thread_key();
    if thread != 0 && KIRBY_RECORD_BUILD_OWNER.load(Ordering::Acquire) == thread {
        return Some(false);
    }
    for _ in 0..100_000 {
        if KIRBY_RECORD_BUILD_LOCK
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            KIRBY_RECORD_BUILD_OWNER.store(thread, Ordering::Release);
            return Some(true);
        }
        core::hint::spin_loop();
    }
    dbg_log!("[kirbyrec] build lock busy; skipping record build rather than blocking the load");
    None
}

#[cfg(feature = "css_slot")]
fn kirby_record_build_release(owned: bool) {
    use core::sync::atomic::Ordering;

    if !owned {
        return;
    }
    KIRBY_RECORD_BUILD_OWNER.store(0, Ordering::Release);
    KIRBY_RECORD_BUILD_LOCK.store(false, Ordering::Release);
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn kirby_record_find(container: u64, kind: i32) -> u64 {
    if container == 0 {
        return 0;
    }
    for slot_index in 0..KIRBY_RECORD_SLOT_COUNT {
        let slot =
            container as usize + KIRBY_RECORD_TABLE_OFFSET + slot_index * KIRBY_RECORD_SLOT_STRIDE;
        if *(slot as *const i32) == kind {
            return slot as u64;
        }
    }
    0
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn kirby_record_model_pair_mask(record: u64) -> u32 {
    if record == 0 {
        return 0;
    }
    let mut mask = 0u32;
    for color in 0..8usize {
        let color_record = record as usize + color * KIRBY_RECORD_COLOR_STRIDE;
        if *((color_record + KIRBY_RECORD_MEMBER1_OFFSET) as *const u64) != 0 {
            mask |= 1 << color;
        }
    }
    mask
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_RECORD_ATTEMPTED_COLORS: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[cfg(feature = "css_slot")]
pub(crate) unsafe fn vanilla_kirby_copy_name(kind: i32) -> Option<&'static [u8]> {
    if !(0..KIRBY_COPY_NAME_COUNT).contains(&kind) {
        return None;
    }
    let entry = (text_base() + KIRBY_COPY_NAME_TABLE) as *const *const u8;
    let record = core::ptr::read_volatile(entry.add(kind as usize));
    if record.is_null() {
        return None;
    }
    let name = core::ptr::read_volatile(record as *const *const core::ffi::c_char);
    if name.is_null() {
        return None;
    }
    Some(core::ffi::CStr::from_ptr(name).to_bytes())
}

pub(crate) unsafe fn ensure_kirby_copy_model_record(
    kind: i32,
    color: i32,
    record_mutex_held: bool,
) -> (u64, u32) {
    let Some(definition) = clone_definition(kind) else {
        return (0, 0);
    };
    if !(0..8).contains(&color) {
        return (0, 0);
    }
    let color = color as u32;
    let tb = text_base();
    let manager = *((tb + OFF_RESOURCE_MANAGER_GLOBAL) as *const u64);
    if manager == 0 {
        return (0, 0);
    }
    let recursive_lock: extern "C" fn(u64) =
        core::mem::transmute(tb + OFF_STD_RECURSIVE_MUTEX_LOCK);
    let recursive_unlock: extern "C" fn(u64) =
        core::mem::transmute(tb + OFF_STD_RECURSIVE_MUTEX_UNLOCK);
    recursive_lock(manager + 0xc8);
    let sub = *((manager as usize + 0x68) as *const u64);
    recursive_unlock(manager + 0xc8);
    if sub == 0 {
        return (0, 0);
    }

    let live_record = kirby_record_find(sub, kind);
    if live_record != 0 {
        return (live_record, kirby_record_model_pair_mask(live_record));
    }

    let Some(lock_owned) = kirby_record_build_acquire() else {
        return (0, 0);
    };
    if KIRBY_RECORD_SUB.load(core::sync::atomic::Ordering::Acquire) != sub {
        KIRBY_RECORD_SUB.store(sub, core::sync::atomic::Ordering::Release);
        KIRBY_RECORD_ATTEMPTED_COLORS.store(0, core::sync::atomic::Ordering::Release);
        KIRBY_RECORD_PAIR_COLORS.store(0, core::sync::atomic::Ordering::Release);
    }
    let mut attempted = KIRBY_RECORD_ATTEMPTED_COLORS.load(core::sync::atomic::Ordering::Acquire);
    if attempted & (1 << color) != 0 {
        let record = kirby_record_find(sub, kind);
        if record != 0 {
            let result = (
                record,
                KIRBY_RECORD_PAIR_COLORS.load(core::sync::atomic::Ordering::Acquire),
            );
            kirby_record_build_release(lock_owned);
            return result;
        }

        KIRBY_RECORD_ATTEMPTED_COLORS.store(0, core::sync::atomic::Ordering::Release);
        KIRBY_RECORD_PAIR_COLORS.store(0, core::sync::atomic::Ordering::Release);
        attempted = 0;
        dbg_log!(
            "[kirbyrec] in-place table reset kind={kind} color={color} sub={sub:#x}; stale attempt cache cleared"
        );
    }
    KIRBY_RECORD_ATTEMPTED_COLORS.store(
        attempted | (1 << color),
        core::sync::atomic::Ordering::Release,
    );

    let mutex_lock: extern "C" fn(u64) = core::mem::transmute(tb + OFF_STD_MUTEX_LOCK);
    let mutex_unlock: extern "C" fn(u64) = core::mem::transmute(tb + OFF_STD_MUTEX_UNLOCK);
    let record_mutex_held = record_mutex_held || kirby_record_mutex_held_here();
    if !record_mutex_held {
        mutex_lock(sub + 0x1d238);
    }
    let mut slot = kirby_record_find(sub, kind);
    let claimed = slot == 0;
    if claimed {
        slot = kirby_record_find(sub, -1);
        if slot != 0 {
            core::ptr::write_volatile(slot as *mut i32, kind);
        }
    }
    if !record_mutex_held {
        mutex_unlock(sub + 0x1d238);
    }
    if slot == 0 {
        dbg_log!("[kirbyrec] no free record slot for kind={kind} sub={sub:#x}");
        kirby_record_build_release(lock_owned);
        return (0, 0);
    }

    let resource_name = definition.resource_name.as_bytes();
    let mut name = [0u8; 48];
    let mut length = 0usize;
    for chunk in [b"copy_" as &[u8], resource_name, b"_fitkirby"] {
        name[length..length + chunk.len()].copy_from_slice(chunk);
        length += chunk.len();
    }
    let own = String::from_utf8_lossy(&name[..length]).into_owned();
    if !crate::fighter_modules::path_exists(&format!(
        "fighter/kirby/model/{own}/c{:02}/model.numdlb",
        color
    )) {
        dbg_log!(
            "[kirbyrec] kind={kind} ships no 'fighter/kirby/model/{own}/c{color:02}'; Kirby will \
             copy it with no hat. Ship that model and declare it in `new-dir-files` under a group \
             this fighter loads, as every working clone pack does."
        );
    }

    let colorrec = slot as usize + color as usize * KIRBY_RECORD_COLOR_STRIDE;
    let member1 = (colorrec + KIRBY_RECORD_MEMBER1_OFFSET) as u64;
    let member_builder: extern "C" fn(u64, u32, u32, i32, u64, *const u8, u32, u32) =
        core::mem::transmute(tb + OFF_KIRBY_COPY_MEMBER_BUILDER);
    let member_builder_2: extern "C" fn(u64, u32, u32, u64, *const u8, u32) =
        core::mem::transmute(tb + OFF_KIRBY_COPY_MEMBER_BUILDER_2);

    if *((colorrec + KIRBY_RECORD_MEMBER1_OFFSET) as *const u64) == 0 {
        member_builder(
            sub,
            kind as u32,
            color,
            0,
            member1,
            name.as_ptr(),
            KIRBY_RECORD_MODEL_TYPE,
            0,
        );
    }
    member_builder_2(
        sub,
        kind as u32,
        color,
        member1,
        name.as_ptr(),
        KIRBY_RECORD_MODEL_TYPE,
    );

    let model_resource = *((colorrec + 0x20) as *const u64);
    let model_owner = *((colorrec + 0x28) as *const u64);
    let motion_resource = *((colorrec + 0xc0) as *const u64);
    let motion_owner = *((colorrec + 0xc8) as *const u64);
    let mut pair_mask = KIRBY_RECORD_PAIR_COLORS.load(core::sync::atomic::Ordering::Acquire);
    if model_resource != 0 {
        pair_mask |= 1 << color;
        KIRBY_RECORD_PAIR_COLORS.store(pair_mask, core::sync::atomic::Ordering::Release);
    }
    kirby_record_build_release(lock_owned);
    dbg_log!(
        "[kirbyrec] build kind={kind} color={color} claimed={claimed} slot={slot:#x} model={model_resource:#x}/{model_owner:#x} motion={motion_resource:#x}/{motion_owner:#x} colors={pair_mask:#x}"
    );
    (slot, pair_mask)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_RESOURCE_TRANSFER)]
pub(crate) unsafe fn kirby_copy_resource_transfer(
    resource: u64,
    kind: i32,
    color: i32,
    variant: i32,
) {
    let active = active_kirby_copy_kind();
    if active == Some(kind)
        && clone_definition(kind).is_some_and(|definition| definition.kirby_copy_full_model)
    {
        let (record, pair_mask) = ensure_kirby_copy_model_record(kind, color, false);
        let n = KIRBY_COPY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if record != 0 && (0..8).contains(&color) && pair_mask & (1 << color) != 0 {
            if n < 24 {
                dbg_log!(
                    "[kirbycopy] #{n} full-model transfer native kind={kind} carrier={KIRBY_FULL_MODEL_BRANCH_KIND} record={record:#x} colors={pair_mask:#x} resource={resource:#x} color={color} variant={variant}"
                );
            }
            call_original!(resource, KIRBY_FULL_MODEL_BRANCH_KIND, color, variant);
            return;
        }
        if n < 24 {
            dbg_log!(
                "[kirbycopy] #{n} full-model transfer bypass (record unavailable) kind={kind} record={record:#x} colors={pair_mask:#x} resource={resource:#x}"
            );
        }
        return;
    }
    call_original!(resource, kind, color, variant);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_TRANSFER_LOOKUP, inline)]
pub(crate) unsafe fn kirby_copy_transfer_lookup(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(kind) = active_kirby_copy_kind() else {
        return;
    };
    let sub = ctx.registers[0].x();
    if kirby_record_find(sub, kind) == 0 {
        return;
    }
    let before = ctx.registers[1].x() as u32;
    ctx.registers[1].set_x(kind as u32 as u64);
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!("[kirbyrec] #{n} site=transfer-lookup kind {before}->{kind} sub={sub:#x}");
    }
}

#[cfg(all(feature = "css_slot", feature = "native_table_backend"))]
#[skyline::hook(offset = OFF_KIRBY_COPY_MODEL_NAME_TABLE, inline)]
pub(crate) unsafe fn kirby_copy_model_name_table(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[20].x() as u32 as i32;
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    let Some(table_base) = native_tables::published_lower_name_table_base(definition) else {
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!(
                "[kirbyname] #{n} lowercase table unavailable kind={kind}; leaving native lookup unchanged"
            );
        }
        return;
    };
    ctx.registers[8].set_x(table_base as u64);
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!(
            "[kirbyname] #{n} lowercase table kind={kind} namespace={}",
            definition.resource_name
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_MODEL_NAME, inline)]
pub(crate) unsafe fn kirby_copy_model_name(ctx: &mut skyline::hooks::InlineCtx) {
    let raw_kind = ctx.registers[20].x() as i32;
    if let Some(definition) = clone_definition(raw_kind) {
        ctx.registers[2].set_x(definition.resource_name_cstr.as_ptr() as u64);
        let site_n = KIRBY_MODEL_SITE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if site_n < 32 {
            dbg_log!(
                "[kirbymodel] #{site_n} site=primary raw_kind={raw_kind} direct namespace=copy_{}_fitkirby",
                definition.resource_name
            );
        }
        return;
    }
    let boma = ctx.registers[26].x();
    let target_entry = if boma == 0 {
        -1
    } else {
        let work = *((boma as usize + 0x50) as *const u64);
        if work == 0 {
            -1
        } else {
            let vt = *(work as *const u64);
            let get_int: extern "C" fn(u64, u32) -> u64 =
                core::mem::transmute(*((vt as usize + 0x98) as *const u64));
            get_int(work, 0x1000_00FD) as i32
        }
    };
    let target_kind = if (0..8).contains(&target_entry) {
        entry_custom_kind(target_entry as u8)
    } else {
        None
    };
    let site_n = KIRBY_MODEL_SITE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if site_n < 32 {
        dbg_log!(
            "[kirbymodel] #{site_n} site=primary raw_kind={raw_kind} target_entry={target_entry} target_kind={target_kind:?}"
        );
    }
    if let Some(kind) = target_kind {
        let definition = clone_definition(kind).unwrap();
        ctx.registers[2].set_x(definition.resource_name_cstr.as_ptr() as u64);
        let n = KIRBY_COPY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[kirbycopy] #{n} model target_kind={kind} namespace=copy_{}_fitkirby",
                definition.resource_name
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_HANDLE_PROBE, inline)]
pub(crate) unsafe fn kirby_copy_handle_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(kind) = active_kirby_copy_kind() else {
        return;
    };
    let handle = ctx.registers[8].x() as u32;
    let raw_kind = ctx.registers[20].x() as i32;
    let fighter = ctx.registers[24].x();
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!(
            "[kirbyhandle] #{n} copy-model handle={handle:#x} target_kind={kind} raw_kind={raw_kind} fighter={fighter:#x}"
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_MODEL_BASE_KIND, inline)]
pub(crate) unsafe fn kirby_copy_model_base_kind(ctx: &mut skyline::hooks::InlineCtx) {
    if let Some(kind) = active_kirby_copy_kind() {
        let definition = clone_definition(kind).unwrap();
        let root = ctx.registers[0].x();
        let before = ctx.registers[1].x() as u32 as i32;
        let record = kirby_record_find(root, kind);
        let target = if record != 0 {
            kind
        } else {
            definition.base_kind
        };
        ctx.registers[1].set_x(target as u32 as u64);
        let site_n = KIRBY_MODEL_SITE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if site_n < 32 {
            dbg_log!(
                "[kirbymodel] #{site_n} site=model-record root={root:#x} kind={before}->{target} record={record:#x} target_kind={kind}"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_BASE_MODEL_PAIR, inline)]
pub(crate) unsafe fn kirby_copy_base_model_pair(ctx: &mut skyline::hooks::InlineCtx) {
    if let Some(kind) = active_kirby_copy_kind() {
        let native_resource = ctx.registers[9].x();
        let native_owner = ctx.registers[8].x();
        let site_n = KIRBY_MODEL_SITE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if site_n < 32 {
            dbg_log!(
                "[kirbymodel] #{site_n} site=model-pair target={kind} native={native_resource:#x}/{native_owner:#x} (record-built, no injection)"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_KIND_LIST_TABLE, inline)]
pub(crate) unsafe fn kirby_copy_kind_list_table(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[20].x() as i32;
    let base = active_kirby_copy_kind()
        .and_then(clone_definition)
        .map(|definition| definition.base_kind)
        .unwrap_or_else(|| bridged_clone_kind(kind));
    let bias = i64::from(base - kind) * 8;
    ctx.registers[8].set_x((ctx.registers[8].x() as i64).wrapping_add(bias) as u64);
    if kind != base {
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!("[kirbyhat] #{n} site=hat-list-table kind={kind}->{base}");
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_SECOND_NAME_MERGE, inline)]
pub(crate) unsafe fn kirby_copy_second_name_merge(ctx: &mut skyline::hooks::InlineCtx) {
    let raw_kind = ctx.registers[20].x() as i32;
    let site_n = KIRBY_MODEL_SITE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if site_n < 32 {
        dbg_log!(
            "[kirbymodel] #{site_n} site=secondary raw_kind={} context={:?}",
            raw_kind,
            active_kirby_copy_kind()
        );
    }
    let kind = clone_definition(raw_kind)
        .map(|_| raw_kind)
        .or_else(active_kirby_copy_kind);
    if let Some(kind) = kind {
        let definition = clone_definition(kind).unwrap();
        ctx.registers[2].set_x(definition.base_resource_name_cstr.as_ptr() as u64);
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!(
                "[kirbyhat] #{n} site=body-motion kind={kind} namespace={}body",
                definition.base_resource_name
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_CALLBACK_KIND_4, inline)]
pub(crate) unsafe fn kirby_copy_callback_kind_4(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[20].x() as i32;
    if let Some(target_kind) = active_kirby_copy_kind() {
        if let Some(definition) = clone_definition(target_kind) {
            if kind == target_kind || kind == definition.base_kind {
                if let Some(slot) = custom_articles::kirby_copy_header_slot(target_kind) {
                    let biased = (slot as u64)
                        .wrapping_sub((kind as u32 as u64) * 8)
                        .wrapping_sub(0x98);
                    ctx.registers[12].set_x(biased);
                    if let Some(n) = kirby_hat_flow_log_index() {
                        dbg_log!(
                            "[kirbyhat] #{n} site=hat-record-found kind={kind} callback-table=custom({target_kind})"
                        );
                    }
                    return;
                }
            }
        }
    }
    let base = active_kirby_copy_kind()
        .and_then(clone_definition)
        .map(|definition| definition.base_kind)
        .unwrap_or_else(|| bridged_clone_kind(kind));
    let bias = i64::from(base - kind) * 8;
    ctx.registers[12].set_x((ctx.registers[12].x() as i64).wrapping_add(bias) as u64);
    if kind != base {
        if let Some(n) = kirby_hat_flow_log_index() {
            dbg_log!("[kirbyhat] #{n} site=hat-record-found kind={kind} callback-table={base}");
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_BODYMOTION_HANDLE, inline)]
pub(crate) unsafe fn kirby_copy_bodymotion_handle_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=bodymotion-handle handle={:#x} kind={} context={:?}",
        ctx.registers[8].x() as u32,
        ctx.registers[20].x() as u32 as i32,
        active_kirby_copy_kind()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_ROW_SEARCH, inline)]
pub(crate) unsafe fn kirby_copy_row_search_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    let link = ctx.registers[19].x();
    let container = if link == 0 {
        0
    } else {
        *((link as usize + 0x258) as *const u64)
    };
    let (count, row0_kind) = if container == 0 {
        (-1i32, -2i32)
    } else {
        let count = *((container as usize + 0x10) as *const i32);
        let rows = *((container as usize + 0x18) as *const u64);
        let row0 = if rows != 0 && count > 0 {
            *(rows as *const i32)
        } else {
            -2
        };
        (count, row0)
    };
    dbg_log!(
        "[kirbyhat] #{n} site=row-search w20={} container={container:#x} count={count} row0_kind={row0_kind} context={:?}",
        ctx.registers[20].x() as u32 as i32,
        active_kirby_copy_kind()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_HAT_RECORD_REGION, inline)]
pub(crate) unsafe fn kirby_copy_hat_record_region_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = kirby_hat_flow_log_index() else {
        return;
    };
    dbg_log!(
        "[kirbyhat] #{n} site=hat-record-region w20={} context={:?}",
        ctx.registers[20].x() as u32 as i32,
        active_kirby_copy_kind()
    );
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn bridge_removal_hashlist(
    ctx: &mut skyline::hooks::InlineCtx,
    dest_reg: usize,
    site: &str,
) {
    let previous = ctx.registers[27].x() as u32 as i32;
    let base = bridged_clone_kind(previous);
    if base == previous || !(0..94).contains(&base) {
        return;
    }
    let entry = *((text_base() + KIRBY_COPY_KIND_HASHLIST_TABLE + base as usize * 8) as *const u64);
    ctx.registers[dest_reg].set_x(entry);
    if let Some(n) = kirby_hat_flow_log_index() {
        dbg_log!("[kirbyhat] #{n} site={site} previous={previous}->{base} entry={entry:#x}");
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_REMOVAL_HASHLIST_A, inline)]
pub(crate) unsafe fn kirby_copy_removal_hashlist_a(ctx: &mut skyline::hooks::InlineCtx) {
    bridge_removal_hashlist(ctx, 9, "removal-hashlist-a");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_REMOVAL_HASHLIST_B, inline)]
pub(crate) unsafe fn kirby_copy_removal_hashlist_b(ctx: &mut skyline::hooks::InlineCtx) {
    bridge_removal_hashlist(ctx, 20, "removal-hashlist-b");
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn bridge_copy_chara_table_index(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[0].x() as i32;
    if let Some(definition) = clone_definition(kind) {
        ctx.registers[0].set_x(definition.base_kind as u32 as u64);
        let n = KIRBY_COPY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[kirbycopy] #{n} chara-table kind={kind}->{}",
                definition.base_kind
            );
        }
    }
}

macro_rules! copy_chara_table_hooks {
    ($($name:ident($offset:ident);)*) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                bridge_copy_chara_table_index(ctx);
            }
        )*
    };
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_MOTION_BIND_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) fn kirby_motion_bind_log_index() -> Option<u32> {
    let n = KIRBY_MOTION_BIND_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    (n < 48).then_some(n)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_COPY_MOTION_BIND_TAIL_KIND, inline)]
pub(crate) unsafe fn copy_motion_bind_tail_kind(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[0].x() as u32 as i32;
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    ctx.registers[0].set_x(definition.base_kind as u32 as u64);
    if let Some(n) = kirby_motion_bind_log_index() {
        dbg_log!(
            "[kirbymotion] #{n} tail kind={kind}->{} requested={:#x} mapped={:#x}",
            definition.base_kind,
            ctx.registers[21].x(),
            ctx.registers[25].x()
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static COPYSET_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) fn copyset_log_index() -> Option<u32> {
    let n = COPYSET_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    (n < 96).then_some(n)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_SETUP)]
pub(crate) unsafe fn kirby_copy_setup_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let base_kind = x2 as i32;
    let target_entry = x1 as i32;
    let target_clone_kind = if (0..8).contains(&target_entry) {
        entry_custom_kind(target_entry as u8).filter(|kind| {
            clone_definition(*kind)
                .map(|definition| definition.base_kind == base_kind)
                .unwrap_or(false)
        })
    } else {
        None
    };
    let idx = copyset_log_index();
    if let Some(n) = idx {
        dbg_log!(
            "[copyset] #{n} copy_setup enter boma={x0:#x} kind={base_kind} target_entry={target_entry} target_clone={target_clone_kind:?} w3={:#x} w4={:#x}",
            x3 as u32,
            x4 as u32
        );
    }
    let ret = if let Some(kind) = target_clone_kind {
        KIRBY_CLONE_COPY_KIND.store(kind, core::sync::atomic::Ordering::Release);
        KIRBY_CLONE_COPY_BOMA.store(x0, core::sync::atomic::Ordering::Release);
        with_kirby_copy_context(kind, || call_original!(x0, x1, x2, x3, x4, x5, x6, x7))
    } else {
        call_original!(x0, x1, x2, x3, x4, x5, x6, x7)
    };
    if let Some(n) = idx {
        dbg_log!("[copyset] #{n} copy_setup exit ret={ret:#x}");
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_SETUP_SHIM)]
pub(crate) unsafe fn kirby_copy_setup_shim_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    if let Some(n) = copyset_log_index() {
        dbg_log!("[copyset] #{n} shim fired state={x0:#x} x1={x1:#x}");
    }
    call_original!(x0, x1, x2, x3, x4, x5, x6, x7)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_SV_BATTLE_OBJECT_KIND)]
pub(crate) unsafe fn sv_battle_object_kind_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let ret = call_original!(x0, x1, x2, x3);
    if clone_definition(ret as u32 as i32).is_some() {
        if let Some(n) = copyset_log_index() {
            dbg_log!(
                "[copyset] #{n} svkind id={:#x} -> {}",
                x0 as u32,
                ret as u32 as i32
            );
        }
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_GET_COPY_KIND)]
pub(crate) unsafe fn kirby_get_copy_kind_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let ret = call_original!(x0, x1, x2, x3);
    if let Some(n) = copyset_log_index() {
        dbg_log!("[copyset] #{n} get_copy_kind -> {}", ret as u32 as i32);
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_GET_COPY_SLOT_NO)]
pub(crate) unsafe fn kirby_get_copy_slot_no_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let ret = call_original!(x0, x1, x2, x3);
    if let Some(n) = copyset_log_index() {
        dbg_log!("[copyset] #{n} get_copy_slot_no -> {}", ret as u32 as i32);
    }
    ret
}

#[cfg(feature = "css_slot")]
pub(crate) static COPYTRACK_RESET_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_ABILITY_RESET)]
pub(crate) unsafe fn kirby_copy_ability_reset_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller_off = lr.wrapping_sub(text_base());
    let n = COPYTRACK_RESET_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[copytrack] #{n} RESET fighter={x0:#x} w1={:#x} lr_raw={lr:#x} lr_off=@{caller_off:#x}",
            x1 as u32
        );
    }
    call_original!(x0, x1, x2, x3, x4, x5, x6, x7)
}

#[cfg(feature = "css_slot")]
pub(crate) static COPYTRACK_STATE: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(u64::MAX);

#[cfg(feature = "css_slot")]
pub(crate) static COPYTRACK_STATE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_CLONE_COPY_BOMA: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_CLONE_COPY_KIND: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_PER_FIGHTER_FRAME)]
pub(crate) unsafe fn kirby_per_fighter_frame_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let ret = call_original!(x0, x1, x2, x3);
    if x1 != 0 {
        let boma = *((x1 as usize + 0x20) as *const u64);
        if boma != 0 {
            let work = *((boma as usize + 0x50) as *const u64);
            if work != 0 {
                let vt = *(work as *const u64);
                let is_flag: extern "C" fn(u64, u32) -> u64 =
                    core::mem::transmute(*((vt as usize + 0x108) as *const u64));
                let get_int: extern "C" fn(u64, u32) -> u64 =
                    core::mem::transmute(*((vt as usize + 0x98) as *const u64));
                let flag = is_flag(work, 0x2000_0102) & 1;
                let kind = get_int(work, 0x1000_00FC) as u32;
                let target_entry = get_int(work, 0x1000_00FD) as u32 as i32;
                let target_kind = if (0..8).contains(&target_entry) {
                    entry_custom_kind(target_entry as u8)
                } else {
                    None
                };
                if flag != 0 && target_kind.is_some() {
                    KIRBY_CLONE_COPY_KIND
                        .store(target_kind.unwrap(), core::sync::atomic::Ordering::Release);
                    KIRBY_CLONE_COPY_BOMA.store(boma, core::sync::atomic::Ordering::Release);
                    poll_kirby_copy_article_state(boma);
                } else {
                    if KIRBY_CLONE_COPY_BOMA
                        .compare_exchange(
                            boma,
                            0,
                            core::sync::atomic::Ordering::AcqRel,
                            core::sync::atomic::Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        KIRBY_CLONE_COPY_KIND.store(-1, core::sync::atomic::Ordering::Release);
                        KIRBY_COPY_ARTICLE_POLL_STATE
                            .store(u64::MAX, core::sync::atomic::Ordering::Release);
                    }
                }
                let state = (kind as u64) | (flag << 32);
                let prev = COPYTRACK_STATE.swap(state, core::sync::atomic::Ordering::Relaxed);
                if prev != state {
                    let n = COPYTRACK_STATE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                    if n < 64 {
                        dbg_log!(
                            "[copytrack] #{n} state flag={flag} copy_int={} target_entry={target_entry} target_kind={target_kind:?} boma={boma:#x}",
                            kind as i32,
                        );
                    }
                }
            }
        }
    }
    trace_kirby_copy_article_lifecycle();
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_COPY_SETUP_GATE1, inline)]
pub(crate) unsafe fn copy_setup_gate1_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = copyset_log_index() else { return };
    dbg_log!(
        "[copyset] #{n} gate1 kirbylink={:#x} kind={}",
        ctx.registers[0].x(),
        ctx.registers[20].x() as i32
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_COPY_SETUP_SLOT, inline)]
pub(crate) unsafe fn copy_setup_slot_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = copyset_log_index() else { return };
    dbg_log!("[copyset] #{n} slot={}", ctx.registers[25].x() as u32);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_COPY_SETUP_DISPATCH, inline)]
pub(crate) unsafe fn copy_setup_dispatch_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = copyset_log_index() else { return };
    dbg_log!(
        "[copyset] #{n} dispatch kind={}",
        ctx.registers[20].x() as i32
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_COPY_SETUP_GRANT, inline)]
pub(crate) unsafe fn copy_setup_grant_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = copyset_log_index() else { return };
    dbg_log!(
        "[copyset] #{n} GRANT set_int(0x100000FC) kind={}",
        ctx.registers[20].x() as i32
    );
}

#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_NRO_DISPATCH_SEAM: usize = 0x237294;
#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_NRO_DISPATCH_START: usize = 0x236ce0;
#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_NRO_DISPATCH_END: usize = 0x239efc;
#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_NRO_DISPATCH_SEAM_OPCODE: u32 = 0xAA0003E1;
#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_NRO_DISPATCH_NEXT_OPCODE: u32 = 0xF940_0380;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_STATUS_SET_KIND_INTERRUPT: usize = 0x2087740;
#[cfg(feature = "css_slot")]
#[allow(dead_code)]
pub(crate) const KIRBY_STATUS_SAMUS_SPECIAL_N: u64 = 0x287;

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_NRO_BASE: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_FAMILY_ARMED: OnceLock<RwLock<HashMap<i32, i32>>> = OnceLock::new();

#[cfg(feature = "css_slot")]
pub(crate) fn armed_kirby_copy_families() -> &'static RwLock<HashMap<i32, i32>> {
    KIRBY_COPY_FAMILY_ARMED.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_FAM_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[no_mangle]
pub extern "C" fn clone_engine_arm_kirby_copy_status_family(
    kind: i32,
    first_status: i32,
    count: i32,
) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        let Some(definition) = clone_definition(kind) else {
            skyline::println!(
                "[kirbyfam] arm REJECTED: kind {kind} is not a registered clone definition"
            );
            return 0;
        };
        if definition.copy_status_first < 0
            || definition.copy_status_first != first_status
            || definition.copy_status_count != count
        {
            skyline::println!(
                "[kirbyfam] arm REJECTED: kind {kind} offered family {first_status:#x}+{count}, descriptor has {:#x}+{}",
                definition.copy_status_first,
                definition.copy_status_count
            );
            return 0;
        }
        armed_kirby_copy_families()
            .write()
            .unwrap()
            .insert(kind, first_status);
        skyline::println!(
            "[kirbyfam] armed: clone kind {kind} (base {}) routes Kirby's copy dispatch to {first_status:#x} (+{count} family)",
            definition.base_kind
        );
        1
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (kind, first_status, count);
        0
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn kirby_copy_routed_status(boma: u64, native_status: u64) -> Option<i32> {
    if boma == 0 {
        return None;
    }
    let work = *((boma as usize + 0x50) as *const u64);
    if work == 0 {
        return None;
    }
    let vt = *(work as *const u64);
    if vt == 0 {
        return None;
    }
    let get_int: extern "C" fn(u64, u32) -> u64 =
        core::mem::transmute(*((vt as usize + 0x98) as *const u64));
    let target_entry = get_int(work, 0x1000_00FD) as i32;
    if !(0..8).contains(&target_entry) {
        return None;
    }
    let kind = entry_custom_kind(target_entry as u8)?;
    clone_definition(kind)?;
    let first_status = armed_kirby_copy_families()
        .read()
        .ok()?
        .get(&kind)
        .copied()
        .unwrap_or(-1);
    let n = KIRBY_FAM_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if first_status < 0 {
        if n < 16 {
            dbg_log!(
                "[kirbyfam] #{n} entry={target_entry} kind={kind} copy family NOT armed; native {native_status:#x} kept"
            );
        }
        return None;
    }
    if !KIRBY_FAMILY_OWNED_ROUTING {
        if n < 16 {
            dbg_log!(
                "[kirbyfam] #{n} entry={target_entry} kind={kind} family {first_status:#x} armed but owned routing disabled; native {native_status:#x} kept"
            );
        }
        return None;
    }
    if n < 32 {
        dbg_log!(
            "[kirbyfam] #{n} ROUTE entry={target_entry} kind={kind} status {native_status:#x} -> {first_status:#x}"
        );
    }
    Some(first_status)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_STATUS_SET_KIND_INTERRUPT)]
pub(crate) unsafe fn kirby_copy_dispatch_status(boma: u64, status: u64) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let base = KIRBY_NRO_BASE.load(core::sync::atomic::Ordering::Relaxed);
    let in_dispatcher =
        base != 0 && (base + KIRBY_NRO_DISPATCH_START..base + KIRBY_NRO_DISPATCH_END).contains(&lr);
    if !in_dispatcher {
        return call_original!(boma, status);
    }

    match kirby_copy_routed_status(boma, status) {
        Some(first_status) => call_original!(boma, first_status as u64),
        None => call_original!(boma, status),
    }
}

#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_FAMILY_OWNED_ROUTING: bool = true;

#[cfg(feature = "css_slot")]
pub(crate) fn kirby_copy_family_nro_hook(info: &skyline::nro::NroInfo) {
    if info.name != "kirby" {
        if info.name.contains("kirby") {
            skyline::println!(
                "[kirbyfam] NOTE: NRO event '{}' contains 'kirby' but is not the expected module name",
                info.name
            );
        }
        return;
    }
    unsafe {
        let module_object = info.module.ModuleObject;
        if module_object.is_null() {
            skyline::println!("[kirbyfam] ERROR: kirby NRO event without ModuleObject");
            return;
        }
        let base = (*module_object).module_base as usize;

        let anchor = base + KIRBY_NRO_DISPATCH_SEAM;
        let opcode = *(anchor as *const u32);
        let next = *((anchor + 4) as *const u32);
        if opcode != KIRBY_NRO_DISPATCH_SEAM_OPCODE || next != KIRBY_NRO_DISPATCH_NEXT_OPCODE {
            KIRBY_NRO_BASE.store(0, core::sync::atomic::Ordering::Relaxed);
            skyline::println!(
                "[kirbyfam] anchor seam mismatch at {anchor:#x} ({opcode:#010x}/{next:#010x}); copy-family routing inert this load"
            );
            return;
        }

        KIRBY_NRO_BASE.store(base, core::sync::atomic::Ordering::Relaxed);
        skyline::println!(
            "[kirbyfam] dispatcher range {:#x}..{:#x} armed (kirby base {base:#x}); one main-side hook routes every branch, so copy families work for a clone of ANY base fighter.",
            base + KIRBY_NRO_DISPATCH_START,
            base + KIRBY_NRO_DISPATCH_END
        );
    }
}

copy_chara_table_hooks! {
    copy_chara_table_impl_1(OFF_COPY_CHARA_IMPL_1);
    copy_chara_table_impl_2(OFF_COPY_CHARA_IMPL_2);
    copy_chara_table_impl_3(OFF_COPY_CHARA_IMPL_3);
    copy_chara_table_thunk_1(OFF_COPY_CHARA_THUNK_1);
    copy_chara_table_thunk_2(OFF_COPY_CHARA_THUNK_2);
    copy_chara_table_thunk_3(OFF_COPY_CHARA_THUNK_3);
    copy_chara_table_thunk_4(OFF_COPY_CHARA_THUNK_4);
    copy_chara_table_thunk_5(OFF_COPY_CHARA_THUNK_5);
    copy_chara_table_thunk_6(OFF_COPY_CHARA_THUNK_6);
    copy_chara_table_thunk_7(OFF_COPY_CHARA_THUNK_7);
    copy_chara_table_thunk_8(OFF_COPY_CHARA_THUNK_8);
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_REG_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_RECORD_CREATOR_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_MEMBER_BUILDER_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_NATIVE_RECORD_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_REG_LOG_LIMIT: u32 = 400;

#[cfg(feature = "css_slot")]
pub(crate) static CLONE_SLOT_REGISTRATION_THREAD: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static CLONE_SLOT_REGISTRATION_KIND: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn active_clone_slot_kind() -> Option<i32> {
    let thread = current_thread_key();
    if thread == 0
        || CLONE_SLOT_REGISTRATION_THREAD.load(core::sync::atomic::Ordering::Acquire) != thread
    {
        return None;
    }
    let kind = CLONE_SLOT_REGISTRATION_KIND.load(core::sync::atomic::Ordering::Acquire);
    clone_definition(kind).map(|_| kind)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_DIR_REGISTRAR)]
pub(crate) unsafe fn kirby_copy_dir_registrar(
    ctx: u64,
    color: u32,
    kind: i32,
    x3: u64,
    x4: u64,
) -> u64 {
    let n = KIRBY_REG_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if let Some(definition) = clone_definition(kind) {
        if n < KIRBY_REG_LOG_LIMIT {
            dbg_log!(
                "[kirbyreg] #{n} REG kind={kind}: base={} + custom namespace={} color={color}",
                definition.base_kind,
                definition.resource_name
            );
        }
        let base_ret = call_original!(ctx, color, definition.base_kind, x3, x4);
        with_kirby_copy_context(kind, || call_original!(ctx, color, kind, x3, x4));
        if n < KIRBY_REG_LOG_LIMIT {
            dbg_log!("[kirbyreg] #{n} RET clone kind={kind} color={color}");
        }
        return base_ret;
    }
    if n < KIRBY_REG_LOG_LIMIT {
        dbg_log!("[kirbyreg] #{n} REG kind={kind} color={color} passthrough");
    }
    let ret = call_original!(ctx, color, kind, x3, x4);
    if n < KIRBY_REG_LOG_LIMIT {
        dbg_log!("[kirbyreg] #{n} RET kind={kind} color={color}");
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_KIRBY_COPY_DIR_REGISTRAR_PARENT)]
pub(crate) unsafe fn kirby_copy_dir_registrar_parent(
    ctx: u64,
    record: u64,
    fighter_kind: i32,
    color: u32,
) -> u64 {
    let n = KIRBY_REG_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < KIRBY_REG_LOG_LIMIT {
        dbg_log!("[kirbyreg] #{n} PARENT fighter_kind={fighter_kind} color={color}");
    }
    let ret = call_original!(ctx, record, fighter_kind, color);
    if n < KIRBY_REG_LOG_LIMIT {
        dbg_log!("[kirbyreg] #{n} PARENT RET fighter_kind={fighter_kind} color={color}");
    }
    ret
}

#[cfg(feature = "css_slot")]
pub(crate) static WPN_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static WPN_SLOT_NEG_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) fn wpn_log_index() -> Option<u32> {
    let n = WPN_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let bracket_active = unsafe { active_construction_kind().is_some() };
    if n < 2000 && (bracket_active || n < 64) {
        Some(n)
    } else {
        None
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_WEAPON_PRELOAD)]
pub(crate) unsafe fn weapon_preload_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    const FIGHTER_KIND_KIRBY: i32 = 6;
    let idx = wpn_log_index();
    let dynamic = if x1 as i32 == FIGHTER_KIND_KIRBY {
        custom_articles::kirby_copy_dynamic_preload_header(clone_kind_in_match)
    } else {
        None
    };

    if let Some(n) = idx {
        dbg_log!(
            "[wpn] #{n} preload enter fighter={:#x} color={:#x} table={x3:#x} dynamic={dynamic:?} ctor={:?}",
            x1 as u32,
            x2 as u32,
            active_construction_kind()
        );
    }

    let base_ret = call_original!(x0, x1, x2, x3, x4, x5, x6, x7);
    if let Some((header, descriptor_count)) = dynamic {
        for color in 0..8u64 {
            dbg_log!("[copyarticle] preload color={color} enter dynamic={descriptor_count}");
            let ret = call_original!(x0, x1, color, header as u64, x4, x5, x6, x7);
            dbg_log!("[copyarticle] preload color={color} exit ret={ret:#x}");
        }
        dbg_log!("[copyarticle] preload colors=0..7 dynamic={descriptor_count} header={header:#x}");
    }

    if let Some(n) = idx {
        dbg_log!(
            "[wpn] #{n} preload exit fighter={:#x} ret={base_ret:#x}",
            x1 as u32
        );
    }
    base_ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_WEAPON_LOOP_CONT_A, inline)]
pub(crate) unsafe fn weapon_loop_cont_a(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = wpn_log_index() else { return };
    let weapon = ctx.registers[22].x() as u32;
    let record = ctx.registers[24].x() as *const u8;
    let kind = if record.is_null() {
        -2
    } else {
        core::ptr::read_unaligned(record.add(0x58) as *const i32)
    };
    dbg_log!("[wpn] #{n} cont-a weapon={weapon:#x} fighter_kind={kind}");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_WEAPON_LOOP_CONT_B, inline)]
pub(crate) unsafe fn weapon_loop_cont_b(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = wpn_log_index() else { return };
    let weapon = ctx.registers[22].x() as u32;
    let fp = ctx.registers[29].x();
    let handle = if fp == 0 {
        u32::MAX
    } else {
        core::ptr::read_unaligned((fp - 0xb0) as *const u32)
    };
    dbg_log!("[wpn] #{n} cont-b weapon={weapon:#x} handle={handle:#x}");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_WEAPON_LOOP_SLOT_ARGS, inline)]
pub(crate) unsafe fn weapon_loop_slot_args(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = wpn_log_index() else { return };
    let w2 = ctx.registers[2].x() as u32;
    let w3 = ctx.registers[3].x() as u32;
    let w4 = ctx.registers[4].x() as u32;
    let w5 = ctx.registers[25].x() as u32;
    dbg_log!("[wpn] #{n} slot-args w2={w2:#x} w3={w3:#x} w4={w4:#x} w5={w5:#x}");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_WEAPON_LOOP_RESOLVED, inline)]
pub(crate) unsafe fn weapon_loop_resolved(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(n) = wpn_log_index() else { return };
    let desc = ctx.registers[0].x();
    let kind = ctx.registers[28].x() as u32;
    dbg_log!("[wpn] #{n} resolved kind={kind} desc={desc:#x}");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_RESOURCE_SLOT)]
pub(crate) unsafe fn resource_slot_probe(
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
) -> u64 {
    let _walker = KIRBY_SLOT_WALKER_ACTIVE.enter(current_thread_key());
    let kind = x2 as u32 as i32;
    let bridged = clone_definition(kind).map(|definition| definition.base_kind);
    let idx = if x5 as u32 == u32::MAX || bridged.is_some() {
        let m = WPN_SLOT_NEG_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if m < 64 {
            Some(WPN_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
        } else {
            None
        }
    } else {
        wpn_log_index()
    };
    let effective_x2 = match bridged {
        Some(base_kind) => base_kind as u32 as u64,
        None => x2,
    };
    if let Some(n) = idx {
        dbg_log!(
            "[wpn] #{n} slot enter w1={:#x} w2={:#x}->{:#x} w3={:#x} w4={:#x} w5={:#x}",
            x1 as u32,
            x2 as u32,
            effective_x2 as u32,
            x3 as u32,
            x4 as u32,
            x5 as u32
        );
    }
    let thread = current_thread_key();
    let previous = bridged.map(|_| {
        (
            CLONE_SLOT_REGISTRATION_THREAD.swap(thread, core::sync::atomic::Ordering::AcqRel),
            CLONE_SLOT_REGISTRATION_KIND.swap(kind, core::sync::atomic::Ordering::AcqRel),
        )
    });
    let ret = call_original!(x0, x1, effective_x2, x3, x4, x5, x6, x7);
    if let Some((thread, kind)) = previous {
        CLONE_SLOT_REGISTRATION_KIND.store(kind, core::sync::atomic::Ordering::Release);
        CLONE_SLOT_REGISTRATION_THREAD.store(thread, core::sync::atomic::Ordering::Release);
    }
    if let Some(n) = idx {
        dbg_log!("[wpn] #{n} slot exit ret={ret:#x}");
    }
    ret
}

#[cfg(all(feature = "css_slot", feature = "diag_kirby_copy"))]
pub(crate) fn install_kirby_copy_diagnostics() {
    skyline::install_hooks!(
        kirby_copy_model_changer_entry_probe,
        kirby_copy_model_remove_result_probe,
        kirby_copy_resource_slot_0_probe,
        kirby_copy_resource_slot_1_probe,
        kirby_copy_resource_slot_2_probe,
        kirby_copy_hat_sync_probe,
        kirby_copy_setup_shim_probe,
        copy_setup_gate1_probe,
        copy_setup_slot_probe,
        copy_setup_dispatch_probe,
        copy_setup_grant_probe,
        sv_battle_object_kind_probe,
        kirby_get_copy_kind_probe,
        kirby_get_copy_slot_no_probe,
        kirby_copy_ability_reset_probe,
        kirby_copy_handle_probe,
        kirby_copy_bodymotion_handle_probe,
        kirby_copy_row_search_probe,
        kirby_copy_hat_record_region_probe,
        kirby_copy_base_model_pair,
        weapon_loop_cont_a,
        weapon_loop_cont_b,
        weapon_loop_slot_args,
        weapon_loop_resolved
    );
    skyline::println!("[clone_engine] installed 24 observation-only Kirby-copy breadcrumbs");
}

#[cfg(all(feature = "css_slot", not(feature = "diag_kirby_copy")))]
pub(crate) fn install_kirby_copy_diagnostics() {}

#[cfg(feature = "css_slot")]
pub(crate) fn install_custom_kirby_copy_hooks() {
    install_kirby_copy_diagnostics();
    #[cfg(feature = "native_table_backend")]
    skyline::install_hook!(kirby_copy_model_name_table);
    skyline::install_hooks!(
        kirby_copy_dir_registrar,
        kirby_copy_dir_registrar_parent,
        kirby_copy_record_creator_probe,
        kirby_copy_member_builder_probe,
        kirby_copy_record_name,
        kirby_copy_record_body_name,
        kirby_copy_record_sound_name
    );
    skyline::install_hooks!(
        kirby_copy_dir_name_merge_probe,
        kirby_copy_visual_kind_promote,
        kirby_copy_full_model_gate,
        kirby_copy_full_model_remove_gate,
        kirby_copy_special_install_probe,
        kirby_copy_resource_transfer,
        kirby_copy_transfer_lookup,
        kirby_copy_record_lookup_kind
    );
    skyline::install_hooks!(weapon_preload_probe, resource_slot_probe);
    skyline::install_hooks!(kirby_copy_setup_probe);
    skyline::install_hooks!(kirby_per_fighter_frame_probe);
    skyline::install_hooks!(
        copy_chara_table_impl_1,
        copy_chara_table_impl_2,
        copy_chara_table_impl_3,
        copy_chara_table_thunk_1,
        copy_chara_table_thunk_2,
        copy_chara_table_thunk_3,
        copy_chara_table_thunk_4,
        copy_chara_table_thunk_5,
        copy_chara_table_thunk_6,
        copy_chara_table_thunk_7,
        copy_chara_table_thunk_8,
        copy_motion_bind_tail_kind
    );
    skyline::install_hooks!(
        kirby_copy_static_setup_table,
        kirby_copy_article_header_assign,
        kirby_copy_callback_kind_1,
        kirby_copy_callback_kind_2,
        kirby_copy_callback_kind_3,
        kirby_copy_resource_kind,
        kirby_copy_model_name,
        kirby_copy_model_base_kind,
        kirby_copy_kind_list_table,
        kirby_copy_second_name_merge,
        kirby_copy_callback_kind_4,
        kirby_copy_removal_hashlist_a,
        kirby_copy_removal_hashlist_b
    );
}
