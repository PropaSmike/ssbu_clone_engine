use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::RwLock;

use the_csk_collection_api::{
    BoolType, CharacterDatabaseEntry, Hash40Map, Hash40Type, IntType, ShortType, SignedByteType,
    StringType, UnsignedByteMap, UnsignedByteType,
};

const PLAYABLE_ROWS: &[&str] = &[
    "mario", "donkey", "link", "samus", "samusd", "yoshi", "kirby", "fox", "pikachu", "luigi",
    "ness", "captain", "purin", "peach", "daisy", "koopa", "ice_climber", "sheik", "zelda",
    "mariod", "pichu", "falco", "marth", "lucina", "younglink", "ganon", "mewtwo", "roy", "chrom",
    "gamewatch", "metaknight", "pit", "pitb", "szerosuit", "wario", "snake", "ike", "diddy",
    "lucas", "sonic", "dedede", "pikmin", "lucario", "robot", "toonlink", "wolf", "murabito",
    "rockman", "wiifit", "rosetta", "littlemac", "gekkouga", "palutena", "pacman", "reflet",
    "shulk", "koopajr", "duckhunt", "ryu", "ken", "cloud", "kamui", "bayonetta", "richter",
    "inkling", "ridley", "krool", "simon", "shizue", "gaogaen", "miifighter", "miiswordsman",
    "miigunner", "packun", "jack", "brave", "buddy", "dolly", "master", "tantan", "pickel",
    "edge", "demon", "trail",
];
const RENAMED_ROWS: &[(&str, &str)] = &[
    ("eflame", "flame_first"),
    ("elight", "light_first"),
    ("flame", "flame_only"),
    ("light", "light_only"),
];
const LOG_BUDGET: u32 = 16;
const PLAYER_COSTUME: usize = 0x19;
const PLAYER_ENTRY_ID: usize = 0xa7;
const PLAYER_INFO_IN_MATCH_SETTINGS: usize = 0x5b0;

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static LOG: AtomicU32 = AtomicU32::new(0);
static COSTUME_LOG: AtomicU32 = AtomicU32::new(0);
static COLOR_STARTS: RwLock<Vec<(u64, u8)>> = RwLock::new(Vec::new());

type AddCharaFn = unsafe extern "C" fn(*const CharacterDatabaseEntry);

fn hash40(text: &str) -> u64 {
    crate::hash40::hash40(text)
}

fn rows() -> impl Iterator<Item = (&'static str, &'static str)> {
    PLAYABLE_ROWS
        .iter()
        .map(|name| (*name, *name))
        .chain(RENAMED_ROWS.iter().copied())
}

fn is_playable_row(ui_chara: u64) -> bool {
    rows().any(|(_, row)| hash40(&format!("ui_chara_{row}")) == ui_chara)
}

fn base_row_for_kind(kind_hash: u64) -> Option<u64> {
    if let Some((_, row)) = rows().find(|(kind, _)| hash40(&format!("fighter_kind_{kind}")) == kind_hash) {
        return Some(hash40(&format!("ui_chara_{row}")));
    }
    let definitions = crate::clone_definitions().read().ok()?;
    definitions
        .iter()
        .find(|definition| hash40(definition.fighter_kind_name) == kind_hash)
        .map(|definition| hash40(&format!("ui_chara_{}", definition.base_resource_name)))
}

fn set_hash(value: &Hash40Type) -> Option<u64> {
    match value {
        Hash40Type::Overwrite(hash) | Hash40Type::Optional(Some(hash)) => Some(*hash),
        Hash40Type::Optional(None) => None,
    }
}

macro_rules! keep_set_values {
    ($entry:ident: $($field:ident as $kind:ident),* $(,)?) => {
        $(
            if let $kind::Optional(Some(value)) = &$entry.$field {
                $entry.$field = $kind::Overwrite(value.clone());
            }
        )*
    };
}

fn kept_index_map(map: &UnsignedByteMap) -> HashMap<u64, UnsignedByteType> {
    let values = match map {
        UnsignedByteMap::Overwrite(values) | UnsignedByteMap::Optional(Some(values)) => values.clone(),
        UnsignedByteMap::Optional(None) => HashMap::new(),
    };
    values
        .into_iter()
        .map(|(key, value)| match value {
            UnsignedByteType::Optional(Some(byte)) => (key, UnsignedByteType::Overwrite(byte)),
            other => (key, other),
        })
        .collect()
}

