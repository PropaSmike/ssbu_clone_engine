use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::RwLock;

use clone_engine_core::article_hurtbox::OBJECT_MODULE_ACCESSOR;
use clone_engine_core::article_lvd::{
    nearest_costume, ArticleLvd, ACCESSOR_GROUND_MODULE, COSTUMES, GROUND_DESCRIPTOR,
    GROUND_FILE_INDEX, GROUND_FIRST_CONTROL, GROUND_FIRST_GROUP, GROUND_FOLLOW_SLOT,
    GROUND_SECOND_CONTROL, GROUND_SECOND_GROUP, GROUP_BODY, GROUP_VERTEX_COUNT, SETUP_SLOT,
    SPEC_SLOT, SPEC_VIRTUAL_JOINTS, VIRTUAL_JOINT_GROUP_TAG, VIRTUAL_JOINT_WORDS,
};
use smash::app::lua_bind::{GroundModule, WorkModule};
use smash::app::BattleObjectModuleAccessor;
use smash::lib::lua_const::*;

use crate::clone_vtables::{self, Space};
use crate::custom_articles::{FIRST_CUSTOM_WEAPON_KIND, MAX_CUSTOM_ARTICLES};
use crate::dbg_log_public;

type Setup = unsafe extern "C" fn(usize, usize) -> usize;
type ModuleCall = unsafe extern "C" fn(usize);
type Spec = unsafe extern "C" fn(usize, usize, usize) -> usize;

#[repr(C)]
#[derive(Clone, Copy)]
struct JointGroup {
    entries: usize,
    count: u64,
    tag: u32,
    reserved: u32,
}

#[repr(C)]
struct JointGroupList {
    groups: usize,
    count: u64,
}

const LVD_PARSE: usize = 0x33bf610;
const PLATFORM_GROUND_VTABLE: usize = 0x51f7b10;
const RESOURCE_INDEX_MISSING: u32 = 0xffffff;
const EXPIRED_CONTROL: u64 = u64::MAX;
const REPORTS: u32 = 16;
const GROUP_VERTEX_LIST: usize = 0x68;
const VERTEX_POSITION: usize = 0x10;

#[repr(C)]
struct Descriptor {
    right: usize,
    left: usize,
    count: u32,
    reserved: u32,
}

struct Platform {
    kind: i32,
    lvd: ArticleLvd,
    shipped: Option<[Option<u32>; COSTUMES]>,
}

static ORIGINAL_SETUP: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static DESCRIPTORS: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static HIGHEST_VERTEX: [AtomicU32; MAX_CUSTOM_ARTICLES] =
    [const { AtomicU32::new(0) }; MAX_CUSTOM_ARTICLES];
static INSTALLED: [AtomicBool; MAX_CUSTOM_ARTICLES] =
    [const { AtomicBool::new(false) }; MAX_CUSTOM_ARTICLES];
static PLATFORMS: RwLock<Vec<Platform>> = RwLock::new(Vec::new());
static APPLIED_LOG: AtomicU32 = AtomicU32::new(0);
static REFUSED_LOG: AtomicU32 = AtomicU32::new(0);
static STRAY_LOG: AtomicU32 = AtomicU32::new(0);
static ORIGINAL_SPEC: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static JOINT_GROUPS: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static SPEC_INSTALLED: [AtomicBool; MAX_CUSTOM_ARTICLES] =
    [const { AtomicBool::new(false) }; MAX_CUSTOM_ARTICLES];
static JOINT_LISTS: RwLock<Vec<(i32, usize, usize)>> = RwLock::new(Vec::new());
static JOINT_LOG: AtomicU32 = AtomicU32::new(0);

fn row(kind: i32) -> Option<usize> {
    let row = usize::try_from(kind.checked_sub(FIRST_CUSTOM_WEAPON_KIND)?).ok()?;
    (row < MAX_CUSTOM_ARTICLES).then_some(row)
}

