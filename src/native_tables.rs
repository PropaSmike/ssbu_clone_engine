use super::{hash40, text_base, CloneDefinition};
use clone_engine_api::{
    BACKEND_STATUS_COMPILED, BACKEND_STATUS_HOOKS_INSTALLED, BACKEND_STATUS_READY,
    BACKEND_STATUS_RESOURCE_LAYOUT_PROVEN, BACKEND_STATUS_RESOURCE_LIFECYCLE_PROVEN,
    BACKEND_STATUS_STATIC_PREFLIGHT_OK, BACKEND_STATUS_STATIC_TABLES_READY,
};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;

struct ExpectedOpcode {
    offset: usize,
    opcode: u32,
    label: &'static str,
}

include!("native_tables_13_0_4.rs");

const NAME_UPPER_OFFSET: usize = 0x4f80a70;
const NAME_LOWER_OFFSET: usize = 0x4f80e20;
const NAME_TITLE_OFFSET: usize = 0x4f811d0;
const KIND_HASH_OFFSET: usize = 0x453b1c0;
const COMPACT_HASH_OFFSET: usize = 0x453bc58;
const CLASS_DESCRIPTOR_OFFSET: usize = 0x529bfd0;
const NAME_HASH_NATIVE_LEN: usize = 118;
const COMPACT_NATIVE_LEN: usize = 94;

static NONE_UPPER: &[u8] = b"NONE\0";
static NONE_LOWER: &[u8] = b"none\0";
static NONE_TITLE: &[u8] = b"None\0";

#[repr(align(64))]
struct Aligned<T>(T);

struct NativeTables {
    name_upper: Box<Aligned<[AtomicUsize; BACKEND_CAPACITY]>>,
    name_lower: Box<Aligned<[AtomicUsize; BACKEND_CAPACITY]>>,
    name_title: Box<Aligned<[AtomicUsize; BACKEND_CAPACITY]>>,
    kind_hash: Box<Aligned<[AtomicU64; BACKEND_CAPACITY]>>,
    compact_hash: Box<Aligned<[AtomicU64; BACKEND_CAPACITY]>>,
    class_descriptor: Box<Aligned<[AtomicUsize; BACKEND_CAPACITY]>>,
    compact_next: AtomicUsize,
}

impl NativeTables {
    unsafe fn copy_from_game() -> Self {
        let none_hash = hash40("fighter_kind_none");
        let tables = Self {
            name_upper: Box::new(Aligned(std::array::from_fn(|_| {
                AtomicUsize::new(NONE_UPPER.as_ptr() as usize)
            }))),
            name_lower: Box::new(Aligned(std::array::from_fn(|_| {
                AtomicUsize::new(NONE_LOWER.as_ptr() as usize)
            }))),
            name_title: Box::new(Aligned(std::array::from_fn(|_| {
                AtomicUsize::new(NONE_TITLE.as_ptr() as usize)
            }))),
            kind_hash: Box::new(Aligned(std::array::from_fn(|_| AtomicU64::new(none_hash)))),
            compact_hash: Box::new(Aligned(std::array::from_fn(|_| AtomicU64::new(none_hash)))),
            class_descriptor: Box::new(Aligned(std::array::from_fn(|_| AtomicUsize::new(0)))),
            compact_next: AtomicUsize::new(COMPACT_NATIVE_LEN),
        };

        let base = text_base();
        for index in 0..NAME_HASH_NATIVE_LEN {
            tables.name_upper.0[index].store(
                core::ptr::read((base + NAME_UPPER_OFFSET + index * 8) as *const usize),
                Ordering::Relaxed,
            );
            tables.name_lower.0[index].store(
                core::ptr::read((base + NAME_LOWER_OFFSET + index * 8) as *const usize),
                Ordering::Relaxed,
            );
            tables.name_title.0[index].store(
                core::ptr::read((base + NAME_TITLE_OFFSET + index * 8) as *const usize),
                Ordering::Relaxed,
            );
            tables.kind_hash.0[index].store(
                core::ptr::read((base + KIND_HASH_OFFSET + index * 8) as *const u64),
                Ordering::Relaxed,
            );
        }
        for index in 0..COMPACT_NATIVE_LEN {
            tables.compact_hash.0[index].store(
                core::ptr::read((base + COMPACT_HASH_OFFSET + index * 8) as *const u64),
                Ordering::Relaxed,
            );
            tables.class_descriptor.0[index].store(
                core::ptr::read((base + CLASS_DESCRIPTOR_OFFSET + index * 8) as *const usize),
                Ordering::Relaxed,
            );
        }
        tables
    }

