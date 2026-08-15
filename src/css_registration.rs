use super::*;

#[cfg(feature = "css_slot")]
struct CssNroTrace<'a>(&'a str);

#[cfg(feature = "css_slot")]
impl Drop for CssNroTrace<'_> {
    fn drop(&mut self) {
        skyline::println!("[nrotrace] engine-first EXIT name={}", self.0);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static CSS_SLOT_REGISTRATION_STARTED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
pub(crate) static CSS_SLOT_REGISTERED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
pub(crate) static CSK_GET_UI_CHARA_FROM_ENTRY: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static CSS_CUSTOM_ENTRY_KINDS: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; 8];

#[cfg(feature = "css_slot")]
pub(crate) fn cache_custom_entry_selection(entry_id: i32, kind: Option<i32>) {
    if !(0..8).contains(&entry_id) {
        return;
    }
    CSS_CUSTOM_ENTRY_KINDS[entry_id as usize]
        .store(kind.unwrap_or(-1), core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "css_slot")]
pub(crate) fn custom_entry_mask() -> u32 {
    let mut mask = 0u32;
    for (entry, kind) in CSS_CUSTOM_ENTRY_KINDS.iter().enumerate() {
        if kind.load(core::sync::atomic::Ordering::SeqCst) >= FIRST_CUSTOM_KIND {
            mask |= 1 << entry;
        }
    }
    mask
}

#[cfg(feature = "css_slot")]
pub(crate) fn entry_custom_kind(entry_id: u8) -> Option<i32> {
    if entry_id >= 8 {
        return None;
    }
    let kind = CSS_CUSTOM_ENTRY_KINDS[entry_id as usize].load(core::sync::atomic::Ordering::SeqCst);
    (kind >= FIRST_CUSTOM_KIND).then_some(kind)
}

#[cfg(feature = "css_slot")]
pub(crate) fn csk_entry_custom_kind(entry_id: u8) -> Option<i32> {
    type GetUiCharaFn = unsafe extern "C" fn(u32) -> u64;
    let address = CSK_GET_UI_CHARA_FROM_ENTRY.load(core::sync::atomic::Ordering::SeqCst);
    if address == 0 || entry_id >= 8 {
        return None;
    }
    let get_ui: GetUiCharaFn = unsafe { core::mem::transmute(address) };
    let ui_chara = unsafe { get_ui(entry_id as u32) } & 0x00ff_ffff_ffff;
    if ui_chara == 0 {
        return None;
    }
    clone_definition_from_ui(ui_chara).map(|definition| definition.kind)
}

#[cfg(feature = "css_slot")]
pub(crate) const ENTRY_RECORD_UI_CHARA: usize = 0x00;

#[cfg(feature = "css_slot")]
pub(crate) const ENTRY_RECORD_COSTUME: usize = 0x19;

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn record_clone_definition(
    source: *const u8,
) -> Option<&'static CloneDefinition> {
    if source.is_null() {
        return None;
    }
    let mut ui_chara = 0u64;
    for index in 0..5 {
        let byte = core::ptr::read_volatile(source.add(ENTRY_RECORD_UI_CHARA + index));
        ui_chara |= u64::from(byte) << (index * 8);
    }
    clone_definition_from_ui(ui_chara)
}