pub(crate) fn wants_platform(kind: i32) -> bool {
    row(kind).is_some_and(|row| DESCRIPTORS[row].load(Ordering::Acquire) != 0)
}

fn leak_entries(entries: &[(u32, u64)]) -> usize {
    let words: &'static [u64] = Box::leak(ArticleLvd::entry_words(entries).into_boxed_slice());
    words.as_ptr() as usize
}

pub(crate) fn register(kind: i32, lvd: &ArticleLvd) -> Result<(), String> {
    let row = row(kind).ok_or_else(|| format!("weapon kind {kind} is not a minted article"))?;
    lvd.check()?;
    let descriptor: &'static Descriptor = Box::leak(Box::new(Descriptor {
        right: leak_entries(&lvd.right()),
        left: leak_entries(&lvd.left()),
        count: lvd.vertices.len() as u32,
        reserved: 0,
    }));
    if let Ok(mut platforms) = PLATFORMS.write() {
        platforms.retain(|platform| platform.kind != kind);
        platforms.push(Platform {
            kind,
            lvd: lvd.clone(),
            shipped: None,
        });
    }
    HIGHEST_VERTEX[row].store(lvd.highest_vertex().unwrap_or(0), Ordering::Release);
    DESCRIPTORS[row].store(descriptor as *const Descriptor as usize, Ordering::Release);
    register_virtual_joints(kind, row, lvd)?;
    if INSTALLED[row].swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let original_out = &ORIGINAL_SETUP[row] as *const AtomicUsize as usize;
    let installed = unsafe {
        clone_vtables::set_entry(
            kind,
            SETUP_SLOT,
            Space::Weapon,
            setup_with_platform as *const () as usize,
            original_out,
        )
    };
    if installed.is_none() {
        INSTALLED[row].store(false, Ordering::Release);
        DESCRIPTORS[row].store(0, Ordering::Release);
        return Err(format!("weapon kind {kind} has no private vtable to carry a platform"));
    }
    Ok(())
}

fn trimmed(bytes: &[u8]) -> Option<String> {
    core::str::from_utf8(bytes.split(|byte| *byte == 0).next()?)
        .ok()
        .map(str::to_owned)
}

unsafe fn shipped_files(kind: i32) -> Option<(String, [Option<u32>; COSTUMES])> {
    if let Ok(platforms) = PLATFORMS.read() {
        let platform = platforms.iter().find(|platform| platform.kind == kind)?;
        if let Some(shipped) = platform.shipped {
            return Some((platform.lvd.file_name().to_string(), shipped));
        }
    }
    let owner = trimmed(crate::custom_articles::resource_owner_of(kind)?)?;
    let name = trimmed(crate::custom_articles::custom_weapon_name(kind)?)?;
    let mut platforms = PLATFORMS.write().ok()?;
    let platform = platforms.iter_mut().find(|platform| platform.kind == kind)?;
    let file = platform.lvd.file_name().to_string();
    let mut shipped = [None; COSTUMES];
    for (costume, slot) in shipped.iter_mut().enumerate() {
        let path = format!("fighter/{owner}/model/{name}/c{costume:02}/{file}");
        *slot = crate::item_params::scan_file_path_index(crate::hash40::hash40(&path));
    }
    platform.shipped = Some(shipped);
    let found: Vec<String> = shipped
        .iter()
        .enumerate()
        .filter(|(_, index)| index.is_some())
        .map(|(costume, _)| format!("c{costume:02}"))
        .collect();
    if found.is_empty() {
        dbg_log_public(&format!(
            "[articlelvd] weapon kind {kind}: no fighter/{owner}/model/{name}/cNN/{file} in any costume, so it has no platform"
        ));
    } else {
        dbg_log_public(&format!(
            "[articlelvd] weapon kind {kind}: {file} found for {}; other costumes borrow the nearest",
            found.join(" ")
        ));
    }
    Some((file, shipped))
}