fn kept_hash_map(map: &Hash40Map) -> HashMap<u64, Hash40Type> {
    let values = match map {
        Hash40Map::Overwrite(values) | Hash40Map::Optional(Some(values)) => values.clone(),
        Hash40Map::Optional(None) => HashMap::new(),
    };
    values
        .into_iter()
        .map(|(key, value)| match value {
            Hash40Type::Optional(Some(hash)) => (key, Hash40Type::Overwrite(hash)),
            other => (key, other),
        })
        .collect()
}

fn set_byte(value: Option<&UnsignedByteType>) -> Option<u8> {
    match value? {
        UnsignedByteType::Overwrite(byte) | UnsignedByteType::Optional(Some(byte)) => Some(*byte),
        UnsignedByteType::Optional(None) => None,
    }
}

fn log(line: String) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&line);
    }
}

fn based_entry(entry: &CharacterDatabaseEntry) -> Option<CharacterDatabaseEntry> {
    if entry.clone_from_ui_chara_id.is_some() || is_playable_row(entry.ui_chara_id) {
        return None;
    }
    let kind = set_hash(&entry.fighter_kind)?;
    let Some(base) = base_row_for_kind(kind) else {
        log(format!(
            "[cskrow] {:#x} has no clone_from and its fighter_kind {kind:#x} is no playable or engine fighter; CSK builds it from scratch",
            entry.ui_chara_id
        ));
        return None;
    };
    let mut based = entry.clone();
    based.clone_from_ui_chara_id = Some(base);
    keep_set_values!(based:
        name_id as StringType,
        fighter_kind as Hash40Type,
        fighter_kind_corps as Hash40Type,
        ui_series_id as Hash40Type,
        fighter_type as Hash40Type,
        alt_chara_id as Hash40Type,
        exhibit_year as ShortType,
        exhibit_day_order as IntType,
        ext_skill_page_num as SignedByteType,
        is_img_ext_skill_page0 as BoolType,
        is_img_ext_skill_page1 as BoolType,
        is_img_ext_skill_page2 as BoolType,
        skill_list_order as SignedByteType,
        disp_order as SignedByteType,
        chara_count as SignedByteType,
        can_select as BoolType,
        is_usable_soundtest as BoolType,
        is_called_pokemon as BoolType,
        is_mii as BoolType,
        is_boss as BoolType,
        is_hidden_boss as BoolType,
        is_dlc as BoolType,
        is_patch as BoolType,
        is_plural_message as BoolType,
        is_plural_narration as BoolType,
        is_article as BoolType,
        extra_flags as IntType,
        has_multiple_face as BoolType,
        result_pf0 as BoolType,
        result_pf1 as BoolType,
        result_pf2 as BoolType,
        color_num as UnsignedByteType,
        shop_item_tag as Hash40Type,
    );
    based.save_no = SignedByteType::Optional(None);
    let mut indices = kept_index_map(&entry.extra_index_maps);
    let color_start = set_byte(indices.get(&hash40("color_start_index"))).unwrap_or(0) as u16;
    let color_count = match &based.color_num {
        UnsignedByteType::Overwrite(count) => *count as u16,
        _ => 0,
    };
    for color in color_start..color_start + color_count {
        for key in [format!("c{color:02}_index"), format!("n{color:02}_index"), format!("c{color:02}_group")] {
            indices.entry(hash40(&key)).or_insert(UnsignedByteType::Overwrite(0));
        }
    }
    based.extra_index_maps = UnsignedByteMap::Overwrite(indices);
    let mut hashes = kept_hash_map(&entry.extra_hash_maps);
    hashes
        .entry(hash40("original_ui_chara_hash"))
        .or_insert(Hash40Type::Overwrite(base));
    based.extra_hash_maps = Hash40Map::Overwrite(hashes);
    log(format!(
        "[cskrow] {:#x} had no clone_from; CSK now copies base row {base:#x} (fighter_kind {kind:#x}) keeping the pack's values, so it carries the base's save_no and shows in World of Light",
        entry.ui_chara_id
    ));
    Some(based)
}