    fn reserve_compact_slot(&self) -> Option<usize> {
        let mut current = self.compact_next.load(Ordering::Acquire);
        loop {
            if current >= BACKEND_CAPACITY {
                return None;
            }
            match self.compact_next.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(current),
                Err(observed) => current = observed,
            }
        }
    }

    fn publish(&self, definition: &CloneDefinition) -> bool {
        let Ok(kind) = usize::try_from(definition.kind) else {
            return false;
        };
        let Ok(base_kind) = usize::try_from(definition.base_kind) else {
            return false;
        };
        if kind >= BACKEND_CAPACITY || base_kind >= COMPACT_NATIVE_LEN {
            return false;
        }
        let Some(compact_slot) = self.reserve_compact_slot() else {
            return false;
        };

        self.name_upper.0[kind].store(
            definition.resource_name_upper_cstr.as_ptr() as usize,
            Ordering::Release,
        );
        self.name_lower.0[kind].store(
            definition.resource_name_cstr.as_ptr() as usize,
            Ordering::Release,
        );
        self.name_title.0[kind].store(
            definition.resource_name_title_cstr.as_ptr() as usize,
            Ordering::Release,
        );
        let fighter_hash = hash40(definition.fighter_kind_name);
        self.kind_hash.0[kind].store(fighter_hash, Ordering::Release);
        self.class_descriptor.0[kind].store(
            self.class_descriptor.0[base_kind].load(Ordering::Acquire),
            Ordering::Release,
        );
        self.compact_hash.0[compact_slot].store(fighter_hash, Ordering::Release);
        true
    }
}

static STATUS: AtomicU32 = AtomicU32::new(BACKEND_STATUS_COMPILED);
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static TABLES: OnceLock<NativeTables> = OnceLock::new();

fn is_hook_instruction(opcode: u32) -> bool {
    if matches!(opcode >> 26, 0b000101 | 0b100101) {
        return true;
    }
    if opcode & 0xFF00001E == 0x58000010 {
        return true;
    }
    matches!(opcode, 0xD61F0200 | 0xD61F0220)
}

pub(crate) fn is_foreign_hook(opcode: u32) -> bool {
    is_hook_instruction(opcode)
}

unsafe fn belongs_to_hook_stub(offset: usize) -> bool {
    let base = text_base();
    (0..=3).any(|back| {
        offset >= back * 4 && {
            let word = core::ptr::read_volatile((base + offset - back * 4) as *const u32);
            is_hook_instruction(word)
        }
    })
}

pub(crate) struct Preflight {
    pub(crate) matched: usize,
    pub(crate) foreign: usize,
    pub(crate) unexpected: usize,
}

impl Preflight {
    pub(crate) fn is_audited_image(&self) -> bool {
        let total = self.matched + self.foreign + self.unexpected;
        self.unexpected == 0 && total > 0 && self.matched * 100 >= total * 95
    }
}