pub(crate) unsafe fn keep_resident(kind: i32) {
    if !wants_platform(kind) {
        return;
    }
    let Some((_, shipped)) = shipped_files(kind) else {
        return;
    };
    for index in shipped.iter().flatten() {
        if crate::fighter_params::file_resident(*index).is_err() {
            crate::fighter_params::request_resource_file(*index as i32);
        }
    }
}

fn register_virtual_joints(kind: i32, row: usize, lvd: &ArticleLvd) -> Result<(), String> {
    let words = lvd.virtual_joint_words();
    if words.is_empty() {
        JOINT_GROUPS[row].store(0, Ordering::Release);
        return Ok(());
    }
    let entries: &'static [u64] = Box::leak(words.into_boxed_slice());
    let group: &'static JointGroup = Box::leak(Box::new(JointGroup {
        entries: entries.as_ptr() as usize,
        count: (entries.len() / VIRTUAL_JOINT_WORDS) as u64,
        tag: VIRTUAL_JOINT_GROUP_TAG,
        reserved: 0,
    }));
    if let Ok(mut lists) = JOINT_LISTS.write() {
        lists.retain(|(listed, _, _)| *listed != kind);
    }
    JOINT_GROUPS[row].store(group as *const JointGroup as usize, Ordering::Release);
    if SPEC_INSTALLED[row].swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let original_out = &ORIGINAL_SPEC[row] as *const AtomicUsize as usize;
    let installed = unsafe {
        clone_vtables::set_entry(
            kind,
            SPEC_SLOT,
            Space::Weapon,
            spec_with_joints as *const () as usize,
            original_out,
        )
    };
    if installed.is_none() {
        SPEC_INSTALLED[row].store(false, Ordering::Release);
        JOINT_GROUPS[row].store(0, Ordering::Release);
        return Err(format!("weapon kind {kind} has no private vtable to carry offset joints"));
    }
    Ok(())
}

unsafe extern "C" fn spec_with_joints(class: usize, object: usize, spec: usize) -> usize {
    let Some((kind, row)) = clone_vtables::weapon_kind_of_class(class)
        .and_then(|kind| Some((kind, row(kind)?)))
    else {
        return 0;
    };
    let original = ORIGINAL_SPEC[row].load(Ordering::Acquire);
    let result = if original != 0 {
        let original: Spec = core::mem::transmute(original);
        original(class, object, spec)
    } else {
        0
    };
    add_virtual_joints(kind, row, spec);
    result
}

unsafe fn joint_list_for(kind: i32, group: usize, existing: usize) -> usize {
    if let Ok(lists) = JOINT_LISTS.read() {
        if let Some((_, _, list)) = lists
            .iter()
            .find(|(listed, source, _)| *listed == kind && *source == existing)
        {
            return *list;
        }
    }
    let mut groups: Vec<JointGroup> = Vec::new();
    if existing != 0 {
        let source = read(existing);
        let count = read(existing + 8);
        for index in 0..count {
            groups.push(core::ptr::read_volatile(
                (source + index * core::mem::size_of::<JointGroup>()) as *const JointGroup,
            ));
        }
    }
    groups.push(core::ptr::read(group as *const JointGroup));
    let count = groups.len() as u64;
    let groups: &'static [JointGroup] = Box::leak(groups.into_boxed_slice());
    let list: &'static JointGroupList = Box::leak(Box::new(JointGroupList {
        groups: groups.as_ptr() as usize,
        count,
    }));
    let list = list as *const JointGroupList as usize;
    if let Ok(mut lists) = JOINT_LISTS.write() {
        lists.push((kind, existing, list));
    }
    list
}

