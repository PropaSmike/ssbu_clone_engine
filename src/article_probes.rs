use super::*;

#[cfg(any(feature = "diag_article", feature = "css_slot"))]
pub(crate) static ARTICLE_LOG_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) const INVALID_BATTLE_OBJECT_ID: u32 = 0x5000_0000;
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_OBJECT_ID: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(INVALID_BATTLE_OBJECT_ID);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_TRACE_REMAINING: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_LIFECYCLE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_POLL_STATE: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(u64::MAX);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_POLL_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_OPERATION_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_STATE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_COPY_ARTICLE_CREATOR_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) const KIRBY_COPY_CHARGE_SHOT_ARTICLE_ID: u32 = 0;

#[cfg(feature = "css_slot")]
#[derive(Clone, Copy)]
pub(crate) struct ArticleCreatorRegistryState {
    owner: u64,
    base_manager: u64,
    base_header: u64,
    base_table: u64,
    base_count: u32,
    base_loaded: u32,
    base_entry: u64,
    base_entry_words: [u64; 4],
    custom_manager: u64,
    custom_direct_header: u64,
    custom_direct_table: u64,
    custom_direct_count: u32,
    custom_fallback_object: u64,
    custom_fallback_header: u64,
    custom_fallback_count: u32,
    custom_entry: u64,
    custom_entry_words: [u64; 4],
    custom_runtime: u64,
    custom_runtime_vtable: u64,
    custom_runtime_slot0: u64,
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn read_article_creator_registry_state(
    module: u64,
    id: u32,
) -> ArticleCreatorRegistryState {
    let owner = *((module as usize + 0x8) as *const u64);

    let base_manager = *((module as usize + 0x240) as *const u64);
    let base_loaded = if base_manager != 0 {
        *((base_manager as usize + 0x220) as *const u32)
    } else {
        0
    };
    let base_header = if base_manager != 0 {
        *((base_manager as usize + 0x228) as *const u64)
    } else {
        0
    };
    let (base_table, base_count) = if base_header != 0 {
        (
            *(base_header as *const u64),
            *((base_header as usize + 0x8) as *const u32),
        )
    } else {
        (0, 0)
    };
    let base_entry = if base_table != 0 && id < base_count {
        base_table + id as u64 * 0x20
    } else {
        0
    };
    let base_entry_words = if base_entry != 0 {
        [
            *(base_entry as *const u64),
            *((base_entry as usize + 0x8) as *const u64),
            *((base_entry as usize + 0x10) as *const u64),
            *((base_entry as usize + 0x18) as *const u64),
        ]
    } else {
        [0; 4]
    };

    let custom_manager = *((module as usize + 0x248) as *const u64);
    let custom_direct_header = if custom_manager != 0 {
        *((custom_manager as usize + 0x260) as *const u64)
    } else {
        0
    };
    let (custom_direct_table, custom_direct_count) = if custom_direct_header != 0 {
        (
            *(custom_direct_header as *const u64),
            *((custom_direct_header as usize + 0x8) as *const u32),
        )
    } else {
        (0, 0)
    };
    let custom_fallback_object = if custom_manager != 0 {
        *((custom_manager as usize + 0x238) as *const u64)
    } else {
        0
    };
    let custom_fallback_header = if custom_fallback_object != 0 {
        *((custom_fallback_object as usize + 0x8) as *const u64)
    } else {
        0
    };
    let custom_fallback_count = if custom_fallback_header != 0 {
        *((custom_fallback_header as usize + 0x8) as *const u32)
    } else {
        0
    };
    let custom_entry = if custom_direct_table != 0 && id < custom_direct_count {
        custom_direct_table + id as u64 * 0x20
    } else {
        0
    };
    let custom_entry_words = if custom_entry != 0 {
        [
            *(custom_entry as *const u64),
            *((custom_entry as usize + 0x8) as *const u64),
            *((custom_entry as usize + 0x10) as *const u64),
            *((custom_entry as usize + 0x18) as *const u64),
        ]
    } else {
        [0; 4]
    };
    let custom_runtime = if custom_manager != 0 {
        *((custom_manager as usize + 0x248) as *const u64)
    } else {
        0
    };
    let custom_runtime_vtable = if custom_runtime != 0 {
        *(custom_runtime as *const u64)
    } else {
        0
    };
    let custom_runtime_slot0 = if custom_runtime_vtable != 0 {
        *(custom_runtime_vtable as *const u64)
    } else {
        0
    };

    ArticleCreatorRegistryState {
        owner,
        base_manager,
        base_header,
        base_table,
        base_count,
        base_loaded,
        base_entry,
        base_entry_words,
        custom_manager,
        custom_direct_header,
        custom_direct_table,
        custom_direct_count,
        custom_fallback_object,
        custom_fallback_header,
        custom_fallback_count,
        custom_entry,
        custom_entry_words,
        custom_runtime,
        custom_runtime_vtable,
        custom_runtime_slot0,
    }
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) unsafe fn tracked_kirby_article_module(module: u64) -> bool {
    if module == 0 {
        return false;
    }
    let owner = *((module as usize + 0x8) as *const u64);
    tracked_kirby_article_owner(owner)
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn tracked_kirby_article_owner(owner: u64) -> bool {
    owner != 0 && KIRBY_CLONE_COPY_BOMA.load(core::sync::atomic::Ordering::Acquire) == owner
}

#[cfg(feature = "css_slot")]
#[repr(C)]
pub(crate) struct ArticleView {
    vtable: u64,
    module_accessor: u64,
    generate_id: i32,
    _padding: i32,
}

#[cfg(feature = "css_slot")]
#[derive(Clone, Copy)]
pub(crate) struct KirbyArticleSlotState {
    module: u64,
    vtable: u64,
    generatable: bool,
    exists: bool,
    active_num: u32,
    article: u64,
    article_vtable: u64,
    article_boma: u64,
    generate_id: i32,
    weapon_object_id: u32,
    active: bool,
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn read_kirby_article_slot_state(
    boma: *mut u8,
    id: u32,
) -> Option<KirbyArticleSlotState> {
    if boma.is_null() {
        return None;
    }
    let module = *((boma as usize + ARTICLE_MODULE_OFF) as *const u64);
    if module == 0 {
        return None;
    }
    let vtable = *(module as *const u64);
    if vtable == 0 {
        return None;
    }

    let is_exist: extern "C" fn(u64, u32) -> bool =
        core::mem::transmute(*((vtable as usize + 0x1c8) as *const u64));
    let is_generatable: extern "C" fn(u64, u32) -> bool =
        core::mem::transmute(*((vtable as usize + 0x1d8) as *const u64));
    let get_active_num: extern "C" fn(u64, u32) -> u64 =
        core::mem::transmute(*((vtable as usize + 0x1e0) as *const u64));
    let get_article: extern "C" fn(u64, u32) -> u64 =
        core::mem::transmute(*((vtable as usize + 0x208) as *const u64));

    let exists = is_exist(module, id);
    let generatable = is_generatable(module, id);
    let active_num = get_active_num(module, id) as u32;
    let article = get_article(module, id);
    let (article_vtable, article_boma, generate_id, weapon_object_id) = if article != 0 {
        let view = &*(article as *const ArticleView);
        let object_id = if view.module_accessor != 0 {
            *((view.module_accessor as usize + 0x8) as *const u32)
        } else {
            INVALID_BATTLE_OBJECT_ID
        };
        (
            view.vtable,
            view.module_accessor,
            view.generate_id,
            object_id,
        )
    } else {
        (0, 0, -1, INVALID_BATTLE_OBJECT_ID)
    };
    let active = if weapon_object_id == INVALID_BATTLE_OBJECT_ID {
        false
    } else {
        let is_active: extern "C" fn(u32) -> bool =
            core::mem::transmute(text_base() + OFF_SV_BATTLE_OBJECT_IS_ACTIVE);
        is_active(weapon_object_id)
    };

    Some(KirbyArticleSlotState {
        module,
        vtable,
        generatable,
        exists,
        active_num,
        article,
        article_vtable,
        article_boma,
        generate_id,
        weapon_object_id,
        active,
    })
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn arm_kirby_article_lifecycle(state: KirbyArticleSlotState) {
    if state.weapon_object_id != INVALID_BATTLE_OBJECT_ID {
        KIRBY_COPY_ARTICLE_OBJECT_ID.store(
            state.weapon_object_id,
            core::sync::atomic::Ordering::Release,
        );
        KIRBY_COPY_ARTICLE_TRACE_REMAINING.store(16, core::sync::atomic::Ordering::Release);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn poll_kirby_copy_article_state(boma: u64) {
    let Some(slot) =
        read_kirby_article_slot_state(boma as *mut u8, KIRBY_COPY_CHARGE_SHOT_ARTICLE_ID)
    else {
        return;
    };

    let tb = text_base();
    let status_kind: extern "C" fn(u64) -> i32 = core::mem::transmute(tb + OFF_STATUS_KIND_IMPL);
    let motion_kind: extern "C" fn(u64) -> u64 = core::mem::transmute(tb + OFF_MOTION_KIND_IMPL);
    let motion_frame: extern "C" fn(u64) -> f32 = core::mem::transmute(tb + OFF_MOTION_FRAME_IMPL);
    let status = status_kind(boma);
    let motion = motion_kind(boma);
    let frame = motion_frame(boma);
    let state = (status as u32 as u64 & 0xffff)
        | ((slot.exists as u64) << 16)
        | ((slot.generatable as u64) << 17)
        | (((slot.active_num.min(0xff)) as u64) << 18)
        | (((slot.article != 0) as u64) << 26)
        | ((slot.active as u64) << 27)
        | (((slot.generate_id as u32 as u64) & 0xff) << 28)
        | ((motion & 0x0fff_ffff) << 36);
    let previous = KIRBY_COPY_ARTICLE_POLL_STATE.swap(state, core::sync::atomic::Ordering::Relaxed);
    if previous == state {
        return;
    }
    arm_kirby_article_lifecycle(slot);
    let n = KIRBY_COPY_ARTICLE_POLL_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 96 {
        dbg_log!(
            "[kirbyarticle] poll #{n} boma={boma:#x} status={status:#x} motion={motion:#x} frame={frame:.1} id={KIRBY_COPY_CHARGE_SHOT_ARTICLE_ID} module={:#x} vtable={:#x} generatable={} exist={} active_num={} article={:#x} generate_id={} weapon_object={:#x} active={}",
            slot.module,
            slot.vtable,
            slot.generatable,
            slot.exists,
            slot.active_num,
            slot.article,
            slot.generate_id,
            slot.weapon_object_id,
            slot.active,
        );
    }
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn tracked_kirby_article_boma(boma: *mut u8) -> bool {
    !boma.is_null()
        && KIRBY_CLONE_COPY_BOMA.load(core::sync::atomic::Ordering::Acquire) == boma as u64
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn log_kirby_article_slot_state(phase: &str, boma: *mut u8, id: u32) {
    if !tracked_kirby_article_boma(boma) {
        return;
    }
    let n = KIRBY_COPY_ARTICLE_STATE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n >= 128 {
        return;
    }
    let tb = text_base();
    let status_kind: extern "C" fn(u64) -> i32 = core::mem::transmute(tb + OFF_STATUS_KIND_IMPL);
    let motion_kind: extern "C" fn(u64) -> u64 = core::mem::transmute(tb + OFF_MOTION_KIND_IMPL);
    let motion_frame: extern "C" fn(u64) -> f32 = core::mem::transmute(tb + OFF_MOTION_FRAME_IMPL);
    let status = status_kind(boma as u64);
    let motion = motion_kind(boma as u64);
    let frame = motion_frame(boma as u64);
    if let Some(slot) = read_kirby_article_slot_state(boma, id) {
        arm_kirby_article_lifecycle(slot);
        dbg_log!(
            "[kirbyarticle] state #{n} phase={phase} boma={:#x} id={id} status={status:#x} motion={motion:#x} frame={frame:.1} module={:#x} vtable={:#x} generatable={} exist={} active_num={} article={:#x} article_vtable={:#x} article_boma={:#x} generate_id={} weapon_object={:#x} active={}",
            boma as usize,
            slot.module,
            slot.vtable,
            slot.generatable,
            slot.exists,
            slot.active_num,
            slot.article,
            slot.article_vtable,
            slot.article_boma,
            slot.generate_id,
            slot.weapon_object_id,
            slot.active,
        );
    } else {
        dbg_log!(
            "[kirbyarticle] state #{n} phase={phase} boma={:#x} id={id} status={status:#x} motion={motion:#x} frame={frame:.1} unavailable",
            boma as usize,
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn log_kirby_article_operation(
    operation: &str,
    boma: *mut u8,
    id: u32,
    arg2: u64,
    arg3: u64,
    ret: u64,
) {
    if !tracked_kirby_article_boma(boma) {
        return;
    }
    let n = KIRBY_COPY_ARTICLE_OPERATION_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 128 {
        dbg_log!(
            "[kirbyarticle] op #{n} {operation} boma={:#x} id={id} arg2={:#x} arg3={:#x} ret={ret:#x} snapshot={:?}",
            boma as usize,
            arg2 as u32,
            arg3 as u32,
            kirby_copy_work_snapshot(boma as u64)
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn trace_kirby_copy_article_lifecycle() {
    let remaining = KIRBY_COPY_ARTICLE_TRACE_REMAINING.load(core::sync::atomic::Ordering::Acquire);
    if remaining == 0 {
        return;
    }
    let object_id = KIRBY_COPY_ARTICLE_OBJECT_ID.load(core::sync::atomic::Ordering::Acquire);
    if object_id == INVALID_BATTLE_OBJECT_ID {
        KIRBY_COPY_ARTICLE_TRACE_REMAINING.store(0, core::sync::atomic::Ordering::Release);
        return;
    }

    let tb = text_base();
    let is_active: extern "C" fn(u32) -> bool =
        core::mem::transmute(tb + OFF_SV_BATTLE_OBJECT_IS_ACTIVE);
    let active = is_active(object_id);
    let (boma, category, kind, status, motion) = if active {
        let module_accessor: extern "C" fn(u32) -> u64 =
            core::mem::transmute(tb + OFF_SV_BATTLE_OBJECT_MODULE_ACCESSOR);
        let category_fn: extern "C" fn(u32) -> i32 =
            core::mem::transmute(tb + OFF_SV_BATTLE_OBJECT_CATEGORY);
        let kind_fn: extern "C" fn(u32) -> i32 =
            core::mem::transmute(tb + OFF_SV_BATTLE_OBJECT_KIND_FOR_ARTICLE);
        let boma = module_accessor(object_id);
        if boma != 0 {
            let status_kind: extern "C" fn(u64) -> i32 =
                core::mem::transmute(tb + OFF_STATUS_KIND_IMPL);
            let motion_kind: extern "C" fn(u64) -> u64 =
                core::mem::transmute(tb + OFF_MOTION_KIND_IMPL);
            (
                boma,
                category_fn(object_id),
                kind_fn(object_id),
                status_kind(boma),
                motion_kind(boma),
            )
        } else {
            (0, category_fn(object_id), kind_fn(object_id), -1, 0)
        }
    } else {
        (0, -1, -1, -1, 0)
    };
    let step = 16u32.saturating_sub(remaining);
    let n = KIRBY_COPY_ARTICLE_LIFECYCLE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbyarticle] lifecycle #{n} step={step} object={object_id:#x} active={active} boma={boma:#x} category={category} kind={kind:#x} status={status:#x} motion={motion:#x}"
        );
    }

    if !active || remaining <= 1 {
        KIRBY_COPY_ARTICLE_TRACE_REMAINING.store(0, core::sync::atomic::Ordering::Release);
        if !active {
            KIRBY_COPY_ARTICLE_OBJECT_ID.store(
                INVALID_BATTLE_OBJECT_ID,
                core::sync::atomic::Ordering::Release,
            );
        }
    } else {
        KIRBY_COPY_ARTICLE_TRACE_REMAINING
            .store(remaining - 1, core::sync::atomic::Ordering::Release);
    }
}

#[cfg(any(feature = "diag_article", feature = "css_slot"))]
#[skyline::hook(offset = OFF_GENERATE_ARTICLE_IMPL)]
pub(crate) unsafe fn generate_article_probe(boma: *mut u8, id: u32, x2: u64, x3: u64) -> u64 {
    #[cfg(feature = "css_slot")]
    let tracked = !boma.is_null()
        && KIRBY_CLONE_COPY_BOMA.load(core::sync::atomic::Ordering::Acquire) == boma as u64;

    #[cfg(feature = "css_slot")]
    let observe = tracked;
    #[cfg(all(feature = "diag_article", not(feature = "css_slot")))]
    let observe = true;

    let module = if !observe || boma.is_null() {
        0
    } else {
        *((boma as usize + ARTICLE_MODULE_OFF) as *const usize)
    };
    let vtable = if module != 0 {
        *(module as *const usize)
    } else {
        0
    };
    let obj_id = if !observe || boma.is_null() {
        u32::MAX
    } else {
        *((boma as usize + 0x8) as *const u32)
    };
    let ret = call_original!(boma, id, x2, x3);

    #[cfg(feature = "css_slot")]
    {
        static GEN_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = GEN_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 64 && !boma.is_null() {
            let module =
                core::ptr::read_volatile((boma as usize + ARTICLE_MODULE_OFF) as *const usize);
            let article = if module != 0 {
                let vtable = core::ptr::read_volatile(module as *const usize);
                if vtable != 0 {
                    let get_article: extern "C" fn(u64, u32) -> u64 = core::mem::transmute(
                        core::ptr::read_volatile((vtable + 0x208) as *const u64),
                    );
                    get_article(module as u64, id)
                } else {
                    0
                }
            } else {
                0
            };
            let weapon_kind = if article != 0 {
                let view = &*(article as *const ArticleView);
                if view.module_accessor != 0 {
                    core::ptr::read_volatile((view.module_accessor as usize + 0xc) as *const i32)
                } else {
                    -1
                }
            } else {
                -1
            };
            dbg_log!(
                "[genarticle] #{n} boma={:#x} id={id} -> article={article:#x} weapon_kind={weapon_kind}",
                boma as usize
            );
        }
    }

    #[cfg(feature = "css_slot")]
    if tracked {
        let n = ARTICLE_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 64 {
            let snapshot = kirby_copy_work_snapshot(boma as u64);
            let article = if module != 0 && vtable != 0 {
                let get_article: extern "C" fn(u64, u32) -> u64 =
                    core::mem::transmute(*((vtable + 0x208) as *const u64));
                get_article(module as u64, id)
            } else {
                0
            };
            let (article_vtable, article_boma, generate_id, weapon_object_id) = if article != 0 {
                let view = &*(article as *const ArticleView);
                let object_id = if view.module_accessor != 0 {
                    *((view.module_accessor as usize + 0x8) as *const u32)
                } else {
                    INVALID_BATTLE_OBJECT_ID
                };
                (
                    view.vtable,
                    view.module_accessor,
                    view.generate_id,
                    object_id,
                )
            } else {
                (0, 0, -1, INVALID_BATTLE_OBJECT_ID)
            };
            let active = if weapon_object_id == INVALID_BATTLE_OBJECT_ID {
                false
            } else {
                let is_active: extern "C" fn(u32) -> bool =
                    core::mem::transmute(text_base() + OFF_SV_BATTLE_OBJECT_IS_ACTIVE);
                is_active(weapon_object_id)
            };
            if weapon_object_id != INVALID_BATTLE_OBJECT_ID {
                KIRBY_COPY_ARTICLE_OBJECT_ID
                    .store(weapon_object_id, core::sync::atomic::Ordering::Release);
                KIRBY_COPY_ARTICLE_TRACE_REMAINING.store(16, core::sync::atomic::Ordering::Release);
            }
            dbg_log!(
                "[kirbyarticle] #{n} generate boma={:#x} obj={obj_id:#x} id={id} arg2={:#x} arg3={:#x} module={module:#x} vtable={vtable:#x} ret={ret:#x} article={article:#x} article_vtable={article_vtable:#x} article_boma={article_boma:#x} generate_id={generate_id} weapon_object={weapon_object_id:#x} active={active} snapshot={snapshot:?}",
                boma as usize,
                x2 as u32,
                x3 as u32,
            );
        }
    }

    #[cfg(all(feature = "diag_article", not(feature = "css_slot")))]
    {
        let n = ARTICLE_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 80 {
            skyline::println!(
                "[article_probe] #{n} boma={:#x} obj={:#x} id={} module={:#x} vtable={:#x} ret={:#x}",
                boma as usize,
                obj_id,
                id,
                module,
                vtable,
                ret
            );
        }
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_GENERATE_ARTICLE_ENABLE_IMPL)]
pub(crate) unsafe fn generate_article_enable_probe(
    boma: *mut u8,
    id: u32,
    x2: u64,
    x3: u64,
) -> u64 {
    log_kirby_article_slot_state("pre-generate_enable", boma, id);
    let ret = call_original!(boma, id, x2, x3);
    log_kirby_article_operation("generate_enable", boma, id, x2, x3, ret);
    log_kirby_article_slot_state("post-generate_enable", boma, id);
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_ARTICLE_CREATOR_DISPATCH)]
pub(crate) unsafe fn article_creator_dispatch_probe(
    module: u64,
    id: u32,
    owner_in: u64,
    enable: u32,
    argument: i32,
) -> u64 {
    if !tracked_kirby_article_module(module) {
        return call_original!(module, id, owner_in, enable, argument);
    }

    let state = read_article_creator_registry_state(module, id);
    let effective_custom_count = if state.custom_direct_header != 0 {
        state.custom_direct_count
    } else {
        state.custom_fallback_count
    };
    let route = if state.custom_manager != 0 && effective_custom_count > id {
        "custom"
    } else if state.base_count > id {
        "base"
    } else {
        "none"
    };
    let n = KIRBY_COPY_ARTICLE_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbycreator] #{n} site=dispatch-pre module={module:#x} id={id} owner_in={owner_in:#x} owner={:#x} enable={} argument={argument} route={route} base={:#x} loaded={} header={:#x} table={:#x} count={} entry={:#x} words=[{:#x},{:#x},{:#x},{:#x}] custom={:#x} direct_header={:#x} direct_table={:#x} direct_count={} fallback_obj={:#x} fallback_header={:#x} fallback_count={} entry={:#x} words=[{:#x},{:#x},{:#x},{:#x}] runtime={:#x} runtime_vtable={:#x} runtime_slot0={:#x}",
            state.owner,
            enable & 1,
            state.base_manager,
            state.base_loaded,
            state.base_header,
            state.base_table,
            state.base_count,
            state.base_entry,
            state.base_entry_words[0],
            state.base_entry_words[1],
            state.base_entry_words[2],
            state.base_entry_words[3],
            state.custom_manager,
            state.custom_direct_header,
            state.custom_direct_table,
            state.custom_direct_count,
            state.custom_fallback_object,
            state.custom_fallback_header,
            state.custom_fallback_count,
            state.custom_entry,
            state.custom_entry_words[0],
            state.custom_entry_words[1],
            state.custom_entry_words[2],
            state.custom_entry_words[3],
            state.custom_runtime,
            state.custom_runtime_vtable,
            state.custom_runtime_slot0,
        );
    }
    let ret = call_original!(module, id, owner_in, enable, argument);
    let n = KIRBY_COPY_ARTICLE_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbycreator] #{n} site=dispatch-post module={module:#x} id={id} route={route} article={ret:#x}"
        );
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_ARTICLE_CUSTOM_CREATOR)]
pub(crate) unsafe fn article_custom_creator_probe(
    manager: u64,
    id: u32,
    owner: u64,
    argument: u32,
) -> u64 {
    if !tracked_kirby_article_owner(owner) {
        return call_original!(manager, id, owner, argument);
    }
    let n = KIRBY_COPY_ARTICLE_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbycreator] #{n} site=custom-pre manager={manager:#x} id={id} owner={owner:#x} argument={argument}"
        );
    }
    let ret = call_original!(manager, id, owner, argument);
    let n = KIRBY_COPY_ARTICLE_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbycreator] #{n} site=custom-post manager={manager:#x} id={id} article={ret:#x}"
        );
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_ARTICLE_BASE_CREATOR)]
pub(crate) unsafe fn article_base_creator_probe(
    manager: u64,
    id: u32,
    owner: u64,
    argument: u32,
) -> u64 {
    if !tracked_kirby_article_owner(owner) {
        return call_original!(manager, id, owner, argument);
    }
    let n = KIRBY_COPY_ARTICLE_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbycreator] #{n} site=base-pre manager={manager:#x} id={id} owner={owner:#x} argument={argument}"
        );
    }
    let ret = call_original!(manager, id, owner, argument);
    let n = KIRBY_COPY_ARTICLE_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[kirbycreator] #{n} site=base-post manager={manager:#x} id={id} article={ret:#x}"
        );
    }
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_SHOOT_ARTICLE_IMPL)]
pub(crate) unsafe fn shoot_article_probe(boma: *mut u8, id: u32, target: u64, arg3: u64) -> u64 {
    log_kirby_article_slot_state("pre-shoot", boma, id);
    let ret = call_original!(boma, id, target, arg3);
    log_kirby_article_operation("shoot", boma, id, target, arg3, ret);
    log_kirby_article_slot_state("post-shoot", boma, id);
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_SHOOT_EXIST_ARTICLE_IMPL)]
pub(crate) unsafe fn shoot_exist_article_probe(
    boma: *mut u8,
    id: u32,
    target: u64,
    arg3: u64,
) -> u64 {
    log_kirby_article_slot_state("pre-shoot_exist", boma, id);
    let ret = call_original!(boma, id, target, arg3);
    log_kirby_article_operation("shoot_exist", boma, id, target, arg3, ret);
    log_kirby_article_slot_state("post-shoot_exist", boma, id);
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_REMOVE_ARTICLE_IMPL)]
pub(crate) unsafe fn remove_article_probe(boma: *mut u8, id: u32, target: u64) -> u64 {
    log_kirby_article_slot_state("pre-remove", boma, id);
    let ret = call_original!(boma, id, target);
    log_kirby_article_operation("remove", boma, id, target, 0, ret);
    log_kirby_article_slot_state("post-remove", boma, id);
    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_REMOVE_EXIST_ARTICLE_IMPL)]