#[cfg(feature = "css_slot")]
pub(crate) fn validated_entry_custom_kind(
    entry_id: u8,
    input_kind: i32,
    record: Option<*const u8>,
    site: &'static str,
) -> Option<i32> {
    let Some(custom_kind) = entry_custom_kind(entry_id) else {
        if input_kind != 0 {
            return None;
        }
        if let Some(source) = record {
            if let Some(definition) = unsafe { record_clone_definition(source) } {
                let costume = unsafe { core::ptr::read_volatile(source.add(ENTRY_RECORD_COSTUME)) };
                let in_range = costume >= definition.color_start
                    && u16::from(costume)
                        < u16::from(definition.color_start)
                            + u16::from(definition.css_color_count());
                cache_custom_entry_selection(entry_id as i32, Some(definition.kind));
                dbg_log!(
                    "[csscache] recovered entry={entry_id} kind={} from record ui_chara costume={costume} in_range={in_range} site={site}",
                    definition.kind
                );
                return Some(definition.kind);
            }
        }
        let recovered = csk_entry_custom_kind(entry_id)?;
        cache_custom_entry_selection(entry_id as i32, Some(recovered));
        dbg_log!(
            "[csscache] recovered entry={entry_id} kind={recovered} from CSK entry->ui map site={site}"
        );
        return Some(recovered);
    };
    if input_kind == 0 || input_kind == custom_kind {
        return Some(custom_kind);
    }

    cache_custom_entry_selection(entry_id as i32, None);
    dbg_log!(
        "[csscache] stale entry={entry_id} cached_kind={custom_kind} current_kind={input_kind} site={site}; cleared"
    );
    None
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn lookup_symbol(name: &[u8]) -> Option<usize> {
    let mut address = 0usize;
    let result = LookupSymbol(&mut address as *mut usize, name.as_ptr());
    (result == 0 && address != 0).then_some(address)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn register_csk_css_slot() {
    type AddEntryFn = unsafe extern "C" fn(*const core::ffi::c_void);

    let Some(add_chara_addr) = lookup_symbol(b"add_chara_db_entry_info\0") else {
        skyline::println!("[css118] CSK add_chara_db_entry_info is not resolvable");
        return;
    };
    let Some(add_layout_addr) = lookup_symbol(b"add_chara_layout_db_entry_info\0") else {
        skyline::println!("[css118] CSK add_chara_layout_db_entry_info is not resolvable");
        return;
    };

    let add_chara: AddEntryFn = core::mem::transmute(add_chara_addr);
    let add_layout: AddEntryFn = core::mem::transmute(add_layout_addr);

    let published = clone_definitions()
        .read()
        .unwrap()
        .iter()
        .copied()
        .filter(|definition| definition.css.is_some() && clone_base(definition.kind).is_some())
        .collect::<Vec<_>>();

    for definition in published {
        let css = definition.css.expect("filtered on css.is_some()");
        register_one_csk_css_slot(definition, css, add_chara, add_layout);
    }

    if let Some(get_ui_addr) = lookup_symbol(b"get_ui_chara_from_entry_id\0") {
        CSK_GET_UI_CHARA_FROM_ENTRY.store(get_ui_addr, core::sync::atomic::Ordering::SeqCst);
    } else {
        skyline::println!("[css118] WARNING: get_ui_chara_from_entry_id is not resolvable");
    }

    CSS_SLOT_REGISTERED.store(true, core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn register_one_csk_css_slot(
    definition: &'static CloneDefinition,
    css: &'static CloneCssEntry,
    add_chara: unsafe extern "C" fn(*const core::ffi::c_void),
    add_layout: unsafe extern "C" fn(*const core::ffi::c_void),
) {
    use the_csk_collection_api::{
        BoolType, CStrCSK, CharacterDatabaseEntry, CharacterLayoutDatabaseEntry, Hash40Map,
        Hash40Type, IntType, ShortType, SignedByteType, StringType, UnsignedByteMap,
        UnsignedByteType,
    };

    type AllowOnlineFn = unsafe extern "C" fn(u64);

    let ui_chara = hash40(definition.ui_chara);
    let custom_kind_hash = hash40(definition.fighter_kind_name);
    let base_ui = hash40(&format!("ui_chara_{}", definition.base_resource_name));
    let base_characall = hash40(&format!(
        "vc_narration_characall_{}",
        definition.base_resource_name
    ));

    let mut ui_fallbacks = HashMap::new();
    let mut ui_indices = HashMap::new();

    let color_count = definition.css_color_count();
    for color in definition.color_start..definition.color_start + color_count {
        ui_fallbacks.insert(
            hash40(&format!("characall_label_c{color:02}")),
            Hash40Type::Overwrite(base_characall),
        );
        ui_fallbacks.insert(
            hash40(&format!("characall_label_article_c{color:02}")),
            Hash40Type::Overwrite(0),
        );
        ui_indices.insert(
            hash40(&format!("c{color:02}_index")),
            UnsignedByteType::Overwrite(0),
        );
        ui_indices.insert(
            hash40(&format!("n{color:02}_index")),
            UnsignedByteType::Overwrite(0),
        );
        ui_indices.insert(
            hash40(&format!("c{color:02}_group")),
            UnsignedByteType::Overwrite(0),
        );
    }
    ui_indices.insert(hash40("color_start_index"), UnsignedByteType::Overwrite(0));
    ui_fallbacks.insert(
        hash40("original_ui_chara_hash"),
        Hash40Type::Overwrite(base_ui),
    );

    let entry = CharacterDatabaseEntry {
        ui_chara_id: ui_chara,
        clone_from_ui_chara_id: Some(base_ui),
        name_id: StringType::Overwrite(CStrCSK::new(css.ui_name)),
        fighter_kind: Hash40Type::Overwrite(custom_kind_hash),
        fighter_kind_corps: Hash40Type::Overwrite(custom_kind_hash),
        ui_series_id: Hash40Type::Overwrite(hash40(css.ui_series)),
        fighter_type: Hash40Type::Overwrite(hash40("fighter_type_normal")),
        alt_chara_id: Hash40Type::Overwrite(0x2302_d482a),
        exhibit_year: ShortType::Overwrite(css.exhibit_year),
        ext_skill_page_num: SignedByteType::Overwrite(0),
        is_img_ext_skill_page0: BoolType::Overwrite(false),
        is_img_ext_skill_page1: BoolType::Overwrite(false),
        is_img_ext_skill_page2: BoolType::Overwrite(false),
        disp_order: SignedByteType::Overwrite(css.disp_order),
        save_no: SignedByteType::Overwrite(css.save_no),
        skill_list_order: SignedByteType::Overwrite(css.disp_order),
        chara_count: SignedByteType::Overwrite(1),
        can_select: BoolType::Overwrite(true),
        is_usable_soundtest: BoolType::Overwrite(true),
        is_called_pokemon: BoolType::Overwrite(false),
        is_mii: BoolType::Overwrite(false),
        is_boss: BoolType::Overwrite(false),
        is_hidden_boss: BoolType::Overwrite(false),
        is_dlc: BoolType::Overwrite(false),
        is_patch: BoolType::Overwrite(false),
        is_plural_message: BoolType::Overwrite(false),
        is_plural_narration: BoolType::Overwrite(false),
        is_article: BoolType::Overwrite(false),
        extra_flags: IntType::Overwrite(0),
        has_multiple_face: BoolType::Overwrite(false),
        result_pf0: BoolType::Overwrite(true),
        result_pf1: BoolType::Overwrite(true),
        result_pf2: BoolType::Overwrite(true),
        color_num: UnsignedByteType::Overwrite(color_count),
        extra_index_maps: UnsignedByteMap::Overwrite(ui_indices),
        extra_hash_maps: Hash40Map::Overwrite(ui_fallbacks),
        shop_item_tag: Hash40Type::Overwrite(0x2302_d482a),
        ..Default::default()
    };
    add_chara(&entry as *const _ as *const core::ffi::c_void);

    for color in definition.color_start..definition.color_start + color_count {
        let custom_layout = format!("{}_{color:02}", definition.ui_chara);
        let base_layout = format!(
            "ui_chara_{}_{:02}",
            definition.base_resource_name,
            color % VANILLA_COSTUME_SLOTS
        );
        let layout = CharacterLayoutDatabaseEntry {
            ui_layout_id: hash40(&custom_layout),
            clone_from_ui_layout_id: Some(hash40(&base_layout)),
            ui_chara_id: Hash40Type::Overwrite(ui_chara),
            chara_color: UnsignedByteType::Overwrite(color),
            ..Default::default()
        };
        add_layout(&layout as *const _ as *const core::ffi::c_void);
    }

    if let Some(allow_addr) = lookup_symbol(b"allow_ui_chara_hash_online\0") {
        let allow_online: AllowOnlineFn = core::mem::transmute(allow_addr);
        allow_online(ui_chara);
    }

    skyline::println!(
        "[css118] registered {} with CSK name_id={}, colors={}..{} -> {} -> kind {}; fighter/{}/",
        definition.ui_chara,
        css.ui_name,
        definition.color_start,
        definition.color_start + definition.color_count - 1,
        definition.fighter_kind_name,
        definition.kind,
        definition.resource_name
    );
}

#[cfg(feature = "css_slot")]
pub(crate) fn css_slot_nro_hook(info: &skyline::nro::NroInfo) {
    let _trace = if info.name == "samus" || info.name == "kirby" {
        skyline::println!("[nrotrace] engine-first ENTER name={}", info.name);
        Some(CssNroTrace(info.name))
    } else {
        None
    };

    if !POCKET_WATCH_INSTALLED.load(core::sync::atomic::Ordering::SeqCst)
        && unsafe { lookup_symbol(b"smashline_install_line_callback\0") }.is_some()
        && !POCKET_WATCH_INSTALLED.swap(true, core::sync::atomic::Ordering::SeqCst)
    {
        unsafe { install_pocket_watch() };
    }

    if CSS_SLOT_REGISTERED.load(core::sync::atomic::Ordering::SeqCst)
        || CSS_SLOT_REGISTRATION_STARTED.load(core::sync::atomic::Ordering::SeqCst)
    {
        return;
    }

    let symbols_ready = unsafe {
        lookup_symbol(b"add_chara_db_entry_info\0").is_some()
            && lookup_symbol(b"add_chara_layout_db_entry_info\0").is_some()
    };
    if !symbols_ready
        || CSS_SLOT_REGISTRATION_STARTED.swap(true, core::sync::atomic::Ordering::SeqCst)
    {
        return;
    }

    skyline::println!(
        "[css118] CSK API became resolvable on post-CSK NRO event '{}'; registering now",
        info.name
    );
    unsafe { register_csk_css_slot() };
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_CSS_ICON_COLUMN_BOUND, inline)]
pub(crate) unsafe fn css_icon_column_bound_hook(ctx: &mut skyline::hooks::InlineCtx) {
    const LAST_NATIVE_COLUMN: i32 = 12;

    let column = ctx.registers[13].x() as u32 as i32;
    if column > LAST_NATIVE_COLUMN {
        ctx.registers[13].set_x(LAST_NATIVE_COLUMN as u64);
        static CLAMP_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = CLAMP_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            dbg_log!(
                "[cssgrid] icon column {column} uses native column {LAST_NATIVE_COLUMN} animation delay"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_UI_FIGHTER_KIND_LOOKUP)]
pub(crate) unsafe fn ui_fighter_kind_lookup_hook(database: *mut u8, encoded_hash: u64) -> i32 {
    close_registration("CSS fighter-kind lookup");
    let kind_hash = encoded_hash & 0x00ff_ffff_ffff;
    if let Some(definition) = clone_definition_from_fighter_hash(kind_hash) {
        static LOGGED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
            dbg_log!(
                "[csskind] resolve {} hash={kind_hash:#x} -> kind {}",
                definition.fighter_kind_name,
                definition.kind
            );
        }
        return definition.kind;
    }
    call_original!(database, encoded_hash)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_UPDATE_SELECTED_FIGHTER)]
pub(crate) unsafe fn update_selected_fighter_hook(
    records: *mut u8,
    entry_id: i32,
    selection: *mut u8,
) {
    close_registration("CSS selection writer");
    type GetUiCharaFn = unsafe extern "C" fn(u32) -> u64;

    let selection_ui = if selection.is_null() {
        0
    } else {
        core::ptr::read_volatile(selection.add(0x18) as *const u64) & 0x00ff_ffff_ffff
    };
    let selection_definition = clone_definition_from_ui(selection_ui);
    if !selection.is_null() {
        cache_custom_entry_selection(
            entry_id,
            selection_definition.map(|definition| definition.kind),
        );
    }
    let selected_mask = custom_entry_mask();

    let get_ui_addr = CSK_GET_UI_CHARA_FROM_ENTRY.load(core::sync::atomic::Ordering::SeqCst);
    let ui_before = if get_ui_addr != 0 && (0..=7).contains(&entry_id) {
        let get_ui: GetUiCharaFn = core::mem::transmute(get_ui_addr);
        get_ui(entry_id as u32)
    } else {
        0
    };

    if let Some(definition) = selection_definition {
        with_resource_context(definition.kind, || {
            call_original!(records, entry_id, selection)
        });
    } else {
        call_original!(records, entry_id, selection);
    }

    if records.is_null() || !(0..=5).contains(&entry_id) {
        return;
    }

    let ui_after = if get_ui_addr != 0 {
        let get_ui: GetUiCharaFn = core::mem::transmute(get_ui_addr);
        get_ui(entry_id as u32)
    } else {
        0
    };
    let record = records.add(entry_id as usize * 0x20);
    let kind_ptr = record as *mut i32;
    let native_kind = core::ptr::read_volatile(kind_ptr);

    if let Some(definition) = selection_definition {
        core::ptr::write_volatile(kind_ptr, definition.kind);
        dbg_log!(
            "[cssselect] entry={entry_id} selection_ui={selection_ui:#x} api_before={ui_before:#x} api_after={ui_after:#x} mask={selected_mask:#04x} kind {native_kind}->{} record={:#x}",
            definition.kind,
            record as usize
        );
    } else {
        static TRACE_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = TRACE_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[cssselect] #{n} entry={entry_id} selection_ui={selection_ui:#x} api_before={ui_before:#x} api_after={ui_after:#x} mask={selected_mask:#04x} native_kind={native_kind}"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn bridge_custom_entry_expander_input(
    ctx: &mut skyline::hooks::InlineCtx,
    site: &'static str,
    counter: &core::sync::atomic::AtomicU32,
) {
    let source = ctx.registers[20].x() as *const u8;
    if source.is_null() {
        return;
    }
    let entry_id = core::ptr::read_volatile(source.add(0xa7));
    if entry_id >= 8 {
        return;
    }

    let input_kind = ctx.registers[0].x() as i32;
    let selected_kind = validated_entry_custom_kind(entry_id, input_kind, Some(source), site);
    let selected_mask = custom_entry_mask();
    let n = counter.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    if let Some(custom_kind) = selected_kind {
        ctx.registers[0].set_x(custom_kind as u64);
        dbg_log!(
            "[cssentry] {site} #{n} entry={entry_id} mask={selected_mask:#04x} expander kind {input_kind}->{custom_kind} source={:#x}",
            source as usize
        );
    } else if n < 24 {
        dbg_log!(
            "[cssentry] {site} #{n} entry={entry_id} mask={selected_mask:#04x} expander kind={input_kind} passthrough"
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static CSS_ENTRY_OUTER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static CSS_ENTRY_INNER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static CSS_ROSTER_ENTRY_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_MATCH_ENTRY_EXPAND_OUTER_CALL, inline)]
pub(crate) unsafe fn match_entry_expand_outer_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    bridge_custom_entry_expander_input(ctx, "outer@66dd14", &CSS_ENTRY_OUTER_COUNT);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_MATCH_ENTRY_EXPAND_INNER_CALL, inline)]
pub(crate) unsafe fn match_entry_expand_inner_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    bridge_custom_entry_expander_input(ctx, "inner@66db84", &CSS_ENTRY_INNER_COUNT);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = OFF_CONSTRUCTION_ROSTER_EXPAND_CALL, inline)]
pub(crate) unsafe fn construction_roster_expand_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let entry_id = ctx.registers[21].x() as u8;
    if entry_id >= 8 {
        return;
    }

    let input_kind = ctx.registers[0].x() as i32;
    let selected_kind = validated_entry_custom_kind(entry_id, input_kind, None, "roster@14ec248");
    let selected_mask = custom_entry_mask();
    let n = CSS_ROSTER_ENTRY_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if let Some(custom_kind) = selected_kind {
        ctx.registers[0].set_x(custom_kind as u64);
        dbg_log!(
            "[cssroster] #{n} entry={entry_id} mask={selected_mask:#04x} expander kind {input_kind}->{custom_kind}"
        );
    } else if n < 24 {
        dbg_log!(
            "[cssroster] #{n} entry={entry_id} mask={selected_mask:#04x} expander kind={input_kind} passthrough"
        );
    }
}