unsafe fn add_virtual_joints(kind: i32, row: usize, spec: usize) {
    let group = JOINT_GROUPS[row].load(Ordering::Acquire);
    if group == 0 || spec == 0 {
        return;
    }
    let existing = read(spec + SPEC_VIRTUAL_JOINTS);
    let list = joint_list_for(kind, group, existing);
    core::ptr::write_volatile((spec + SPEC_VIRTUAL_JOINTS) as *mut usize, list);
    if JOINT_LOG.fetch_add(1, Ordering::Relaxed) < REPORTS {
        dbg_log_public(&format!(
            "[articlelvd] weapon kind {kind}: {} offset joint(s) added to its model{}",
            (*(group as *const JointGroup)).count,
            if existing != 0 { " next to its source's own" } else { "" }
        ));
    }
}

unsafe fn kind_of(class: usize, weapon: usize) -> Option<i32> {
    clone_vtables::weapon_kind_of_class(class).or_else(|| clone_vtables::minted_kind_of_object(weapon))
}

unsafe extern "C" fn setup_with_platform(class: usize, weapon: usize) -> usize {
    let Some((kind, row)) = kind_of(class, weapon).and_then(|kind| Some((kind, row(kind)?))) else {
        if STRAY_LOG.fetch_add(1, Ordering::Relaxed) < 4 {
            dbg_log_public(&format!(
                "[articlelvd] setup reached for class {class:#x} object {weapon:#x}, which is no minted article; no platform"
            ));
        }
        return 0;
    };
    let original = ORIGINAL_SETUP[row].load(Ordering::Acquire);
    let result = if original != 0 {
        let original: Setup = core::mem::transmute(original);
        original(class, weapon)
    } else {
        0
    };
    apply(kind, row, weapon);
    result
}

