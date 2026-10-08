use crate::custom_articles::{is_custom_weapon_kind, ArticleDescriptor, StaticArticleData};
use crate::offsets::{
    OFF_ARTICLE_BASE_CREATOR, OFF_ARTICLE_CONTAINER_ASSIGN_COUNT, OFF_ARTICLE_GET_ACTIVE_NUM,
    OFF_ARTICLE_IS_GENERATABLE,
};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};

const CONTAINER_LEADING_WEAPONS: usize = 0x220;
const CONTAINER_TABLE: usize = 0x228;
const MODULE_BASE_CONTAINER: usize = 0x240;
const ITEM_MARKER: u64 = 1;
const MAX_TABLE_ENTRIES: usize = 0x100;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;

const SITES: [(usize, [u32; 2]); 4] = [
    (OFF_ARTICLE_CONTAINER_ASSIGN_COUNT, [0xb902_201f, 0xb940_0829]),
    (OFF_ARTICLE_BASE_CREATOR, [0xd102_83ff, 0xfd00_2be8]),
    (OFF_ARTICLE_IS_GENERATABLE, [0xa9bd_57f6, 0xa901_4ff4]),
    (OFF_ARTICLE_GET_ACTIVE_NUM, [0xf81d_0ff5, 0xa901_4ff4]),
];

#[derive(Clone, Copy)]
struct View {
    view: usize,
    original: usize,
    descriptors: usize,
    count: usize,
    leading: u32,
}

pub(crate) struct Saved {
    container: usize,
    table: usize,
    leading: u32,
}

static VIEWS: OnceLock<RwLock<Vec<View>>> = OnceLock::new();
static VIEW_COUNT: AtomicUsize = AtomicUsize::new(0);

fn views() -> &'static RwLock<Vec<View>> {
    VIEWS.get_or_init(|| RwLock::new(Vec::new()))
}

unsafe fn view_for(table: usize) -> Option<usize> {
    if table < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let data = core::ptr::read_volatile(table as *const StaticArticleData);
    if (data.descriptors as usize) < LOWEST_PLAUSIBLE_POINTER
        || data.count == 0
        || data.count > MAX_TABLE_ENTRIES
    {
        return None;
    }
    let descriptors = core::slice::from_raw_parts(data.descriptors, data.count);
    let first_item = descriptors
        .iter()
        .position(|descriptor| descriptor.extra & ITEM_MARKER != 0)?;
    let trailing = descriptors[first_item..]
        .iter()
        .filter(|descriptor| descriptor.extra & ITEM_MARKER == 0 && is_custom_weapon_kind(descriptor.weapon_id))
        .count();
    if trailing == 0 {
        return None;
    }
    if let Some(found) = views().read().ok().and_then(|views| {
        views
            .iter()
            .find(|view| view.descriptors == data.descriptors as usize && view.count == data.count)
            .map(|view| view.view)
    }) {
        return Some(found);
    }
    let Ok(mut views) = views().write() else {
        return None;
    };
    if let Some(found) = views
        .iter()
        .find(|view| view.descriptors == data.descriptors as usize && view.count == data.count)
    {
        return Some(found.view);
    }
    let as_weapons: Vec<ArticleDescriptor> = descriptors
        .iter()
        .enumerate()
        .map(|(index, descriptor)| {
            let mut copy = *descriptor;
            if index >= first_item && descriptor.extra & ITEM_MARKER != 0 {
                copy.extra &= !ITEM_MARKER;
                copy.max_count = 0;
            }
            copy
        })
        .collect();
    let view = Box::leak(Box::new(StaticArticleData {
        descriptors: Vec::leak(as_weapons).as_ptr(),
        count: data.count,
    })) as *const StaticArticleData as usize;
    let original = Box::leak(Box::new(data)) as *const StaticArticleData as usize;
    views.push(View {
        view,
        original,
        descriptors: data.descriptors as usize,
        count: data.count,
        leading: first_item as u32,
    });
    VIEW_COUNT.store(views.len(), Ordering::Release);
    skyline::println!(
        "[articleorder] a {}-entry article table has {trailing} minted weapon(s) after its first item (entry {first_item}); its containers count them as weapons and its items keep the item path",
        data.count
    );
    Some(view)
}