#[cfg(any(feature = "diag_article_initspoof", feature = "clone_runtime"))]
pub(crate) fn base_name_hash(base: i32) -> Option<u64> {
    let name = match base {
        0 => "mario",
        1 => "donkey",
        2 => "link",
        3 => "samus",
        4 => "samusd",
        5 => "yoshi",
        6 => "kirby",
        7 => "fox",
        8 => "pikachu",
        _ => return None,
    };
    Some(hash40(&format!("fighter_kind_{name}")))
}

#[cfg(feature = "diag_article_initspoof")]
pub(crate) unsafe fn manager_chain() -> (usize, usize, u64) {
    let mgr = *((text_base() + 0x5323680) as *const usize);
    if mgr == 0 {
        return (0, 0, 0);
    }
    let sub = *((mgr + 0x68) as *const usize);
    if sub == 0 {
        return (mgr, 0, 0);
    }
    (mgr, sub, *((sub + 0x1d238) as *const u64))
}

#[cfg(any(feature = "diag_article_initspoof", feature = "clone_runtime"))]
pub(crate) unsafe fn entry_kind_slot(entry_id: i32, kind: i32) -> Option<(*mut i32, usize, u32)> {
    let t1 = *((text_base() + 0x52b84f8) as *const usize);
    if t1 == 0 {
        return None;
    }
    let t2 = *(t1 as *const usize);
    if t2 == 0 {
        return None;
    }
    let per = *((t2 + entry_id as usize * 8 + 0x20) as *const usize);
    if per == 0 {
        return None;
    }
    let count = *((per + 0x14) as *const u32);
    if count == 0 || count > 8 {
        return None;
    }
    let arr = (per + 0x18) as *mut i32;
    for i in 0..count as usize {
        if *arr.add(i) == kind {
            return Some((arr.add(i), i, count));
        }
    }
    None
}