unsafe fn read(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn owner_costume(accessor: *mut BattleObjectModuleAccessor) -> usize {
    for work in [
        *WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER,
        *WEAPON_INSTANCE_WORK_ID_INT_ACTIVATE_FOUNDER_ID,
    ] {
        let owner_id = WorkModule::get_int(accessor, work) as u32;
        if owner_id == *BATTLE_OBJECT_ID_INVALID as u32
            || !smash::app::sv_battle_object::is_active(owner_id)
        {
            continue;
        }
        let owner = smash::app::sv_battle_object::module_accessor(owner_id);
        if owner.is_null()
            || smash::app::utility::get_category(&mut *owner) != *BATTLE_OBJECT_CATEGORY_FIGHTER
        {
            continue;
        }
        return WorkModule::get_int(owner, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR).max(0) as usize;
    }
    0
}

unsafe fn group_vertices(module: usize, group: usize, control: usize) -> Option<u32> {
    let group = read(module + group);
    let control = read(module + control);
    if group == 0 || control == 0 || read(control + 8) as u64 == EXPIRED_CONTROL {
        return None;
    }
    let body = read(group + GROUP_BODY);
    if body == 0 {
        return None;
    }
    Some(core::ptr::read_volatile((body + GROUP_VERTEX_COUNT) as *const u32))
}

unsafe fn vertex_position(module: usize, vertex: u32, first: u32) -> Option<(f32, f32)> {
    let (group, index) = if vertex < first {
        (read(module + GROUND_FIRST_GROUP), vertex)
    } else {
        (read(module + GROUND_SECOND_GROUP), vertex - first)
    };
    if group == 0 {
        return None;
    }
    let body = read(group + GROUP_BODY);
    if body == 0 {
        return None;
    }
    let mut node = read(body + GROUP_VERTEX_LIST);
    for _ in 0..index {
        if node == 0 {
            return None;
        }
        node = read(node);
    }
    if node == 0 {
        return None;
    }
    Some((
        core::ptr::read_volatile((node + VERTEX_POSITION) as *const f32),
        core::ptr::read_volatile((node + VERTEX_POSITION + 4) as *const f32),
    ))
}

fn refuse(kind: i32, message: String) {
    if REFUSED_LOG.fetch_add(1, Ordering::Relaxed) < REPORTS {
        dbg_log_public(&format!("[articlelvd] weapon kind {kind}: {message}"));
    }
}

unsafe fn apply(kind: i32, row: usize, weapon: usize) {
    let descriptor = DESCRIPTORS[row].load(Ordering::Acquire);
    if descriptor == 0 || weapon == 0 {
        return;
    }
    let accessor = read(weapon + OBJECT_MODULE_ACCESSOR);
    if accessor == 0 {
        return;
    }
    let module = read(accessor + ACCESSOR_GROUND_MODULE);
    if module == 0 {
        return;
    }
    let vtable = read(module);
    if vtable != crate::game::addr(PLATFORM_GROUND_VTABLE) {
        refuse(
            kind,
            format!(
                "its GroundModule has vtable {:#x}, not the platform one {PLATFORM_GROUND_VTABLE:#x}; no platform",
                crate::game::offset_of(vtable)
            ),
        );
        return;
    }
    core::ptr::write_volatile((module + GROUND_DESCRIPTOR) as *mut usize, 0);
    let Some((file, shipped)) = shipped_files(kind) else {
        return;
    };
    let boma = accessor as *mut BattleObjectModuleAccessor;
    let costume = owner_costume(boma);
    let Some((from, index)) = nearest_costume(&shipped, costume) else {
        return;
    };
    if let Err(reason) = crate::fighter_params::file_resident(index) {
        crate::fighter_params::request_resource_file(index as i32);
        refuse(
            kind,
            format!("{file} for c{from:02} (file {index:#x}) is not loaded yet ({reason}); requested, no platform this time"),
        );
        return;
    }
    let held = core::ptr::read_volatile((module + GROUND_FILE_INDEX) as *const u32);
    crate::fighter_params::request_resource_file(index as i32);
    core::ptr::write_volatile((module + GROUND_FILE_INDEX) as *mut u32, index);
    if held != RESOURCE_INDEX_MISSING {
        crate::fighter_params::release_resource_file(held as i32);
    }
    let parse: ModuleCall = core::mem::transmute(crate::game::addr(LVD_PARSE));
    parse(module);
    let Some(first) = group_vertices(module, GROUND_FIRST_GROUP, GROUND_FIRST_CONTROL) else {
        refuse(kind, format!("{file} (file {index:#x}) built no collision; no platform"));
        return;
    };
    let second = group_vertices(module, GROUND_SECOND_GROUP, GROUND_SECOND_CONTROL).unwrap_or(0);
    let total = first + second;
    let highest = HIGHEST_VERTEX[row].load(Ordering::Acquire);
    if highest >= total {
        refuse(
            kind,
            format!("lvd_vertex {highest} is past the {total} vertices {file} has; no platform"),
        );
        return;
    }
    core::ptr::write_volatile((module + GROUND_DESCRIPTOR) as *mut usize, descriptor);
    let follow: ModuleCall = core::mem::transmute(read(vtable + GROUND_FOLLOW_SLOT * 8));
    follow(module);
    GroundModule::set_gr_collision_mode(boma, true);
    if APPLIED_LOG.fetch_add(1, Ordering::Relaxed) < REPORTS {
        let listed = core::ptr::read_volatile((descriptor + 0x10) as *const u32);
        let placed: Vec<String> = (0..total)
            .map(|vertex| match vertex_position(module, vertex, first) {
                Some((x, y)) => format!("{vertex}=({x:.1},{y:.1})"),
                None => format!("{vertex}=?"),
            })
            .collect();
        dbg_log_public(&format!(
            "[articlelvd] weapon kind {kind} object {weapon:#x}: platform {file} c{from:02} for costume {costume} (file {index:#x}), {total} vertices, {listed} on joints{}; vertices {} with the article at ({:.1},{:.1})",
            if (listed as usize) < total as usize {
                ", the rest stay where the file puts them"
            } else {
                ""
            },
            placed.join(" "),
            smash::app::lua_bind::PostureModule::pos_x(boma),
            smash::app::lua_bind::PostureModule::pos_y(boma)
        ));
    }
}