pub(crate) fn preflight_report() -> Preflight {
    let base = text_base();
    let mut report = Preflight {
        matched: 0,
        foreign: 0,
        unexpected: 0,
    };
    for expected in EXPECTED_OPCODES {
        let actual = unsafe { core::ptr::read_volatile((base + expected.offset) as *const u32) };
        if actual == expected.opcode {
            report.matched += 1;
        } else if is_foreign_hook(actual) || unsafe { belongs_to_hook_stub(expected.offset) } {
            if report.foreign < 8 {
                skyline::println!(
                    "[native_tables] foreign hook at {:#x} ({}): {:#010x} replaces {:#010x}",
                    expected.offset,
                    expected.label,
                    actual,
                    expected.opcode
                );
            }
            report.foreign += 1;
        } else {
            skyline::println!(
                "[native_tables] UNEXPECTED opcode at {:#x} ({}): expected {:#010x}, got {:#010x}",
                expected.offset,
                expected.label,
                expected.opcode,
                actual
            );
            report.unexpected += 1;
        }
    }
    skyline::println!(
        "[native_tables] preflight: {} matched, {} foreign hooks, {} unexpected",
        report.matched,
        report.foreign,
        report.unexpected
    );
    report
}

static IMAGE_VERDICT: AtomicU32 = AtomicU32::new(0);

pub(crate) fn validate_runtime_opcodes() -> bool {
    match IMAGE_VERDICT.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let verified = preflight_report().is_audited_image();
    IMAGE_VERDICT.store(if verified { 1 } else { 2 }, Ordering::Release);
    verified
}

pub(crate) fn initialize() {
    if INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }
    skyline::println!(
        "[native_tables] feature compiled: capacity={} manifest_sha256={} anchors={}",
        BACKEND_CAPACITY,
        MANIFEST_SHA256,
        EXPECTED_OPCODES.len()
    );
    if !validate_runtime_opcodes() {
        skyline::println!("[native_tables] disabled: exact 13.0.4 opcode preflight failed");
        return;
    }
    STATUS.fetch_or(BACKEND_STATUS_STATIC_PREFLIGHT_OK, Ordering::AcqRel);

    let tables = unsafe { NativeTables::copy_from_game() };
    if TABLES.set(tables).is_err() {
        skyline::println!("[native_tables] disabled: static table publication raced");
        return;
    }
    STATUS.fetch_or(BACKEND_STATUS_STATIC_TABLES_READY, Ordering::AcqRel);

    if RESOURCE_LAYOUT_PROVEN {
        STATUS.fetch_or(BACKEND_STATUS_RESOURCE_LAYOUT_PROVEN, Ordering::AcqRel);
    }
    if RESOURCE_LIFECYCLE_PROVEN {
        STATUS.fetch_or(BACKEND_STATUS_RESOURCE_LIFECYCLE_PROVEN, Ordering::AcqRel);
    }
    if !RESOURCE_LAYOUT_PROVEN || !RESOURCE_LIFECYCLE_PROVEN {
        skyline::println!(
            "[native_tables] fail-closed: static tables are ready, but the manifest does not yet prove the 0xc88 resource block's layout and lifecycle; kinds {}..={} remain unavailable",
            FIRST_NATIVE_BACKEND_KIND,
            LAST_NATIVE_BACKEND_KIND
        );
        return;
    }

    let _required_before_ready = BACKEND_STATUS_HOOKS_INSTALLED | BACKEND_STATUS_READY;
    skyline::println!(
        "[native_tables] resource proof passed, but redirect hooks are not implemented"
    );
}

pub(crate) fn publish_descriptor(definition: &CloneDefinition) -> bool {
    let Some(tables) = TABLES.get() else {
        return definition.kind <= super::MAX_PROVEN_CUSTOM_KIND;
    };
    tables.publish(definition)
}

pub(crate) fn published_lower_name_table_base(definition: &CloneDefinition) -> Option<usize> {
    let kind = usize::try_from(definition.kind).ok()?;
    let tables = TABLES.get()?;
    if kind >= BACKEND_CAPACITY {
        return None;
    }
    let expected = definition.resource_name_cstr.as_ptr() as usize;
    if tables.name_lower.0[kind].load(Ordering::Acquire) != expected {
        return None;
    }
    Some(tables.name_lower.0.as_ptr() as usize)
}

pub(crate) fn status() -> u32 {
    STATUS.load(Ordering::Acquire)
}

pub(crate) const fn last_supported_kind() -> i32 {
    LAST_NATIVE_BACKEND_KIND
}