fn remember_color_start(entry: &CharacterDatabaseEntry) {
    let indices = kept_index_map(&entry.extra_index_maps);
    let Some(start) = set_byte(indices.get(&hash40("color_start_index"))) else {
        return;
    };
    if start == 0 {
        return;
    }
    if let Ok(mut starts) = COLOR_STARTS.write() {
        starts.retain(|(ui_chara, _)| *ui_chara != entry.ui_chara_id);
        starts.push((entry.ui_chara_id, start));
    }
}

fn color_start(ui_chara: u64) -> Option<u8> {
    COLOR_STARTS
        .read()
        .ok()?
        .iter()
        .find(|(row, _)| *row == ui_chara)
        .map(|(_, start)| *start)
}

unsafe extern "C" fn add_chara_bridge(entry: *const CharacterDatabaseEntry) {
    let original = ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    let original: AddCharaFn = core::mem::transmute(original);
    if let Some(entry) = entry.as_ref() {
        remember_color_start(entry);
    }
    match entry.as_ref().and_then(based_entry) {
        Some(based) => original(&based),
        None => original(entry),
    }
}

#[skyline::hook(offset = crate::game::rel(crate::OFF_PLAYER_INFO_COSTUME_STORED), inline)]
unsafe fn player_info_costume_hook(ctx: &mut skyline::hooks::InlineCtx) {
    apply_color_start(ctx.registers[19].x() as *mut u8, "player written");
}

#[skyline::hook(offset = crate::game::rel(crate::OFF_TEAM_MEMBER_COSTUME_STORED), inline)]
unsafe fn team_member_costume_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let settings = ctx.registers[8].x() as *mut u8;
    if settings.is_null() {
        return;
    }
    apply_color_start(settings.add(PLAYER_INFO_IN_MATCH_SETTINGS), "team member written");
}

unsafe fn apply_color_start(player: *mut u8, site: &str) {
    if player.is_null() {
        return;
    }
    let ui_chara = core::ptr::read_unaligned(player as *const u64) & 0x00ff_ffff_ffff;
    let Some(start) = color_start(ui_chara) else {
        return;
    };
    let costume = core::ptr::read_volatile(player.add(PLAYER_COSTUME));
    if costume >= start {
        return;
    }
    let Some(shifted) = costume.checked_add(start) else {
        return;
    };
    core::ptr::write_volatile(player.add(PLAYER_COSTUME), shifted);
    if COSTUME_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&format!(
            "[cskrow] entry {} {ui_chara:#x} costume {costume} -> {shifted} ({site}): CSK's color_start_index {start} added as CSK does at match start, so World of Light's map fighter and finale team members get it too",
            core::ptr::read_volatile(player.add(PLAYER_ENTRY_ID))
        ));
    }
}

unsafe fn branch_destination(mut address: usize) -> usize {
    for _ in 0..4 {
        let word = core::ptr::read_volatile(address as *const u32);
        if word & 0xfc00_0000 != 0x1400_0000 {
            break;
        }
        let offset = (((word & 0x03ff_ffff) << 6) as i32 >> 4) as isize;
        address = address.wrapping_add_signed(offset);
    }
    address
}

pub(crate) unsafe fn install() {
    if ORIGINAL.load(Ordering::Acquire) != 0 {
        return;
    }
    let Some(export) = crate::css_registration::lookup_symbol(b"add_chara_db_entry_info\0") else {
        crate::dbg_log_public("[cskrow] CSK add_chara_db_entry_info is not resolvable; CSK rows registered without clone_from stay hidden in World of Light");
        return;
    };
    let target = branch_destination(export);
    let mut original = core::ptr::null_mut();
    skyline::hooks::A64HookFunction(
        target as *const libc::c_void,
        add_chara_bridge as *const () as *const libc::c_void,
        &mut original,
    );
    if original.is_null() {
        crate::dbg_log_public("[cskrow] hooking CSK add_chara_db_entry_info failed; CSK rows registered without clone_from stay hidden in World of Light");
        return;
    }
    ORIGINAL.store(original as usize, Ordering::Release);
    skyline::install_hook!(player_info_costume_hook);
    skyline::install_hook!(team_member_costume_hook);
    crate::dbg_log_public("[cskrow] armed: a CSK row registered without clone_from copies its fighter's base row and keeps the values its pack set; a player's costume gets the row's color_start_index whenever the player or a finale team member is written, not only at match start");
}