pub(crate) unsafe fn remove_exist_article_probe(boma: *mut u8, id: u32, target: u64) -> u64 {
    log_kirby_article_slot_state("pre-remove_exist", boma, id);
    let ret = call_original!(boma, id, target);
    log_kirby_article_operation("remove_exist", boma, id, target, 0, ret);
    log_kirby_article_slot_state("post-remove_exist", boma, id);
    ret
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_INIT_PROBE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const NULL_PAGE: u64 = 0x1000;

#[cfg(feature = "css_slot")]
const KIRBY_ARTICLE_KIND_TABLE: usize = 0x4fcd098;

#[cfg(feature = "css_slot")]
const KIRBY_ARTICLE_KIND_FIELD: u64 = 0x17398;

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0xba3e24, inline)]
pub(crate) unsafe fn kirby_article_init_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let n = KIRBY_INIT_PROBE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n >= 16 {
        return;
    }
    let read = |address: u64| -> u64 {
        if address < NULL_PAGE {
            0
        } else {
            core::ptr::read_volatile(address as *const u64)
        }
    };
    let read32 = |address: u64| -> u32 {
        if address < NULL_PAGE {
            0
        } else {
            core::ptr::read_volatile(address as *const u32)
        }
    };

    let x0 = ctx.registers[0].x();
    let x1 = ctx.registers[1].x();
    let descriptor = ctx.registers[8].x();
    let descriptors = ctx.registers[9].x();
    let count = ctx.registers[10].w();
    let index = read32(x1 + 0x10) as i32;

    let kind = if x0 == 0 {
        -1
    } else {
        read32(x0 + KIRBY_ARTICLE_KIND_FIELD) as i32
    };
    let slot = if (0..0x100).contains(&kind) {
        (crate::text_base() + KIRBY_ARTICLE_KIND_TABLE) as u64 + (kind as u64) * 8
    } else {
        0
    };
    let entry = read(slot);

    let copy_kind =
        crate::kirby_copy::KIRBY_CLONE_COPY_KIND.load(core::sync::atomic::Ordering::Acquire);
    let (published, published_count) =
        custom_articles::kirby_copy_header(copy_kind).unwrap_or((0, 0));
    let ours_descriptors = read(published as u64);
    let ours_descriptor = if ours_descriptors == 0 || index < 0 || index as usize >= published_count
    {
        0
    } else {
        ours_descriptors + (index as u64) * 0x20
    };

    dbg_log!(
        "[kirbyinit] #{n} x0={x0:#x} kind={kind} | x1={x1:#x} index={index} \
         walked_descriptors={descriptors:#x} walked_count={count} descriptor={descriptor:#x} \
         weapon_id={} max_count={} on_init={:#x} on_fini={:#x} \
         | published={published:#x} published_count={published_count} \
         published_descriptors={ours_descriptors:#x} ours={} \
         ours_descriptor={ours_descriptor:#x} ours_weapon_id={} ours_on_init={:#x} \
         ours_on_fini={:#x} \
         | UNRELIABLE(assumes contiguous module) slot={slot:#x} entry={entry:#x}",
        read32(descriptor) as i32,
        read32(descriptor + 4) as i32,
        read(descriptor + 8),
        read(descriptor + 0x10),
        published != 0 && descriptors == ours_descriptors,
        read32(ours_descriptor) as i32,
        read(ours_descriptor + 8),
        read(ours_descriptor + 0x10),
    );
}