#[cfg(feature = "diag_article_initspoof")]
pub(crate) static INIT_LOG_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "diag_article_initspoof")]
#[skyline::hook(offset = OFF_FIGHTER_INIT_OBJ)]
pub(crate) unsafe fn fighter_init_object_data_hook(
    object: *mut u8,
    id: u32,
    kind: i32,
    entry_id: i32,
    name_hash: u64,
) {
    let n = INIT_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        let (mgr, sub, mtx) = manager_chain();
        dbg_log!(
            "[initspoof] #{n} ENTER kind={kind} entry={entry_id} id={id:#x} \
             name={name_hash:#x} mgr={mgr:#x} sub={sub:#x} mtx={mtx:#x}"
        );
    }
    #[cfg(feature = "true_kind")]
    if n < 4 {
        log_entry_installer_vt(n, entry_id);
    }
    if let Some(base) = clone_base(kind) {
        let spoof_hash = match base_name_hash(base) {
            Some(h) => h,
            None => {
                dbg_log!("[initspoof] WARNING: no name hash for base {base}; spoofing kind only");
                name_hash
            }
        };
        dbg_log!(
            "[initspoof] #{n} SPOOF kind {kind} -> base {base}, name {name_hash:#x} -> {spoof_hash:#x}"
        );
        let patched = entry_kind_slot(entry_id, kind);
        match patched {
            Some((_, idx, count)) => dbg_log!(
                "[initspoof] #{n} KINDARR entry={entry_id} count={count} idx={idx}: {kind} -> {base}"
            ),
            None => dbg_log!(
                "[initspoof] #{n} WARNING: kind {kind} not in entry {entry_id} kind array; \
                 search at 0x607a5c will miss (w5=-1 -> null-mutex crash)"
            ),
        }
        if let Some((slot, _, _)) = patched {
            *slot = base;
        }
        #[cfg(feature = "css_slot")]
        with_resource_context(kind, || {
            call_original!(object, id, base, entry_id, spoof_hash)
        });
        #[cfg(not(feature = "css_slot"))]
        call_original!(object, id, base, entry_id, spoof_hash);
        if let Some((slot, _, _)) = patched {
            *slot = kind;
        }
    } else {
        call_original!(object, id, kind, entry_id, name_hash);
    }
    if n < 64 {
        let (_, sub, mtx) = manager_chain();
        dbg_log!("[initspoof] #{n} EXIT kind={kind} sub={sub:#x} mtx={mtx:#x}");
    }
}
