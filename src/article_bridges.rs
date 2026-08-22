use super::*;

macro_rules! custom_article_name_hooks {
    ($install:ident, $lookup:path; $($name:ident($src:expr, $dst:expr, $offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[$src].x() as i32;
                if let Some(value) = $lookup(weapon_kind) {
                    ctx.registers[$dst].set_x(value.as_ptr() as u64);
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

macro_rules! custom_article_scaled_name_hooks {
    ($install:ident, $lookup:path; $($name:ident($src:expr, $dst:expr, $offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = (ctx.registers[$src].x() >> 3) as i32;
                if let Some(value) = $lookup(weapon_kind) {
                    ctx.registers[$dst].set_x(value.as_ptr() as u64);
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_scaled_name_hooks! {
    install_custom_article_scaled_weapon_name_hooks, custom_articles::source_weapon_name;
    custom_weapon_name_game_acmd(8, 3, 0x33ace8c);
    custom_weapon_name_sound_acmd(8, 3, 0x33aed4c);
    custom_weapon_name_effect_acmd(8, 3, 0x33addec);
    custom_weapon_name_status(8, 3, 0x33abf64);
}

custom_article_scaled_name_hooks! {
    install_custom_article_scaled_owner_name_hooks, custom_articles::source_weapon_owner_name;
    custom_weapon_owner_name_game_acmd(8, 2, 0x33ace7c);
    custom_weapon_owner_name_sound_acmd(8, 2, 0x33aed3c);
    custom_weapon_owner_name_effect_acmd(8, 2, 0x33adddc);
    custom_weapon_owner_name_status(8, 2, 0x33abf54);
}

macro_rules! custom_article_agent_gate_hooks {
    ($install:ident; $($name:ident($offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[22].x() as i32;
                if custom_articles::is_custom_weapon_kind(weapon_kind) {
                    ctx.registers[23].set_x(0);
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_agent_gate_hooks! {
    install_custom_article_agent_gate_hooks;
    custom_weapon_agent_gate_status(0x33abf24);
    custom_weapon_agent_gate_game_acmd(0x33ace2c);
    custom_weapon_agent_gate_sound_acmd(0x33aecec);
    custom_weapon_agent_gate_effect_acmd(0x33add8c);
}

custom_article_name_hooks! {
    install_custom_article_weapon_name_hooks, custom_articles::custom_weapon_name;
    custom_weapon_name_param(21, 27, 0x33b6830);
    custom_weapon_name_map_collision(21, 2, 0x33b69f0);
    custom_weapon_name_visibility(21, 2, 0x33b6d14);
    custom_weapon_name_visibility_data(21, 2, 0x33b6c80);
}

macro_rules! custom_article_owner_kind_hooks {
    ($install:ident, $lookup:path; $($name:ident($src:expr, $dst:expr, $offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[$src].x() as i32;
                if let Some(owner) = $lookup(weapon_kind) {
                    ctx.registers[$dst].set_x(owner as u64);
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_owner_kind_hooks! {
    install_custom_article_owner_kind_hooks, custom_articles::custom_weapon_owner_kind;
    custom_weapon_owner_kind_param(21, 26, 0x33b6628);
}

custom_article_owner_kind_hooks! {
    install_custom_article_creator_owner_kind_hooks, custom_articles::source_weapon_owner_kind;
    custom_weapon_owner_kind_game_acmd(22, 8, 0x33acf78);
    custom_weapon_owner_kind_sound_acmd(22, 8, 0x33aee38);
    custom_weapon_owner_kind_effect_acmd(22, 8, 0x33aded8);
    custom_weapon_owner_kind_status(22, 8, 0x33ac040);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33b5f44, inline)]
pub(crate) unsafe fn custom_article_weapon_record_base(ctx: &mut skyline::hooks::InlineCtx) {
    const WEAPON_RECORD_STRIDE: i64 = 0xe8;

    let weapon_kind = ctx.registers[24].x() as i32;

    {
        static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 48 {
            dbg_log!("[wpnctor] #{n} kind={weapon_kind}");
        }
        if custom_articles::custom_weapon_source_kind(weapon_kind).is_some()
            || custom_articles::source_weapon_owner_kind(weapon_kind).is_some()
        {
            static REGS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
            let r = REGS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if r < 8 {
                let mut hits: Vec<String> = Vec::new();
                for index in 0..31usize {
                    let value = ctx.registers[index].x() as usize;
                    if let Some(entry) = crate::entry_of_fighter_boma_public(value) {
                        hits.push(format!("x{index}=>entry{entry}"));
                    }
                }
                let stack = crate::scan_stack_for_owner(ctx.sp.x() as usize, 0x400);
                dbg_log!(
                    "[wpnscan] #{r} kind={weapon_kind} regs[{}] stack[{stack}]",
                    if hits.is_empty() { "none".to_string() } else { hits.join(",") }
                );
            }
        }
    }

    let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
        return;
    };

    if let Some(owner) = custom_articles::source_weapon_owner_kind(weapon_kind) {
        fighter_modules::ensure_loaded(owner, 3000);
    }

    let bias = (source as i64 - weapon_kind as i64) * WEAPON_RECORD_STRIDE;
    let base = ctx.registers[25].x();
    ctx.registers[25].set_x(base.wrapping_add(bias as u64));

    static RECORD_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let n = RECORD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 16 {
        dbg_log!(
            "[articlerecord] #{n} weapon kind {weapon_kind} uses source {source}'s record              (base {base:#x} biased by {bias})"
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17e09a8, inline)]
pub(crate) unsafe fn custom_article_owner_category(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[26].x() as i32;
    if let Some(category) = custom_articles::custom_weapon_owner_category(weapon_kind) {
        ctx.registers[25].set_x(category as u64);
    }
}

macro_rules! custom_article_source_kind_sites {
    ($install:ident; $($name:ident($reg:expr, $offset:expr, $what:expr));* $(;)?) => {
        $(
            #[doc = $what]
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let kind = ctx.registers[$reg].x() as i32;
                if let Some(source) = custom_articles::custom_weapon_source_kind(kind) {
                    ctx.registers[$reg].set_x(source as u64);
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_source_kind_sites! {
    install_custom_article_source_kind_sites;
    custom_article_kind_spec(0, 0x33be790, "spec singleton 0x529c400");
    custom_article_kind_table_452a(0, 0x33aa1e0, "per-kind table 0x452abd8");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17e0890, inline)]
pub(crate) unsafe fn custom_article_path_weapon_name(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[23].x() as i32;
    if let Some(base) = custom_articles::weapon_name_table_bias(weapon_kind) {
        ctx.registers[9].set_x(base);
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x48aba4, inline)]
pub(crate) unsafe fn module_190_factory(ctx: &mut skyline::hooks::InlineCtx) {
    static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    if SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed) >= 64 {
        return;
    }
    let object = ctx.registers[0].x();
    let vtable = ctx.registers[8].x();
    let method = if vtable == 0 {
        0
    } else {
        core::ptr::read_volatile((vtable + 0x30) as *const u64)
    };
    let target = if object == 0 {
        0
    } else {
        core::ptr::read_volatile((object + 8) as *const u64)
    };
    dbg_log!(
        "[mod190] factory obj={object:#x} vt={vtable:#x} method={method:#x} rel={:#x} target={target:#x} target_rel={:#x} x9={:#x} x10={:#x}",
        method.wrapping_sub(text_base() as u64),
        target.wrapping_sub(text_base() as u64),
        ctx.registers[9].x(),
        ctx.registers[10].x()
    );
}

#[cfg(feature = "css_slot")]
pub(crate) static SPOOFED_ARTICLE_KIND: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn article_object_kind_field(setup: u64) -> Option<*mut i32> {
    if setup == 0 {
        return None;
    }
    let object = core::ptr::read_volatile(setup as *const u64);
    if object == 0 {
        return None;
    }
    Some((object + 0xc) as *mut i32)
}

#[cfg(feature = "css_slot")]
#[allow(dead_code)]
#[skyline::hook(offset = 0x48aba4, inline)]
pub(crate) unsafe fn article_agent_kind_spoof_enter(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(field) = article_object_kind_field(ctx.registers[20].x()) else {
        return;
    };
    let kind = core::ptr::read_volatile(field);
    let Some(source) = custom_articles::custom_weapon_source_kind(kind) else {
        return;
    };
    core::ptr::write_volatile(field, source);
    SPOOFED_ARTICLE_KIND.store(kind, core::sync::atomic::Ordering::Relaxed);

    static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let n = SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        dbg_log!("[agentspoof] #{n} object kind {kind} -> {source} for agent creation");
    }
}

#[cfg(feature = "css_slot")]
#[allow(dead_code)]
#[skyline::hook(offset = 0x48abb0, inline)]
pub(crate) unsafe fn article_agent_kind_spoof_leave(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = SPOOFED_ARTICLE_KIND.swap(-1, core::sync::atomic::Ordering::Relaxed);
    if kind < 0 {
        return;
    }
    if let Some(field) = article_object_kind_field(ctx.registers[20].x()) {
        core::ptr::write_volatile(field, kind);
    }
    dbg_log!(
        "[agentspoof] restored object kind {kind}; agent={:#x}",
        ctx.registers[0].x()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33abee0)]
pub(crate) unsafe fn article_status_agent_create(
    object: *mut u8,
    boma: *mut u8,
    lua_state: *mut u8,
) -> *mut u8 {
    let kind = if object.is_null() {
        -1
    } else {
        core::ptr::read_volatile(object.add(0xc) as *const i32)
    };
    let minted = custom_articles::custom_weapon_source_kind(kind).is_some();
    if !minted {
        return call_original!(object, boma, lua_state);
    }

    {
        static MAPPED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
        if !MAPPED.swap(true, core::sync::atomic::Ordering::Relaxed) {
            let smashline = lookup_symbol(b"smashline_install_status_script\0").unwrap_or(0);
            let l2c_common =
                lookup_symbol(b"_ZN7lua2cpp15L2CWeaponCommon26sub_weapon_common_settingsEv\0")
                    .unwrap_or(0);
            let l2c_agent = lookup_symbol(
                b"_ZN7lua2cpp12L2CAgentBase18sv_set_status_funcEN3lib8L2CValueES2_PN3app8lua_boolE\0",
            )
            .unwrap_or(0);
            dbg_log!(
                "[modmap] smashline={smashline:#x} l2c_common={l2c_common:#x} l2c_agent={l2c_agent:#x} self_attach={:#x}",
                article_agents::self_addresses().0
            );
        }
    }
    let source = custom_articles::custom_weapon_source_kind(kind);
    let kind_field = object.add(0xc) as *mut i32;
    if let Some(source) = source {
        core::ptr::write_volatile(kind_field, source);
    }
    dbg_log!(
        "[articleagent] creator entered for weapon kind {kind} (presenting {:?})",
        source
    );

    let agent = {
        let _pending = crate::enter_pending_weapon_kind(kind);
        call_original!(object, boma, lua_state)
    };

    if source.is_some() {
        core::ptr::write_volatile(kind_field, kind);
    }

    dbg_log!(
        "[articleagent] creator returned {:#x} vtable {:#x}",
        agent as usize,
        if agent.is_null() {
            0
        } else {
            core::ptr::read_volatile(agent as *const usize)
        }
    );
    if !agent.is_null() {
        if article_agents::wants_agent(kind) {
            article_agents::attach(kind, agent);
        }
    } else {
        dbg_log!("[articleagent] game declined weapon kind {kind}; passing the null through");
    }
    dbg_log!(
        "[articleagent] creator hook returning {:#x}",
        agent as usize
    );
    agent
}

#[cfg(feature = "css_slot")]
pub(crate) fn seen_motion_modules() -> &'static std::sync::RwLock<std::collections::HashSet<usize>>
{
    static MODULES: OnceLock<std::sync::RwLock<std::collections::HashSet<usize>>> = OnceLock::new();
    MODULES.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()))
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x49dc50)]
pub(crate) unsafe fn motion_rate_guard(module: *mut u8) -> f32 {
    const DEFAULT_RATE: f32 = 1.0;
    const MAX_LOGGED: usize = 64;

    if module.is_null() {
        static NULLS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        if NULLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) == 0 {
            dbg_log!("[motionrate] null module");
        }
        return DEFAULT_RATE;
    }

    let first_sight = match seen_motion_modules().write() {
        Ok(mut seen) => seen.len() < MAX_LOGGED && seen.insert(module as usize),
        Err(_) => false,
    };

    if first_sight {
        let flag = core::ptr::read_volatile(module.add(0x239));
        let holder = core::ptr::read_volatile(module.add(0x10) as *const *const u8);
        let set = if holder.is_null() {
            core::ptr::null()
        } else {
            core::ptr::read_volatile(holder.add(0x38) as *const *const u8)
        };
        let vtable = if set.is_null() {
            core::ptr::null()
        } else {
            core::ptr::read_volatile(set as *const *const usize)
        };
        let lookup = if vtable.is_null() {
            0
        } else {
            core::ptr::read_volatile(vtable.add(2))
        };

        let resource = core::ptr::read_volatile(module.add(0x148) as *const *const u8);
        let motion_count = if resource.is_null() {
            0
        } else {
            core::ptr::read_volatile(resource.add(0x98) as *const u32)
        };

        let module_vtable = core::ptr::read_volatile(module as *const usize);
        let change_motion = if module_vtable == 0 {
            0
        } else {
            core::ptr::read_volatile((module_vtable + 0xe0) as *const usize)
        };

        let stops_at = if flag != 0 && resource.is_null() {
            "NO MOTION RESOURCE: [module+0x148] null, change_motion bails at 0x4983d0"
        } else if flag != 0 && motion_count == 0 {
            "EMPTY MOTION RESOURCE: count 0, change_motion bails at 0x4983d8"
        } else if flag != 0 {
            "flag set but the resource looks bindable"
        } else if holder.is_null() {
            "[module+0x10] null: faults at 0x49dc70"
        } else if set.is_null() {
            "[holder+0x38] null: faults at 0x49dc74"
        } else if vtable.is_null() {
            "[set] null: faults at 0x49dc78"
        } else if lookup == 0 {
            "vtable+0x10 null: branches to zero at 0x49dc80"
        } else {
            "chain intact up to the lookup"
        };

        dbg_log!(
            "[motionrate] module={:#x} flag={flag} res={:#x} count={motion_count} holder={:#x} mvt={:#x} change_motion={:#x} set={:#x} vt={:#x} vt+0x10={:#x} - {stops_at}",
            module as usize,
            resource as usize,
            holder as usize,
            module_vtable,
            change_motion,
            set as usize,
            vtable as usize,
            lookup
        );
    }

    call_original!(module)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x497350)]
pub(crate) unsafe fn motion_bind_probe(
    module: *mut u8,
    arg1: *mut u8,
    arg2: u64,
    arg3: u32,
    arg4: u32,
    arg5: u32,
    arg6: u32,
) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller = lr.wrapping_sub(text_base());

    macro_rules! pass_through {
        () => {
            call_original!(module, arg1, arg2, arg3, arg4, arg5, arg6)
        };
    }

    if module.is_null() {
        return pass_through!();
    }

    static BOUND: OnceLock<std::sync::RwLock<std::collections::HashSet<usize>>> = OnceLock::new();
    let seen = BOUND.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()));
    let first_sight = match seen.write() {
        Ok(mut seen) => seen.len() < 64 && seen.insert(module as usize),
        Err(_) => false,
    };
    if !first_sight {
        return pass_through!();
    }

    let res_before = core::ptr::read_volatile(module.add(0x148) as *const usize);
    let holder_before = core::ptr::read_volatile(module.add(0x10) as *const usize);
    let teardown_flag = core::ptr::read_volatile(module.add(0x256));

    let ret = pass_through!();

    let res_after = core::ptr::read_volatile(module.add(0x148) as *const usize);
    let holder_after = core::ptr::read_volatile(module.add(0x10) as *const usize);
    let motion_flag = core::ptr::read_volatile(module.add(0x239));
    let expected = if res_after == 0 { 0 } else { res_after + 0xa0 };

    dbg_log!(
        "[motionbind] module={:#x} lr=@{caller:#x} +0x256={teardown_flag} res {:#x}->{:#x} holder {:#x}->{:#x} (expected {:#x}) flag={motion_flag}",
        module as usize,
        res_before,
        res_after,
        holder_before,
        holder_after,
        expected
    );

    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x6defa0)]
pub(crate) unsafe fn change_motion_probe(
    module: *mut u8,
    motion: u64,
    start_frame: f32,
    rate: f32,
    flag2: u32,
    frame: f32,
    flag3: u32,
    flag4: u32,
) -> u64 {
    const SHOOT: u64 = 0x5_7044_fcbe;

    static SEEN: OnceLock<std::sync::RwLock<std::collections::HashSet<usize>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()));
    let first_sight = match seen.write() {
        Ok(mut seen) => seen.len() < 64 && seen.insert(module as usize),
        Err(_) => false,
    };

    if !module.is_null() && (first_sight || motion == SHOOT) {
        static LINES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        if LINES.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 80 {
            let res = core::ptr::read_volatile(module.add(0x148) as *const usize);
            let holder = core::ptr::read_volatile(module.add(0x10) as *const usize);
            let flag = core::ptr::read_volatile(module.add(0x239));
            dbg_log!(
                "[changemotion] module={:#x} motion={:#x} res={:#x} holder={:#x} flag={flag}",
                module as usize,
                motion,
                res,
                holder
            );
        }
    }

    call_original!(
        module,
        motion,
        start_frame,
        rate,
        flag2,
        frame,
        flag3,
        flag4
    )
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x339fee0)]
pub(crate) unsafe fn weapon_motion_setup_probe(object: *mut u8) -> u64 {
    if object.is_null() {
        return call_original!(object);
    }



    #[cfg(feature = "diag_article_motion")]
    let kind = core::ptr::read_volatile(object.add(0xc) as *const i32);
    let boma = core::ptr::read_volatile(object.add(0x20) as *const usize);
    let owner_kind = if boma == 0 || crate::active_construction_kind_public().is_some() {
        None
    } else {
        crate::article_owner_kind_by_entry(boma as u64)
    };

    #[cfg(feature = "diag_article_motion")]
    let module = if boma == 0 {
        0
    } else {
        core::ptr::read_volatile((boma + 0x88) as *const usize)
    };
    #[cfg(feature = "diag_article_motion")]
    let holder_before = if module == 0 {
        0
    } else {
        core::ptr::read_volatile((module + 0x10) as *const usize)
    };

    #[cfg(feature = "diag_article_motion")]
    let model = if boma == 0 {
        0
    } else {
        core::ptr::read_volatile((boma + 0x78) as *const usize)
    };
    #[cfg(feature = "diag_article_motion")]
    let model_before = read_head(model);

    let ret = match owner_kind {
        Some(kind) => with_construction_context(kind, || call_original!(object)),
        None => call_original!(object),
    };

    if let Some(kind) = owner_kind {
        static SCOPE_LINES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = SCOPE_LINES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            dbg_log!(
                "[articlescope] #{n} weapon motion setup scoped to clone {kind} by entry                  (object={:#x})",
                object as usize
            );
        }
    }

    #[cfg(feature = "diag_article_motion")]
    let model_after = read_head(model);
    #[cfg(feature = "diag_article_motion")]
    let holder_after = if module == 0 {
        0
    } else {
        core::ptr::read_volatile((module + 0x10) as *const usize)
    };

    #[cfg(feature = "diag_article_motion")]
    {
        static LINES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = LINES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let minted = custom_articles::is_custom_weapon_kind(kind);
        if minted || n < 24 {
            dbg_log!(
            "[weaponmotion] #{n} kind={kind} minted={minted} obj={:#x} module={:#x} holder {:#x}->{:#x}",
            object as usize,
            module,
            holder_before,
            holder_after
        );
            dbg_log!(
            "[weaponmodel] #{n} kind={kind} model={model:#x} before={model_before:x?} after={model_after:x?}"
        );
        }
    }

    ret
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn read_head(object: usize) -> [usize; 24] {
    let mut head = [0usize; 24];
    if object == 0 {
        return head;
    }
    for (index, slot) in head.iter_mut().enumerate() {
        *slot = core::ptr::read_volatile((object + index * 8) as *const usize);
    }
    head
}

#[cfg(all(feature = "css_slot", feature = "diag_article_motion"))]
pub(crate) fn install_article_motion_diagnostics() {
    skyline::install_hooks!(motion_rate_guard, motion_bind_probe, change_motion_probe,);
}

#[cfg(all(feature = "css_slot", not(feature = "diag_article_motion")))]
pub(crate) fn install_article_motion_diagnostics() {}

#[cfg(feature = "css_slot")]
pub(crate) fn install_article_motion_scope_bridge() {
    skyline::install_hook!(weapon_motion_setup_probe);
}

#[cfg(feature = "css_slot")]
macro_rules! custom_article_gate_base_hooks {
    ($install:ident; $($name:ident($offset:expr, $base:expr, $kind_reg:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[$kind_reg].x() as i32;
                let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
                    return;
                };
                let bias = source as i64 - weapon_kind as i64;
                let base = ctx.registers[$base].x();
                ctx.registers[$base].set_x(base.wrapping_add(bias as u64));

                static GATE_LOG: core::sync::atomic::AtomicU32 =
                    core::sync::atomic::AtomicU32::new(0);
                let n = GATE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                if n < 4 {
                    dbg_log!(
                        "[articlecaps] {:#x}: weapon kind {weapon_kind} reads source {source}'s gate byte",
                        $offset as usize
                    );
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_gate_base_hooks! {
    install_custom_article_gate_base_hooks;
    custom_article_gate_motion(0x339ff90, 8, 0);
    custom_article_gate_0x3db770(0x3db770, 9, 8);
    custom_article_gate_0x641b40(0x641b40, 9, 8);
    custom_article_gate_0x64588c(0x64588c, 8, 9);
    custom_article_gate_0x33a486c(0x33a486c, 8, 9);
    custom_article_gate_0x33b6530(0x33b6530, 22, 21);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33afc00)]
pub(crate) unsafe fn custom_article_capability_index(weapon_kind: i32) -> i32 {
    let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
        return call_original!(weapon_kind);
    };

    let index = call_original!(source);

    static INDEX_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let n = INDEX_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        dbg_log!(
            "[articlecaps] #{n} weapon kind {weapon_kind} indexes source {source}'s capability row ({index})"
        );
    }
    index
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_article_status_v1(
    weapon_kind: i32,
    line: i32,
    status_kind: i32,
    function: *const (),
) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        if article_agents::register_status(weapon_kind, line, status_kind, function) {
            return RESULT_OK;
        }
        return clone_engine_api::ERROR_ARTICLE_SOURCE;
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (weapon_kind, line, status_kind, function);
        clone_engine_api::ERROR_UNSUPPORTED
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static MODULE_PROBE_SEEN: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

macro_rules! module_190_probes {
    ($install:ident; $($name:ident($offset:expr, $tag:expr, [$($reg:expr),*]));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let n = MODULE_PROBE_SEEN.load(core::sync::atomic::Ordering::Relaxed);
                if n >= 36 {
                    return;
                }
                MODULE_PROBE_SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                let mut parts = String::new();
                $(
                    parts.push_str(&format!(" x{}={:#x}", $reg, ctx.registers[$reg].x()));
                )*
                dbg_log!("[mod190] {}{}", $tag, parts);
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

module_190_probes! {
    install_module_190_probes;
    module_190_entry(0x48aa54, "entry", [0, 1]);
    module_190_graph(0x48aaf0, "graph", [19, 20, 9, 10]);
    module_190_list(0x48ab30, "list", [0, 20]);
    module_190_vtable(0x48ab68, "vtable", [0, 8]);
    module_190_past_call(0x48ab7c, "past-blr", [0, 19]);

    module_190_new_agent(0x48abb0, "newagent", [0, 19]);
    module_190_swap(0x48abb8, "swap", [0, 8, 19]);
    module_190_deleter(0x48abc4, "deleter", [0, 8]);
    module_190_teardown(0x48abd8, "teardown", [0]);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17e0840)]
pub(crate) unsafe fn custom_article_path_probe(
    out: *mut u32,
    weapon_kind: i32,
    resource_type: i32,
    variant: i32,
    color: i32,
    flags: i32,
) {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller = lr.wrapping_sub(text_base());

    call_original!(out, weapon_kind, resource_type, variant, color, flags);

    let construction_kind = active_construction_kind();
    let tracked = custom_articles::is_custom_weapon_kind(weapon_kind)
        || construction_kind
            .and_then(clone_definition)
            .map(|definition| {
                definition
                    .articles
                    .iter()
                    .any(|article| article.base_weapon_kind == weapon_kind)
            })
            .unwrap_or_else(|| (0x25..=0x2d).contains(&weapon_kind));
    if !tracked {
        return;
    }
    let n = ARTICLE_PATH_RESOLVE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n >= 96 {
        return;
    }
    let index = if out.is_null() {
        u32::MAX
    } else {
        core::ptr::read_volatile(out)
    };
    let namespace = construction_kind
        .and_then(clone_definition)
        .map(|definition| definition.resource_name)
        .unwrap_or("vanilla");
    dbg_log!(
        "[articlepath] #{n} lr=@{caller:#x} namespace={namespace} true_kind={:?} weapon_kind={weapon_kind:#x} type={resource_type} variant={variant} color={color} flags={flags:#x} index={index:#x}",
        construction_kind
    );

    if n == 0 && custom_articles::is_custom_weapon_kind(weapon_kind) {
        for candidate in [
            "fighter/donkey/motion/cannonballcloned/c00",
            "fighter/donkey/motion/cannonballcloned/c00/motion_list.bin",
            "fighter/donkey/model/cannonballcloned/c00",
            "fighter/donkey/model/cannonballcloned/c00/model.numdlb",
            "fighter/koopajr/motion/cannonball/c00",
            "fighter/koopajr/motion/cannonball/c00/motion_list.bin",
        ] {
            let candidate_index = fighter_modules::search_path_index(candidate);
            dbg_log!(
                "[articlepath]   candidate {candidate_index:#x} {}{candidate}",
                if candidate_index == index {
                    "<== MATCHES the builder  "
                } else {
                    ""
                }
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn is_custom_article_data_key(definition: &CloneDefinition, key: u64) -> bool {
    let weapon_kind = (key & 0x1ffff) as i32;
    definition
        .articles
        .iter()
        .any(|article| article.base_weapon_kind == weapon_kind)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn tag_custom_article_cache_key(key: u64, site: &str) -> u64 {
    let Some(definition) = active_construction_kind().and_then(clone_definition) else {
        return key;
    };
    if definition.articles.is_empty() || definition.article_namespace == 0 {
        return key;
    }
    let tagged = key | (u64::from(definition.article_namespace) << 33);
    let n = ARTICLE_CACHE_KEY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!("[articlecache] #{n} site={site} key={key:#x}->{tagged:#x}");
    }
    tagged
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17eea04, inline)]
pub(crate) unsafe fn custom_article_cache_key_single(ctx: &mut skyline::hooks::InlineCtx) {
    let key = ctx.registers[22].x();
    ctx.registers[22].set_x(tag_custom_article_cache_key(key, "single"));
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17eed48, inline)]
pub(crate) unsafe fn custom_article_cache_key_variant(ctx: &mut skyline::hooks::InlineCtx) {
    let key = ctx.registers[1].x();
    ctx.registers[1].set_x(tag_custom_article_cache_key(key, "variant"));
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17f0924, inline)]
pub(crate) unsafe fn custom_article_cache_key_direct(ctx: &mut skyline::hooks::InlineCtx) {
    let key = ctx.registers[21].x();
    ctx.registers[21].set_x(tag_custom_article_cache_key(key, "direct"));
}

#[cfg(feature = "css_slot")]
static ARTICLE_DATA_VANILLA_LOOKUP_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const ADDED_RESOURCE_INDEX_FLOOR: u32 = 0x8_0000;

#[cfg(feature = "css_slot")]
const BARE_WEAPON_KIND_CEILING: u64 = 0x1000;

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17dded0)]
pub(crate) unsafe fn custom_article_data_cache_insert(
    tree: *mut u8,
    key: u64,
    search_index: *const u32,
    variant: u32,
    mode: u32,
) -> u32 {
    let definition = active_construction_kind().and_then(clone_definition);
    let is_custom = definition
        .map(|definition| is_custom_article_data_key(definition, key))
        .unwrap_or(false);
    let effective_key = if let Some(definition) = definition.filter(|_| is_custom) {
        key | (u64::from(definition.article_namespace) << 36)
    } else {
        key
    };
    let index = if search_index.is_null() {
        u32::MAX
    } else {
        core::ptr::read_volatile(search_index)
    };
    let result = call_original!(tree, effective_key, search_index, variant, mode);
    let added = index != u32::MAX && index >= ADDED_RESOURCE_INDEX_FLOOR;
    if is_custom || added || key < BARE_WEAPON_KIND_CEILING {
        let n = ARTICLE_DATA_CACHE_KEY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 160 {
            let tag = if effective_key != key {
                "tagged"
            } else if added {
                "UNTAGGED-ADDED"
            } else {
                "vanilla"
            };
            dbg_log!(
                "[articledata] #{n} {tag} key={key:#x}->{effective_key:#x} index={index:#x} variant={variant} mode={mode} ret={result}"
            );
        }
    }
    result
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn tag_custom_article_data_lookup(
    ctx: &mut skyline::hooks::InlineCtx,
    site: &str,
) {
    let key = ctx.registers[2].x();
    let Some(definition) = active_construction_kind().and_then(clone_definition) else {
        if key < BARE_WEAPON_KIND_CEILING {
            let n = ARTICLE_DATA_VANILLA_LOOKUP_LOG
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 64 {
                dbg_log!("[articledata_lookup] #{n} site={site} VANILLA key={key:#x} untagged");
            }
        }
        return;
    };
    if !is_custom_article_data_key(definition, key) {
        return;
    }
    let tagged = key | (u64::from(definition.article_namespace) << 36);
    ctx.registers[2].set_x(tagged);
    let n = ARTICLE_DATA_CACHE_LOOKUP_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        dbg_log!("[articledata_lookup] #{n} site={site} key={key:#x}->{tagged:#x}");
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33a03c4, inline)]
pub(crate) unsafe fn custom_article_data_lookup_primary(ctx: &mut skyline::hooks::InlineCtx) {
    tag_custom_article_data_lookup(ctx, "primary");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33a9798, inline)]
pub(crate) unsafe fn custom_article_data_lookup_variant(ctx: &mut skyline::hooks::InlineCtx) {
    tag_custom_article_data_lookup(ctx, "variant");
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn log_article_data_result(
    ctx: &skyline::hooks::InlineCtx,
    stack_offset: usize,
    site: &str,
) {
    let Some(definition) = active_construction_kind().and_then(clone_definition) else {
        return;
    };
    if definition.articles.is_empty() {
        return;
    }
    let sp = ctx.sp.x() as *const u8;
    if sp.is_null() {
        return;
    }
    let object = core::ptr::read_unaligned(sp.add(stack_offset) as *const u64);
    let owner = core::ptr::read_unaligned(sp.add(stack_offset + 8) as *const u64);
    let key = ctx.registers[23].x();
    let n = ARTICLE_DATA_CACHE_RESULT_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        dbg_log!(
            "[articledata_result] #{n} site={site} key={key:#x} object={object:#x} owner={owner:#x}"
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33a03c8, inline)]
pub(crate) unsafe fn custom_article_data_result_primary(ctx: &mut skyline::hooks::InlineCtx) {
    log_article_data_result(ctx, 0x30, "primary");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x33a979c, inline)]
pub(crate) unsafe fn custom_article_data_result_variant(ctx: &mut skyline::hooks::InlineCtx) {
    log_article_data_result(ctx, 0x20, "variant");
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn handle_custom_base_module_name(
    ctx: &mut skyline::hooks::InlineCtx,
    destination: usize,
) {
    let raw_kind = ctx.registers[20].x() as i32;
    if let Some(definition) = clone_definition(raw_kind) {
        ctx.registers[destination].set_x(definition.base_resource_name_cstr.as_ptr() as u64);
    } else if let Some(kind) = active_resource_kind() {
        if let Some(definition) = clone_definition(kind) {
            ctx.registers[destination].set_x(definition.base_resource_name_cstr.as_ptr() as u64);
        }
    }
}

#[cfg(feature = "css_slot")]
macro_rules! declare_resource_name_hooks {
    ($install:ident, $handler:ident; $($name:ident($offset:expr, $dst:expr));* $(;)?) => {
        $(
            #[skyline::hook(offset = $offset, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                $handler(ctx, $dst);
            }
        )*

        pub(crate) fn $install() {
            skyline::install_hooks!($($name),*);
        }
    };
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_contextual_custom_resource_name_hooks, handle_custom_resource_name;
    custom_name_model_1(0x17e9aa4, 2);
    custom_name_model_2(0x17e9b94, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w1_custom_resource_name_hooks, handle_custom_resource_name_w1;
    custom_name_path_02(0x17df95c, 2);
    custom_name_path_03(0x17df888, 2);
    custom_name_path_18(0x17df920, 2);
    custom_name_path_19(0x17df934, 2);
    custom_name_path_20(0x17df8a8, 2);
    custom_name_path_21(0x17df948, 2);
    custom_name_path_22(0x17df970, 2);
    custom_name_path_23(0x17df984, 2);
    custom_name_path_24(0x17df8bc, 2);
    custom_name_path_25(0x17df8d0, 2);
    custom_name_path_26(0x17df8e4, 2);
    custom_name_path_27(0x17df998, 2);
    custom_name_path_28(0x17df9ac, 2);
    custom_name_path_29(0x17df9c0, 2);
    custom_name_path_30(0x17df8f8, 2);
    custom_name_path_31(0x17df90c, 2);
    custom_name_path_32(0x17df9d4, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w20_custom_resource_name_hooks, handle_custom_resource_name_w20;
    custom_name_path_04(0x17dfcfc, 2);
    custom_name_path_08(0x17e9094, 2);
    custom_name_path_09(0x17e9118, 2);
    custom_name_path_10(0x17e91b8, 2);
    custom_name_path_11(0x17e9238, 2);
    custom_name_path_12(0x17e9348, 2);
    custom_name_path_13(0x17e936c, 2);
    custom_name_path_14(0x17e9494, 2);
    custom_name_path_15(0x17e9600, 2);
    custom_name_path_17(0x17f0058, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w21_custom_resource_name_hooks, handle_custom_resource_name_w21;
    custom_name_path_05(0x17e0340, 2);
    custom_name_path_06(0x17e040c, 2);
    custom_name_path_16(0x17e9d58, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w19_custom_resource_name_hooks, handle_custom_resource_name_w19;
    custom_name_path_07(0x17e7558, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w8_custom_resource_name_hooks, handle_custom_resource_name_w8;
    custom_name_motion(0x60c184, 2);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = 0x17df4f0, inline)]
pub(crate) unsafe fn custom_name_path_root(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[1].x() as i32;
    if let Some(definition) = clone_definition(kind) {
        ctx.registers[2].set_x(definition.resource_name_cstr.as_ptr() as u64);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) fn install_custom_resource_name_hooks() {
    {
        let (attach, set_status_scripts) = article_agents::self_addresses();
        dbg_log!(
            "[selfbase] text={:#x} attach={attach:#x} set_status_scripts={set_status_scripts:#x} dbg_out={:#x}",
            text_base(),
            dbg_out as *const () as usize
        );
    }
    install_custom_article_weapon_name_hooks();
    install_custom_article_scaled_weapon_name_hooks();
    install_custom_article_scaled_owner_name_hooks();
    install_custom_article_agent_gate_hooks();
    install_custom_article_gate_base_hooks();
    install_custom_article_source_kind_sites();
    install_module_190_probes();
    install_article_motion_diagnostics();
    install_article_motion_scope_bridge();
    skyline::install_hooks!(module_190_factory, article_status_agent_create,);
    skyline::install_hooks!(
        custom_article_weapon_record_base,
        custom_article_capability_index,
        custom_article_owner_category,
        custom_article_path_weapon_name,
    );
    install_custom_article_owner_kind_hooks();
    install_custom_article_creator_owner_kind_hooks();
    install_contextual_custom_resource_name_hooks();
    install_w1_custom_resource_name_hooks();
    install_w19_custom_resource_name_hooks();
    install_w20_custom_resource_name_hooks();
    install_w21_custom_resource_name_hooks();
    install_w8_custom_resource_name_hooks();
    skyline::install_hooks!(
        custom_name_path_root,
        custom_article_path_probe,
        custom_article_owner_name,
        custom_article_weapon_name,
        custom_article_cache_key_single,
        custom_article_cache_key_variant,
        custom_article_cache_key_direct,
        custom_article_data_cache_insert,
        custom_article_data_lookup_primary,
        custom_article_data_lookup_variant,
        custom_article_data_result_primary,
        custom_article_data_result_variant,
        custom_effect_bank_load
    );
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_custom_module_name_hook, handle_custom_base_module_name;
    custom_base_module_name(0x17e4bcc, 22);
}