#[cfg(feature = "css_slot")]
pub(crate) static KIRBY_INIT_GUARD_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const KIRBY_ARTICLE_INDEX_FIELD: u64 = 0x10;

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0xba3e2c, inline)]
pub(crate) unsafe fn kirby_article_init_guard(ctx: &mut skyline::hooks::InlineCtx) {
    if ctx.registers[3].x() != 0 {
        return;
    }
    let read = |address: u64| -> u64 {
        if address < NULL_PAGE {
            0
        } else {
            core::ptr::read_volatile(address as *const u64)
        }
    };

    let article = ctx.registers[0].x();
    let index = if article == 0 {
        -1
    } else {
        core::ptr::read_volatile((article + KIRBY_ARTICLE_INDEX_FIELD) as *const u32) as i32
    };

    let copy_kind =
        crate::kirby_copy::KIRBY_CLONE_COPY_KIND.load(core::sync::atomic::Ordering::Acquire);
    let (published, published_count) =
        custom_articles::kirby_copy_header(copy_kind).unwrap_or((0, 0));
    let descriptors = read(published as u64);

    let served = if descriptors != 0 && index >= 0 && (index as usize) < published_count {
        read(descriptors + (index as u64) * 0x20 + 8)
    } else {
        0
    };

    let n = KIRBY_INIT_GUARD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if served != 0 {
        ctx.registers[3].set_x(served);
        if n < 8 {
            dbg_log!(
                "[kirbyinit] #{n} SERVED copy-header on_init={served:#x} for index={index} \
                 copy_kind={copy_kind} weapon_id={} (native table gave null)",
                core::ptr::read_volatile((descriptors + (index as u64) * 0x20) as *const u32)
                    as i32,
            );
        }
        return;
    }

    ctx.registers[3].set_x((crate::text_base() + 0xba3e34) as u64);
    if n < 8 {
        dbg_log!(
            "[kirbyinit] #{n} GUARDED null on_init_callback and the copy header could not \
             supply one (index={index} copy_kind={copy_kind} published={published:#x} \
             count={published_count}); returning no article"
        );
    }
}