pub(crate) unsafe fn item_view(container: usize, id: i32) -> Option<Saved> {
    if VIEW_COUNT.load(Ordering::Acquire) == 0 || container < LOWEST_PLAUSIBLE_POINTER || id < 0 {
        return None;
    }
    let table = core::ptr::read_volatile((container + CONTAINER_TABLE) as *const usize);
    let found = views()
        .read()
        .ok()?
        .iter()
        .find(|view| view.view == table)
        .copied()?;
    if id as usize >= found.count {
        return None;
    }
    let entry = core::ptr::read_volatile((found.descriptors as *const ArticleDescriptor).add(id as usize));
    if entry.extra & ITEM_MARKER == 0 {
        return None;
    }
    let leading = core::ptr::read_volatile((container + CONTAINER_LEADING_WEAPONS) as *const u32);
    core::ptr::write_volatile((container + CONTAINER_TABLE) as *mut usize, found.original);
    core::ptr::write_volatile((container + CONTAINER_LEADING_WEAPONS) as *mut u32, found.leading);
    Some(Saved {
        container,
        table,
        leading,
    })
}

pub(crate) unsafe fn restore(saved: Saved) {
    core::ptr::write_volatile((saved.container + CONTAINER_TABLE) as *mut usize, saved.table);
    core::ptr::write_volatile(
        (saved.container + CONTAINER_LEADING_WEAPONS) as *mut u32,
        saved.leading,
    );
}

unsafe fn module_item_view(module: u64, id: u32) -> Option<Saved> {
    if VIEW_COUNT.load(Ordering::Acquire) == 0 || (module as usize) < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let container = core::ptr::read_volatile((module as usize + MODULE_BASE_CONTAINER) as *const usize);
    item_view(container, id as i32)
}

#[skyline::hook(offset = crate::game::rel(OFF_ARTICLE_CONTAINER_ASSIGN_COUNT), inline)]
unsafe fn container_assign_count(ctx: &mut skyline::hooks::InlineCtx) {
    let container = ctx.registers[0].x() as usize;
    let table = ctx.registers[1].x() as usize;
    let Some(view) = view_for(table) else {
        return;
    };
    core::ptr::write_volatile((container + CONTAINER_TABLE) as *mut usize, view);
    ctx.registers[1].set_x(view as u64);
}

#[cfg(not(feature = "diag_kirby_copy"))]
#[skyline::hook(offset = crate::game::rel(OFF_ARTICLE_BASE_CREATOR))]
unsafe fn base_creator_item_view(container: u64, id: u32, owner: u64, argument: u32) -> u64 {
    let saved = item_view(container as usize, id as i32);
    let created = call_original!(container, id, owner, argument);
    if let Some(saved) = saved {
        restore(saved);
    }
    created
}

#[skyline::hook(offset = crate::game::rel(OFF_ARTICLE_IS_GENERATABLE))]
unsafe fn is_generatable_item_view(module: u64, id: u32) -> u64 {
    let saved = module_item_view(module, id);
    let generatable = call_original!(module, id);
    if let Some(saved) = saved {
        restore(saved);
    }
    generatable
}

#[skyline::hook(offset = crate::game::rel(OFF_ARTICLE_GET_ACTIVE_NUM))]
unsafe fn active_num_item_view(module: u64, id: u32) -> u64 {
    let saved = module_item_view(module, id);
    let active = call_original!(module, id);
    if let Some(saved) = saved {
        restore(saved);
    }
    active
}

pub(crate) fn install() {
    for (site, expected) in SITES {
        let found = unsafe {
            [
                crate::game::text_word(site),
                crate::game::text_word(site + 4),
            ]
        };
        if found != expected {
            skyline::println!(
                "[articleorder] preflight FAILED at {site:#x}: {:#010x} {:#010x}, expected {:#010x} {:#010x}; minted weapons after a base's items will not spawn",
                found[0],
                found[1],
                expected[0],
                expected[1]
            );
            return;
        }
    }
    skyline::install_hooks!(
        container_assign_count,
        is_generatable_item_view,
        active_num_item_view
    );
    #[cfg(not(feature = "diag_kirby_copy"))]
    skyline::install_hooks!(base_creator_item_view);
    skyline::println!(
        "[articleorder] armed: article containers count minted weapons placed after a base's items (0x3a6298, 0x3a6ce0, 0x3d6560, 0x3d6730)"
    );
}
