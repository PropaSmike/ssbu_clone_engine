use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

const SLOT_SIZE: usize = 0xF0;
const SLOT_STRIDE: usize = 0xF0;
const SLOT_BASE_OFFSET: usize = 0x28;
const SCRIPT_GROUP_RESOURCE: usize = 0x24;

const SCRIPT_CONTROL_KINDS: usize = 2;

const SCRIPT_FACTORY_VTABLE_SLOT: usize = 0x30;

const ITEM_DESCRIPTOR_TABLE: usize = 0x5070FC8;
const ITEM_DESCRIPTOR_NAME_OFFSET: usize = 0x18;
const SLOT_INIT_WORD_04: u32 = 0xFFFF_FFFF;
const SLOT_INIT_WORD_0C: u32 = 0x681;
const SLOT_INIT_ONE_FLOAT: u32 = 0x3F80_0000;
const SLOT_INIT_ONE_FLOAT_OFFSETS: &[usize] = &[0x98, 0xC0, 0xE8];
const SLOT_CONTAINER_OFFSET: usize = 0x10;
const SLOT_DATA_VECTORS: &[usize] = &[0x30];
const SLOT_RESOURCE_VECTOR: usize = 0x50;
const SLOT_RESOURCE_POINTERS: usize = 2;
const SLOT_KEY_OFFSET: usize = 0x04;
const SLOT_CONFIG_BYTES: &[usize] = &[0x08, 0x0C, 0x0D, 0x4D];

#[repr(C, align(16))]
pub(crate) struct CloneSlot([u8; SLOT_SIZE]);

impl CloneSlot {
    fn boxed() -> Box<Self> {
        let mut slot = Box::new(CloneSlot([0u8; SLOT_SIZE]));
        unsafe { CloneSlot::init(slot.0.as_mut_ptr()) };
        slot
    }

    unsafe fn init(base: *mut u8) {
        core::ptr::write_bytes(base, 0, SLOT_SIZE);
        {
            (base.add(0x04) as *mut u32).write_unaligned(SLOT_INIT_WORD_04);
            (base.add(0x0C) as *mut u32).write_unaligned(SLOT_INIT_WORD_0C);
            for offset in SLOT_INIT_ONE_FLOAT_OFFSETS {
                (base.add(*offset) as *mut u32).write_unaligned(SLOT_INIT_ONE_FLOAT);
            }
        }
    }

    fn address(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

#[derive(Clone, Copy)]
struct BasenameSite {
    load: usize,
    hook: usize,
    expected_load: u32,
    expected_window: [u32; 5],
    row_register: u8,
    register: u8,
}

#[derive(Clone, Copy)]
struct SlotSite {
    offset: usize,
    expected: u32,
    expected_window: [u32; 5],
    kind_register: u8,
    base_register: u8,
}

const BASENAME_SITES: &[BasenameSite] = &[
    BasenameSite {
        load: 0x15F874C,
        hook: 0x15F8750,
        expected_load: 0xF9400D36,
        expected_window: [0x93407F88, 0xD1029909, 0xF101CD3F, 0xD0016C17, 0x913702F7],
        row_register: 9,
        register: 22,
    },
    BasenameSite {
        load: 0x1605828,
        hook: 0x160582C,
        expected_load: 0xF9400D36,
        expected_window: [0x93407F28, 0xD1029909, 0xF101CD3F, 0xB0016BB7, 0x913702F7],
        row_register: 9,
        register: 22,
    },
];

const KNOWN_SLOT_SITE_COUNT: usize = 68;

const SLOT_SITES: &[SlotSite] = &[
    SlotSite {
        offset: 0x160ACB8,
        expected: 0x9B084F08,
        expected_window: [0x9B084F08, 0x3941D108, 0x35001B68, 0x321C0FE8, 0x9B084F17],
        kind_register: 24,
        base_register: 19,
    },
    SlotSite {
        offset: 0x160ACC8,
        expected: 0x9B084F17,
        expected_window: [0x9B084F17, 0xF8438EF5, 0xB40004F5, 0x321C0FE8, 0x9B084F16],
        kind_register: 24,
        base_register: 19,
    },
    SlotSite {
        offset: 0x160ACD8,
        expected: 0x9B084F16,
        expected_window: [0x9B084F16, 0xB8428EC8, 0x35000108, 0x321C0FE8, 0x9B084F08],
        kind_register: 24,
        base_register: 19,
    },
    SlotSite {
        offset: 0x160ACE8,
        expected: 0x9B084F08,
        expected_window: [0x9B084F08, 0xF90002F5, 0xB9402D01, 0xAA1503E0, 0x94077D8E],
        kind_register: 24,
        base_register: 19,
    },
    SlotSite {
        offset: 0x160AD10,
        expected: 0x9B084F08,
        expected_window: [0x9B084F08, 0x320003E9, 0x3901D109, 0x9001E5D5, 0x913DE2B5],
        kind_register: 24,
        base_register: 19,
    },
    SlotSite {
        offset: 0x4EBA80,
        expected: 0x9B092329,
        expected_window: [0x9B092329, 0xF9401D28, 0xB9402D2A, 0xF8410D09, 0xB4000209],
        kind_register: 25,
        base_register: 8,
    },
    SlotSite {
        offset: 0x134C854,
        expected: 0x9B164EB4,
        expected_window: [0x9B164EB4, 0xB8428E97, 0x710006FF, 0x540000AB, 0xAA1403E0],
        kind_register: 21,
        base_register: 19,
    },
    SlotSite {
        offset: 0x1482418,
        expected: 0x9B1856F6,
        expected_window: [0x9B1856F6, 0xB8428EDA, 0x7100075F, 0x540000AB, 0xAA1603E0],
        kind_register: 23,
        base_register: 21,
    },
    SlotSite {
        offset: 0x1482F7C,
        expected: 0x9B1856F6,
        expected_window: [0x9B1856F6, 0xB8428EDA, 0x7100075F, 0x540000AB, 0xAA1603E0],
        kind_register: 23,
        base_register: 21,
    },
    SlotSite {
        offset: 0x1486BE8,
        expected: 0x9B1856F6,
        expected_window: [0x9B1856F6, 0xB8428EDA, 0x7100075F, 0x540000AB, 0xAA1603E0],
        kind_register: 23,
        base_register: 21,
    },
    SlotSite {
        offset: 0x148E0B8,
        expected: 0x9B1856F6,
        expected_window: [0x9B1856F6, 0xB8428EDA, 0x7100075F, 0x540000AB, 0xAA1603E0],
        kind_register: 23,
        base_register: 21,
    },
    SlotSite {
        offset: 0x15A3FF8,
        expected: 0x9B086A88,
        expected_window: [0x9B086A88, 0x3941D508, 0x34000808, 0x321C0FE8, 0x9B086A88],
        kind_register: 20,
        base_register: 26,
    },
    SlotSite {
        offset: 0x15A4008,
        expected: 0x9B086A88,
        expected_window: [0x9B086A88, 0x3941C108, 0x34000568, 0x321C0FE8, 0x9B086A89],
        kind_register: 20,
        base_register: 26,
    },
    SlotSite {
        offset: 0x15A4018,
        expected: 0x9B086A89,
        expected_window: [0x9B086A89, 0x93407F2A, 0xA947A528, 0xCB080129, 0x9344FD29],
        kind_register: 20,
        base_register: 26,
    },
    SlotSite {
        offset: 0x15A4550,
        expected: 0x9B096A89,
        expected_window: [0x9B096A89, 0xF8490D2C, 0xB4FFD78C, 0x321C0FE8, 0x9B086A88],
        kind_register: 20,
        base_register: 26,
    },
    SlotSite {
        offset: 0x15A4560,
        expected: 0x9B086A88,
        expected_window: [0x9B086A88, 0xF90023EC, 0xF8498D0A, 0xF90027EA, 0xB40002EA],
        kind_register: 20,
        base_register: 26,
    },
    SlotSite {
        offset: 0x15D3D7C,
        expected: 0x9B165275,
        expected_window: [0x9B165275, 0xB8428EB7, 0x710006FF, 0x540000AB, 0xAA1503E0],
        kind_register: 19,
        base_register: 20,
    },
    SlotSite {
        offset: 0x15D3F14,
        expected: 0x9B145E76,
        expected_window: [0x9B145E76, 0xB8428ED5, 0x710006BF, 0x540000AB, 0xAA1603E0],
        kind_register: 19,
        base_register: 23,
    },
    SlotSite {
        offset: 0x15DB6B8,
        expected: 0x9B092348,
        expected_window: [0x9B092348, 0xF9401D00, 0xB40001E0, 0xF9400008, 0xF9401908],
        kind_register: 26,
        base_register: 8,
    },
    SlotSite {
        offset: 0x15DB6E0,
        expected: 0x9B092348,
        expected_window: [0x9B092348, 0xF9401D00, 0xB40000A0, 0xF9400008, 0xF9401908],
        kind_register: 26,
        base_register: 8,
    },
    SlotSite {
        offset: 0x15DB718,
        expected: 0x9B182348,
        expected_window: [0x9B182348, 0xF9401D00, 0xB4FFFF00, 0xF9400008, 0xF9401908],
        kind_register: 26,
        base_register: 8,
    },
    SlotSite {
        offset: 0x15DB77C,
        expected: 0x9B086348,
        expected_window: [0x9B086348, 0x9100A100, 0x2A1F03E1, 0x9400B20E, 0xF001E739],
        kind_register: 26,
        base_register: 24,
    },
    SlotSite {
        offset: 0x15DB874,
        expected: 0x9B0E6148,
        expected_window: [0x9B0E6148, 0x9100A100, 0x2A1F03E1, 0x9400B1D0, 0xA9402728],
        kind_register: 10,
        base_register: 24,
    },
    SlotSite {
        offset: 0x160A90C,
        expected: 0x9B084E88,
        expected_window: [0x9B084E88, 0x9100A100, 0xA9437BFD, 0xA9424FF4, 0xA94157F6],
        kind_register: 20,
        base_register: 19,
    },
    SlotSite {
        offset: 0x16216A0,
        expected: 0x9B0A2509,
        expected_window: [0x9B0A2509, 0xF9401D28, 0xB9402D2A, 0xF8410D09, 0xB4000209],
        kind_register: 8,
        base_register: 9,
    },
    SlotSite {
        offset: 0x1621764,
        expected: 0x9B0A2509,
        expected_window: [0x9B0A2509, 0xF9401D28, 0xB9402D2A, 0xF8410D09, 0xB4000209],
        kind_register: 8,
        base_register: 9,
    },
    SlotSite {
        offset: 0x1621820,
        expected: 0x9B0A2509,
        expected_window: [0x9B0A2509, 0xF9401D28, 0xB9402D2A, 0xF8410D09, 0xB4000209],
        kind_register: 8,
        base_register: 9,
    },
    SlotSite {
        offset: 0x1621EDC,
        expected: 0x9B0A2509,
        expected_window: [0x9B0A2509, 0xF9401D28, 0xB9402D2A, 0xF8410D09, 0xB4000209],
        kind_register: 8,
        base_register: 9,
    },
    SlotSite {
        offset: 0x22D28A4,
        expected: 0x9B1C6288,
        expected_window: [0x9B1C6288, 0xF9401D00, 0xB4FFFE80, 0xF9400008, 0xF9401908],
        kind_register: 20,
        base_register: 24,
    },
    SlotSite {
        offset: 0x22D29C4,
        expected: 0x9B1C6288,
        expected_window: [0x9B1C6288, 0x9100A100, 0x2A1F03E1, 0x97CCD57C, 0x90017F88],
        kind_register: 20,
        base_register: 24,
    },
    SlotSite {
        offset: 0x22D2BC4,
        expected: 0x9B1C6148,
        expected_window: [0x9B1C6148, 0x9100A100, 0x2A1F03E1, 0x97CCD4FC, 0x90017F88],
        kind_register: 10,
        base_register: 24,
    },
    SlotSite {
        offset: 0x24CF66C,
        expected: 0x9B092148,
        expected_window: [0x9B092148, 0xF9401D00, 0xB4001860, 0xF9400008, 0xF9401908],
        kind_register: 10,
        base_register: 8,
    },
    SlotSite {
        offset: 0x24CF6C0,
        expected: 0x9B092148,
        expected_window: [0x9B092148, 0xF9401D00, 0xB40015C0, 0xF9400008, 0xF9401908],
        kind_register: 10,
        base_register: 8,
    },
    SlotSite {
        offset: 0x24CF714,
        expected: 0x9B092148,
        expected_window: [0x9B092148, 0xF9401D00, 0xB4001320, 0xF9400008, 0xF9401908],
        kind_register: 10,
        base_register: 8,
    },
    SlotSite {
        offset: 0x24CF768,
        expected: 0x9B092148,
        expected_window: [0x9B092148, 0xF9401D00, 0xB4001080, 0xF9400008, 0xF9401908],
        kind_register: 10,
        base_register: 8,
    },
    SlotSite {
        offset: 0x24CF7BC,
        expected: 0x9B0A2508,
        expected_window: [0x9B0A2508, 0xF9401D00, 0xB4000DE0, 0xF9400008, 0xF9401908],
        kind_register: 8,
        base_register: 9,
    },
];

const ROW_SITE: SlotSite = SlotSite {
    offset: 0x17E1270,
    expected: 0x8B091549,
    expected_window: [0x8B091549, 0xF9400D24, 0xF0016889, 0x91229129, 0xB8A87928],
    kind_register: 9,
    base_register: 10,
};

const PATH_CATEGORY_SITE: usize = 0x17E1258;
const PATH_CATEGORY_EXPECTED: u32 = 0x7100245F;

const OFF_ITEM_POPULATE: usize = 0x160AC80;
const OFF_ITEM_ACQUIRE: usize = 0x1607FC0;
const ITEM_MANAGER_GLOBAL: usize = 0x52C3498;
const ITEM_RESOURCE_MANAGER_GLOBAL: usize = 0x5323680;
const ITEM_RESOURCE_CONTAINER_OFFSET: usize = 0x78;
const MATCH_LOAD_TAIL: usize = 0x15D5604;
const MATCH_LOAD_TAIL_EXPECTED: u32 = 0xF106C27F;
const POST_ACQUIRE: usize = 0x15DB790;
const POST_ACQUIRE_EXPECTED: u32 = 0x913DE339;

const ITEM_PATH_KIND_REGISTER: usize = 1;
const ITEM_PATH_CATEGORY_REGISTER: usize = 2;
const ITEM_PATH_NAMESPACE_REGISTER: usize = 3;
const ITEM_PATH_BASENAME_REGISTER: usize = 4;
const PATH_PROBE_LIMIT: usize = 240;

const CONTAINER_INSERT: usize = 0x17E2E68;
const CONTAINER_INSERT_EXPECTED: u32 = 0x32005FE8;
const CONTAINER_INSERT_KEY_REGISTER: usize = 21;
const CONTAINER_INSERT_INDEX_REGISTER: usize = 23;

const ACQUIRE_MISS: usize = 0x1608040;
const ACQUIRE_MISS_EXPECTED: u32 = 0xB2005FE8;
const ACQUIRE_HIT: usize = 0x1608240;
const ACQUIRE_HIT_EXPECTED: u32 = 0xB9402E81;
const MATCH_LOAD_COUNTER: usize = 19;
const ITEM_KIND_TERM: u64 = 0x1B0;

const SIBLING_SITES: &[SlotSite] = &[
    SlotSite {
        offset: 0x160AE24,
        expected: 0x9B184F5B,
        expected_window: [0x9B184F5B, 0xF94002E0, 0xB8428F68, 0x350000C8, 0x9B184F48],
        kind_register: 26,
        base_register: 19,
    },
    SlotSite {
        offset: 0x160AE34,
        expected: 0x9B184F48,
        expected_window: [0x9B184F48, 0xB9402D01, 0xF9001D00, 0x94077D3C, 0xB9400368],
        kind_register: 26,
        base_register: 19,
    },
    SlotSite {
        offset: 0x160AE50,
        expected: 0x9B184F48,
        expected_window: [0x9B184F48, 0x3901D119, 0xA94026A8, 0xEB08012B, 0x54000E60],
        kind_register: 26,
        base_register: 19,
    },
];

const ROW_STRIDE: usize = 0x20;

const NATIVE_ITEM_KIND_COUNT: usize = 432;

#[repr(C, align(16))]
struct CloneRow([u8; ROW_STRIDE]);

pub(crate) fn rebased_row_operand(row: usize, kind: i32) -> usize {
    row.wrapping_sub((kind as usize).wrapping_mul(ROW_STRIDE))
}

static PREFLIGHT_OK: AtomicBool = AtomicBool::new(false);
static HOOKS_INSTALLED: AtomicBool = AtomicBool::new(false);
static ROUTER_READY: AtomicBool = AtomicBool::new(false);
static SLOT_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn rebased_slot_operand(slot: usize, kind: i32) -> usize {
    slot.wrapping_sub(SLOT_BASE_OFFSET)
        .wrapping_sub((kind as usize).wrapping_mul(SLOT_STRIDE))
}

#[derive(Debug)]
enum PreflightError {
    Geometry,
    Opcode {
        offset: usize,
        expected: u32,
        actual: u32,
    },
    Register {
        offset: usize,
    },
}

pub(crate) unsafe fn text_word(offset: usize) -> u32 {
    core::ptr::read_volatile((crate::text_base() + offset) as *const u32)
}

unsafe fn expect_word(offset: usize, expected: u32) -> Result<(), PreflightError> {
    let actual = text_word(offset);
    if actual == expected {
        Ok(())
    } else {
        Err(PreflightError::Opcode {
            offset,
            expected,
            actual,
        })
    }
}

fn add_shifted_operands(opcode: u32) -> Option<(u8, u8, u8)> {
    if opcode & 0x7F200000 != 0x0B000000 || opcode & 0x80000000 == 0 {
        return None;
    }
    Some((
        (opcode & 0x1F) as u8,
        ((opcode >> 5) & 0x1F) as u8,
        ((opcode >> 16) & 0x1F) as u8,
    ))
}

fn madd_operands(opcode: u32) -> Option<(u8, u8, u8, u8)> {
    if opcode & 0xFFE08000 != 0x9B000000 {
        return None;
    }
    let rd = (opcode & 0x1F) as u8;
    let rn = ((opcode >> 5) & 0x1F) as u8;
    let rm = ((opcode >> 16) & 0x1F) as u8;
    let ra = ((opcode >> 10) & 0x1F) as u8;
    Some((rd, rn, rm, ra))
}

unsafe fn preflight() -> Result<(), PreflightError> {
    if SLOT_SIZE != SLOT_STRIDE || core::mem::size_of::<CloneSlot>() != SLOT_SIZE {
        return Err(PreflightError::Geometry);
    }
    for site in BASENAME_SITES {
        expect_word(site.load, site.expected_load)?;
        if (site.expected_load & 0x1F) as u8 != site.register {
            return Err(PreflightError::Register { offset: site.load });
        }
        for (index, expected) in site.expected_window.iter().copied().enumerate() {
            expect_word(site.hook + index * 4, expected)?;
        }
    }
    expect_word(ROW_SITE.offset, ROW_SITE.expected)?;
    let Some((_, row_rn, row_rm)) = add_shifted_operands(ROW_SITE.expected) else {
        return Err(PreflightError::Register {
            offset: ROW_SITE.offset,
        });
    };
    if (ROW_SITE.kind_register != row_rn && ROW_SITE.kind_register != row_rm)
        || ROW_SITE.base_register != row_rn
    {
        return Err(PreflightError::Register {
            offset: ROW_SITE.offset,
        });
    }
    for (index, expected) in ROW_SITE.expected_window.iter().copied().enumerate() {
        expect_word(ROW_SITE.offset + index * 4, expected)?;
    }
    expect_word(PATH_CATEGORY_SITE, PATH_CATEGORY_EXPECTED)?;
    expect_word(MATCH_LOAD_TAIL, MATCH_LOAD_TAIL_EXPECTED)?;
    expect_word(POST_ACQUIRE, POST_ACQUIRE_EXPECTED)?;
    expect_word(CONTAINER_INSERT, CONTAINER_INSERT_EXPECTED)?;
    expect_word(ACQUIRE_MISS, ACQUIRE_MISS_EXPECTED)?;
    expect_word(ACQUIRE_HIT, ACQUIRE_HIT_EXPECTED)?;
    for site in SLOT_SITES.iter().chain(SIBLING_SITES) {
        expect_word(site.offset, site.expected)?;
        let Some((_, rn, rm, ra)) = madd_operands(site.expected) else {
            return Err(PreflightError::Register {
                offset: site.offset,
            });
        };
        if (site.kind_register != rn && site.kind_register != rm)
            || site.base_register != ra
            || ra == 31
        {
            return Err(PreflightError::Register {
                offset: site.offset,
            });
        }
        for (index, expected) in site.expected_window.iter().copied().enumerate() {
            expect_word(site.offset + index * 4, expected)?;
        }
    }
    Ok(())
}

struct CloneResource {
    public_kind: i32,
    base_kind: i32,
    resource_name: *const core::ffi::c_char,
    slot: Box<CloneSlot>,
    row: Box<CloneRow>,
}

unsafe impl Send for CloneResource {}
unsafe impl Sync for CloneResource {}

fn resources() -> &'static std::sync::RwLock<Vec<CloneResource>> {
    static RESOURCES: std::sync::OnceLock<std::sync::RwLock<Vec<CloneResource>>> =
        std::sync::OnceLock::new();
    RESOURCES.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

#[derive(Clone, Copy)]
pub(crate) struct CloneResourceRegistration {
    pub public_kind: i32,
    pub base_kind: i32,
    pub resource_name: *const core::ffi::c_char,
}

pub(crate) fn can_register_family(entries: &[CloneResourceRegistration]) -> bool {
    if entries.is_empty()
        || !PREFLIGHT_OK.load(Ordering::Acquire)
        || entries.iter().any(|entry| {
            entry.resource_name.is_null()
                || !(0..NATIVE_ITEM_KIND_COUNT as i32).contains(&entry.base_kind)
        })
    {
        return false;
    }
    let Ok(resources) = resources().read() else {
        return false;
    };
    !entries.iter().enumerate().any(|(index, entry)| {
        resources
            .iter()
            .any(|resource| resource.public_kind == entry.public_kind)
            || entries[..index]
                .iter()
                .any(|earlier| earlier.public_kind == entry.public_kind)
    })
}

pub(crate) fn register(
    public_kind: i32,
    base_kind: i32,
    resource_name: *const core::ffi::c_char,
) -> bool {
    register_family(&[CloneResourceRegistration {
        public_kind,
        base_kind,
        resource_name,
    }])
}

pub(crate) fn register_family(entries: &[CloneResourceRegistration]) -> bool {
    if entries.is_empty() {
        return false;
    }
    if !PREFLIGHT_OK.load(Ordering::Acquire) {
        crate::dbg_log_public(&format!(
            "[itemslot] REFUSED family owner {:#x}: preflight has not run yet. \
             `item_slots::install()` must precede `item_clones::install()`",
            entries[0].public_kind
        ));
        return false;
    }
    if entries.iter().any(|entry| entry.resource_name.is_null()) {
        crate::dbg_log_public("[itemslot] REFUSED family: null resource name");
        return false;
    }
    let Ok(mut resources) = resources().write() else {
        crate::dbg_log_public("[itemslot] REFUSED family: resource lock poisoned");
        return false;
    };
    if entries.iter().enumerate().any(|(index, entry)| {
        resources
            .iter()
            .any(|resource| resource.public_kind == entry.public_kind)
            || entries[..index]
                .iter()
                .any(|earlier| earlier.public_kind == entry.public_kind)
    }) {
        crate::dbg_log_public("[itemslot] REFUSED family: duplicate public kind");
        return false;
    }

    let mut staged = Vec::with_capacity(entries.len());
    for entry in entries {
        if !(0..NATIVE_ITEM_KIND_COUNT as i32).contains(&entry.base_kind) {
            crate::dbg_log_public(&format!(
                "[itemslot] REFUSED family member {:#x}: invalid base {:#x}",
                entry.public_kind, entry.base_kind
            ));
            return false;
        }
        let slot = CloneSlot::boxed();
        let mut row = Box::new(CloneRow([0u8; ROW_STRIDE]));
        unsafe {
            let source = (crate::text_base()
                + ITEM_DESCRIPTOR_TABLE
                + entry.base_kind as usize * ROW_STRIDE) as *const u8;
            core::ptr::copy_nonoverlapping(source, row.0.as_mut_ptr(), ROW_STRIDE);
            (row.0.as_mut_ptr().add(ITEM_DESCRIPTOR_NAME_OFFSET) as *mut *const core::ffi::c_char)
                .write(entry.resource_name);
        }
        staged.push(CloneResource {
            public_kind: entry.public_kind,
            base_kind: entry.base_kind,
            resource_name: entry.resource_name,
            slot,
            row,
        });
    }

    for resource in &staged {
        crate::dbg_log_public(&format!(
            "[itemslot] clone {:#x} (base {:#x}) owns slot {:#x} row {:#x}",
            resource.public_kind,
            resource.base_kind,
            resource.slot.address(),
            resource.row.0.as_ptr() as usize
        ));
    }
    resources.extend(staged);
    SLOT_COUNT.store(resources.len(), Ordering::Release);
    true
}

unsafe fn slot_is_populated(slot: usize, native_slot: usize) -> bool {
    let word = |at: usize, offset: usize| core::ptr::read_volatile((at + offset) as *const usize);
    let container = word(slot, SLOT_CONTAINER_OFFSET);
    if container == 0 {
        return false;
    }
    let native = word(native_slot, SLOT_CONTAINER_OFFSET);
    if native == 0 || container != native {
        return false;
    }
    if !SLOT_DATA_VECTORS
        .iter()
        .all(|offset| word(slot, *offset) != 0)
    {
        return false;
    }
    let vec50 = word(slot, SLOT_RESOURCE_VECTOR);
    let vec50_end = word(slot, SLOT_RESOURCE_VECTOR + 8);
    if vec50_end.saturating_sub(vec50) < SLOT_RESOURCE_POINTERS * 8 {
        return false;
    }
    (0..SLOT_RESOURCE_POINTERS).all(|step| word(vec50, step * 8) != 0)
}

fn active_clone() -> Option<(usize, i32)> {
    let (_, slot, base_kind, populated) = active_clone_entry()?;
    populated.then_some((slot, base_kind))
}

fn active_clone_entry() -> Option<(i32, usize, i32, bool)> {
    let forced = FORCED_CLONE.load(Ordering::Acquire);
    let public_kind = redirect_target()?;
    let resources = resources().read().ok()?;
    let resource = resources
        .iter()
        .find(|resource| resource.public_kind == public_kind)?;
    let slot = resource.slot.address();
    let native_slot = unsafe {
        let manager = item_manager();
        if manager == 0 {
            0
        } else {
            manager + resource.base_kind as usize * SLOT_STRIDE + SLOT_BASE_OFFSET
        }
    };
    let populated =
        forced >= 0 || (native_slot != 0 && unsafe { slot_is_populated(slot, native_slot) });
    report_decision(public_kind, slot, native_slot, populated);
    Some((public_kind, slot, resource.base_kind, populated))
}

static FORCED_CLONE: AtomicI32 = AtomicI32::new(-1);

pub(crate) fn force_clone(public_kind: i32) {
    FORCED_CLONE.store(public_kind, Ordering::Release);
}

pub(crate) fn clear_forced_clone() {
    FORCED_CLONE.store(-1, Ordering::Release);
}

unsafe fn report_slot_difference(manager: usize, base_kind: i32, clone_slot: usize) {
    let native = manager + base_kind as usize * SLOT_STRIDE + SLOT_BASE_OFFSET;
    let byte = |slot: usize, offset: usize| core::ptr::read_volatile((slot + offset) as *const u8);
    let mut parts: Vec<String> = Vec::new();
    for offset in 0..SLOT_SIZE {
        let (mine, theirs) = (byte(clone_slot, offset), byte(native, offset));
        if mine != theirs {
            parts.push(format!("{offset:#04x}={mine:02x}/{theirs:02x}"));
        }
    }
    for (index, chunk) in parts.chunks(24).enumerate() {
        crate::dbg_log_public(&format!(
            "[itemslot] slot diff base {base_kind:#x} #{index} (clone/native): {}",
            chunk.join(" ")
        ));
    }
    if parts.is_empty() {
        crate::dbg_log_public(&format!(
            "[itemslot] slot diff base {base_kind:#x}: byte-identical to the loaded native slot"
        ));
    }
}

fn acquired() -> &'static std::sync::RwLock<Vec<i32>> {
    static ACQUIRED: std::sync::OnceLock<std::sync::RwLock<Vec<i32>>> = std::sync::OnceLock::new();
    ACQUIRED.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

fn claim_acquire(public_kind: i32) -> bool {
    let Ok(mut acquired) = acquired().write() else {
        return false;
    };
    if acquired.contains(&public_kind) {
        return false;
    }
    acquired.push(public_kind);
    true
}

const SCRIPT_REGION: core::ops::Range<usize> = 0x70..0xF0;
const SCRIPT_DUMP_DEADLINE: usize = 240;

static CLONE_LUA_STATE: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];
static BASE_LUA_STATE: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];

pub(crate) fn script_lua_states(public_kind: i32) -> Option<(usize, usize)> {
    let resources = resources().read().ok()?;
    let index = resources
        .iter()
        .position(|resource| resource.public_kind == public_kind)?;
    let clone = CLONE_LUA_STATE.get(index)?.load(Ordering::Acquire);
    let base = BASE_LUA_STATE.get(index)?.load(Ordering::Acquire);
    (clone != 0 && base != 0).then_some((clone, base))
}

static CLONE_SCRIPT_ITEM: AtomicUsize = AtomicUsize::new(0);
static VANILLA_SCRIPT_ITEM: AtomicUsize = AtomicUsize::new(0);

unsafe fn item_manager() -> usize {
    let holder =
        core::ptr::read_volatile((crate::text_base() + ITEM_MANAGER_GLOBAL) as *const usize);
    if holder == 0 {
        return 0;
    }
    core::ptr::read_volatile(holder as *const usize)
}

#[skyline::from_offset(OFF_ITEM_POPULATE)]
fn item_populate(manager: usize, kind: i32, flag: u32);

#[skyline::from_offset(OFF_ITEM_ACQUIRE)]
fn item_acquire(slot: usize, flag: u32);

unsafe fn load_clone_slots() {
    let manager = item_manager();
    if manager == 0 {
        crate::dbg_log_public("[itemslot] match load reached with no ItemManager; nothing loaded");
        return;
    }
    let pending: Vec<(i32, i32, usize)> = {
        let Ok(resources) = resources().read() else {
            return;
        };
        resources
            .iter()
            .map(|resource| {
                (
                    resource.public_kind,
                    resource.base_kind,
                    resource.slot.address(),
                )
            })
            .collect()
    };
    if let Ok(mut acquired) = acquired().write() {
        acquired.clear();
    }
    for (public_kind, base_kind, slot) in pending {
        CloneSlot::init(slot as *mut u8);
        core::ptr::write_volatile((slot + SLOT_KEY_OFFSET) as *mut u32, public_kind as u32);
        let native = manager + base_kind as usize * SLOT_STRIDE + SLOT_BASE_OFFSET;
        for offset in SLOT_CONFIG_BYTES {
            let value = core::ptr::read_volatile((native + *offset) as *const u8);
            core::ptr::write_volatile((slot + *offset) as *mut u8, value);
        }
        FORCED_CLONE.store(public_kind, Ordering::Release);
        item_populate(manager, base_kind, 1);
        FORCED_CLONE.store(-1, Ordering::Release);
        let word = |offset: usize| core::ptr::read_volatile((slot + offset) as *const u32);
        let quad = |offset: usize| core::ptr::read_volatile((slot + offset) as *const usize);
        crate::dbg_log_public(&format!(
            "[itemslot] forced load: clone {public_kind:#x} (base {base_kind:#x}) slot {slot:#x} \
             refcount={:#x} key={:#x} flags={:#x} container={:#x} vec18=({:#x},{:#x}) \
             vec30=({:#x},{:#x}) vec50=({:#x},{:#x}) loaded={:#x} usable={}",
            word(0x00),
            word(SLOT_KEY_OFFSET),
            word(0x0C),
            quad(SLOT_CONTAINER_OFFSET),
            quad(0x18),
            quad(0x20),
            quad(0x30),
            quad(0x38),
            quad(0x50),
            quad(0x58),
            word(0x4C),
            slot_is_populated(slot, native),
        ));
        report_slot_difference(manager, base_kind, slot);
        crate::item_scripts::prepare(public_kind);
    }
}

fn report_decision(public_kind: i32, slot: usize, native_slot: usize, populated: bool) {
    static LAST: AtomicI32 = AtomicI32::new(i32::MIN);
    static SEEN: AtomicUsize = AtomicUsize::new(0);
    let state = (public_kind << 1) | populated as i32;
    if LAST.swap(state, Ordering::Relaxed) == state {
        return;
    }
    if SEEN.fetch_add(1, Ordering::Relaxed) >= DECISION_LOG_LIMIT {
        return;
    }
    if populated {
        crate::dbg_log_public(&format!(
            "[itemslot] clone {public_kind:#x} USING its own slot {slot:#x}"
        ));
        return;
    }
    crate::dbg_log_public(&format!(
        "[itemslot] clone {public_kind:#x} slot {slot:#x} not usable (container={:#x} live={:#x}          vec30={:#x}); using the base's slot",
        unsafe { core::ptr::read_volatile((slot + SLOT_CONTAINER_OFFSET) as *const usize) },
        unsafe {
            if native_slot == 0 {
                0
            } else {
                core::ptr::read_volatile((native_slot + SLOT_CONTAINER_OFFSET) as *const usize)
            }
        },
        unsafe { core::ptr::read_volatile((slot + 0x30) as *const usize) },
    ));
}

const DECISION_LOG_LIMIT: usize = 24;

fn redirect_target() -> Option<i32> {
    let forced = FORCED_CLONE.load(Ordering::Acquire);
    if forced >= 0 {
        return Some(forced);
    }
    crate::item_clones::active_public_kind()
}

fn active_basename() -> Option<*const core::ffi::c_char> {
    let public_kind = redirect_target()?;
    let resources = resources().read().ok()?;
    resources
        .iter()
        .find(|resource| resource.public_kind == public_kind)
        .map(|resource| resource.resource_name)
}

fn active_row() -> Option<(usize, i32, i32)> {
    let public_kind = redirect_target()?;
    let resources = resources().read().ok()?;
    resources
        .iter()
        .find(|resource| resource.public_kind == public_kind)
        .map(|resource| {
            (
                resource.row.0.as_ptr() as usize,
                resource.base_kind,
                resource.public_kind,
            )
        })
}

static SITE_MARKED: [AtomicBool; 40] = [const { AtomicBool::new(false) }; 40];

const MARK_ROW: usize = 0;
const MARK_BASENAME: usize = 1;
const MARK_SLOT: usize = 3;

fn mark(index: usize, what: &str, offset: usize) {
    let Some(cell) = SITE_MARKED.get(index) else {
        return;
    };
    if cell.swap(true, Ordering::Relaxed) {
        return;
    }
    crate::dbg_log_public(&format!("[itemslot] reached {what} site {offset:#x}"));
}

unsafe extern "C" fn descriptor_row(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some((row, base_kind, public_kind)) = active_row() else {
        return;
    };
    let kind = ctx.registers[ROW_SITE.kind_register as usize].x() as u32 as i32;
    if kind != base_kind && kind != public_kind {
        return;
    }
    mark(MARK_ROW, "row", ROW_SITE.offset);
    ctx.registers[ROW_SITE.base_register as usize].set_x(rebased_row_operand(row, kind) as u64);
}

unsafe extern "C" fn path_category(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some((public_kind, _, base_kind, _)) = active_clone_entry() else {
        return;
    };
    let incoming = ctx.registers[ITEM_PATH_KIND_REGISTER].x() as u32 as i32;
    if incoming != public_kind && incoming != base_kind {
        return;
    }
    let root = crate::item_params::item_content_category(public_kind).main_root_offset();
    ctx.registers[ITEM_PATH_NAMESPACE_REGISTER].set_x((crate::text_base() + root) as u64);
}

unsafe fn kind_of_descriptor_row(row: usize) -> Option<i32> {
    let table = crate::text_base() + ITEM_DESCRIPTOR_TABLE;
    let offset = row.checked_sub(table)?;
    if offset % ROW_STRIDE != 0 {
        return None;
    }
    let kind = offset / ROW_STRIDE;
    if kind < NATIVE_ITEM_KIND_COUNT {
        return Some(kind as i32);
    }
    let kind = i32::try_from(kind).ok()?;
    resources()
        .read()
        .ok()?
        .iter()
        .any(|resource| resource.public_kind == kind)
        .then_some(kind)
}

unsafe fn apply_basename(ctx: &mut skyline::hooks::InlineCtx, index: usize) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(site) = BASENAME_SITES.get(index) else {
        return;
    };
    let Some(name) = active_basename() else {
        return;
    };
    let Some((public_kind, _, base_kind, _)) = active_clone_entry() else {
        return;
    };
    let row = ctx.registers[site.row_register as usize].x() as usize;
    match kind_of_descriptor_row(row) {
        Some(kind) if kind == base_kind || kind == public_kind => {}
        _ => return,
    }
    mark(MARK_BASENAME + index, "basename", site.hook);
    ctx.registers[site.register as usize].set_x(name as u64);
}

unsafe fn snapshot_native_slot(manager: usize, kind: i32) {
    static SNAPPED: AtomicBool = AtomicBool::new(false);
    if SNAPPED.swap(true, Ordering::Relaxed) {
        return;
    }
    let slot = manager + kind as usize * SLOT_STRIDE + SLOT_BASE_OFFSET;
    let word = |offset: usize| core::ptr::read_volatile((slot + offset) as *const u32);
    let quad = |offset: usize| core::ptr::read_volatile((slot + offset) as *const usize);
    crate::dbg_log_public(&format!(
        "[itemslot] native slot for kind {kind:#x} at {slot:#x}: refcount={:#x} key={:#x} \
         flags={:#x} container={:#x} vec18=({:#x},{:#x}) vec30=({:#x},{:#x}) vec50=({:#x},{:#x}) \
         loaded={:#x}",
        word(0x00),
        word(0x04),
        word(0x0C),
        quad(0x10),
        quad(0x18),
        quad(0x20),
        quad(0x30),
        quad(0x38),
        quad(0x50),
        quad(0x58),
        word(0x4C),
    ));
}

unsafe fn apply_slot(ctx: &mut skyline::hooks::InlineCtx, index: usize) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(site) = SLOT_SITES.get(index) else {
        return;
    };
    let Some((_, slot, base_kind, populated)) = active_clone_entry() else {
        return;
    };
    let kind = ctx.registers[site.kind_register as usize].x() as u32 as i32;
    if kind != base_kind {
        return;
    }
    if !populated {
        snapshot_native_slot(
            ctx.registers[site.base_register as usize].x() as usize,
            kind,
        );
        return;
    }
    mark(MARK_SLOT + index, "slot", site.offset);
    ctx.registers[site.base_register as usize].set_x(rebased_slot_operand(slot, kind) as u64);
}

unsafe fn apply_sibling(ctx: &mut skyline::hooks::InlineCtx, index: usize) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(site) = SIBLING_SITES.get(index) else {
        return;
    };
    let Some((_, slot, base_kind, populated)) = active_clone_entry() else {
        return;
    };
    if !populated {
        return;
    }
    let member = ctx.registers[site.kind_register as usize].x() as u32 as i32;
    let base = if member == base_kind {
        rebased_slot_operand(slot, member)
    } else {
        let native = item_manager();
        if native == 0 {
            return;
        }
        native
    };
    ctx.registers[site.base_register as usize].set_x(base as u64);
}

static BASE_KEEPER_ARMED: AtomicBool = AtomicBool::new(false);
static BASE_KEEPER_SEEN: AtomicUsize = AtomicUsize::new(0);
static BASE_KEEPER_RELOADS: AtomicUsize = AtomicUsize::new(0);
static BASE_KEEPER_INEFFECTIVE: AtomicUsize = AtomicUsize::new(0);
const BASE_KEEPER_REPORTS: usize = 24;
const BASE_KEEPER_RELOAD_REPORTS: usize = 24;
const BASE_KEEPER_GIVE_UP: usize = 16;

const SLOT_GROUP_FLAGS: [usize; 3] = [0x48, 0x49, 0x4B];

unsafe fn slot_flags(slot: usize) -> [u8; 6] {
    let mut out = [0u8; 6];
    for (index, offset) in (0x48usize..0x4E).enumerate() {
        out[index] = core::ptr::read_volatile((slot + offset) as *const u8);
    }
    out
}

unsafe fn slot_needs_reload(slot: usize) -> bool {
    if core::ptr::read_volatile((slot + SLOT_CONTAINER_OFFSET) as *const usize) == 0 {
        return true;
    }
    SLOT_GROUP_FLAGS
        .iter()
        .any(|offset| core::ptr::read_volatile((slot + *offset) as *const u8) == 0)
}

unsafe fn reacquire(slot: usize, forced: i32) -> ([u8; 6], [u8; 6]) {
    let before = slot_flags(slot);
    let saved = FORCED_CLONE.swap(forced, Ordering::AcqRel);
    item_acquire(slot, 0);
    FORCED_CLONE.store(saved, Ordering::Release);
    (before, slot_flags(slot))
}

unsafe fn keep_base_loaded(public_kind: i32, clone_slot: usize, base_kind: i32) {
    if !BASE_KEEPER_ARMED.swap(true, Ordering::AcqRel) {
        crate::dbg_log_public(
            "[itembase] keeper armed: every clone construction reports both slots",
        );
    }
    if !(0..NATIVE_ITEM_KIND_COUNT as i32).contains(&base_kind) {
        crate::dbg_log_public(&format!(
            "[itembase] clone {public_kind:#x}: base {base_kind:#x} is out of range; no keeper"
        ));
        return;
    }
    let manager = item_manager();
    if manager == 0 {
        crate::dbg_log_public(&format!(
            "[itembase] clone {public_kind:#x}: no ItemManager; keeper skipped"
        ));
        return;
    }
    let base_slot = manager + base_kind as usize * SLOT_STRIDE + SLOT_BASE_OFFSET;
    let clone_flags = slot_flags(clone_slot);
    let base_flags = slot_flags(base_slot);
    let clone_stale = slot_needs_reload(clone_slot);
    let base_stale = slot_needs_reload(base_slot);
    let seen = BASE_KEEPER_SEEN.fetch_add(1, Ordering::Relaxed);
    if seen < BASE_KEEPER_REPORTS {
        crate::dbg_log_public(&format!(
            "[itembase] #{seen} clone {public_kind:#x} base {base_kind:#x}: clone slot {clone_slot:#x} flags={clone_flags:02x?} stale={clone_stale}, base slot {base_slot:#x} flags={base_flags:02x?} stale={base_stale}"
        ));
    }
    if !clone_stale && !base_stale {
        return;
    }
    if BASE_KEEPER_INEFFECTIVE.load(Ordering::Acquire) >= BASE_KEEPER_GIVE_UP {
        return;
    }
    let reload = BASE_KEEPER_RELOADS.fetch_add(1, Ordering::Relaxed);
    let announce = reload < BASE_KEEPER_RELOAD_REPORTS;
    let mut recovered = true;
    if clone_stale {
        let (before, after) = reacquire(clone_slot, public_kind);
        if slot_needs_reload(clone_slot) {
            recovered = false;
        }
        if announce {
            crate::dbg_log_public(&format!(
                "[itembase] reload #{reload} clone {public_kind:#x} own slot {clone_slot:#x}: flags {before:02x?} -> {after:02x?}"
            ));
        }
    }
    if base_stale {
        let (before, after) = reacquire(base_slot, -1);
        if slot_needs_reload(base_slot) {
            recovered = false;
        }
        if announce {
            crate::dbg_log_public(&format!(
                "[itembase] reload #{reload} clone {public_kind:#x} base slot {base_slot:#x}: flags {before:02x?} -> {after:02x?}"
            ));
        }
    }
    if recovered {
        BASE_KEEPER_INEFFECTIVE.store(0, Ordering::Release);
        return;
    }
    let failures = BASE_KEEPER_INEFFECTIVE.fetch_add(1, Ordering::AcqRel) + 1;
    if failures == BASE_KEEPER_GIVE_UP {
        crate::dbg_log_public(&format!(
            "[itembase] GIVING UP after {failures} reloads that changed nothing; item_acquire does not rebuild a released slot mid-match"
        ));
    }
}

unsafe extern "C" fn post_acquire(_ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some((public_kind, slot, base_kind, _)) = active_clone_entry() else {
        return;
    };
    keep_base_loaded(public_kind, slot, base_kind);
    if !claim_acquire(public_kind) {
        return;
    }
    crate::dbg_log_public(&format!(
        "[itemslot] post-acquire clone {public_kind:#x}: acquiring slot {slot:#x}"
    ));
    FORCED_CLONE.store(public_kind, Ordering::Release);
    item_acquire(slot, 0);
    FORCED_CLONE.store(-1, Ordering::Release);
    let word = |offset: usize| core::ptr::read_volatile((slot + offset) as *const u32);
    let quad = |offset: usize| core::ptr::read_volatile((slot + offset) as *const usize);
    crate::dbg_log_public(&format!(
        "[itemslot] post-acquire clone {public_kind:#x} slot {slot:#x}: flags={:#x} \
         container={:#x} vec18=({:#x},{:#x}) vec30=({:#x},{:#x}) vec50=({:#x},{:#x}) \
         b08={:#x} b4d={:#x}",
        word(0x0C),
        quad(SLOT_CONTAINER_OFFSET),
        quad(0x18),
        quad(0x20),
        quad(0x30),
        quad(0x38),
        quad(0x50),
        quad(0x58),
        core::ptr::read_volatile((slot + 0x08) as *const u8),
        core::ptr::read_volatile((slot + 0x4D) as *const u8),
    ));
    let manager = item_manager();
    if manager == 0 {
        return;
    }
    let native_slot = manager + base_kind as usize * SLOT_STRIDE + SLOT_BASE_OFFSET;
    let elements = |at: usize, offset: usize| -> String {
        let begin = core::ptr::read_volatile((at + offset) as *const usize);
        let end = core::ptr::read_volatile((at + offset + 8) as *const usize);
        if begin == 0 || end <= begin || end - begin > 0x40 {
            return format!("<{begin:#x}..{end:#x}>");
        }
        let mut out = String::new();
        for step in (0..end - begin).step_by(4) {
            let value = core::ptr::read_volatile((begin + step) as *const u32);
            out.push_str(&format!("{}{value:#x}", if step == 0 { "" } else { "," }));
        }
        out
    };
    for offset in [0x18usize, 0x30, 0x50] {
        crate::dbg_log_public(&format!(
            "[itemslot] post-acquire clone {public_kind:#x} vec{offset:02x}: clone=[{}] base=[{}]",
            elements(slot, offset),
            elements(native_slot, offset),
        ));
    }
}

unsafe extern "C" fn container_insert(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(public_kind) = redirect_target() else {
        return;
    };
    static SEEN: AtomicUsize = AtomicUsize::new(0);
    let seen = SEEN.fetch_add(1, Ordering::Relaxed);
    if seen >= PATH_PROBE_LIMIT {
        return;
    }
    let key = ctx.registers[CONTAINER_INSERT_KEY_REGISTER].x();
    let index =
        core::ptr::read_volatile(ctx.registers[CONTAINER_INSERT_INDEX_REGISTER].x() as *const u32);
    crate::dbg_log_public(&format!(
        "[itemslot] clone {public_kind:#x} insert #{seen}: key={key:#x} (tag {}) index={index:#x} -> {}",
        key & 0xFFFF,
        if index == 0xFF_FFFF {
            "SKIPPED, no node"
        } else {
            "inserted"
        }
    ));
}

unsafe extern "C" fn acquire_miss(_ctx: &mut skyline::hooks::InlineCtx) {
    acquire_outcome("MISS - the node is not in the tree acquire is reading");
}

unsafe extern "C" fn acquire_hit(_ctx: &mut skyline::hooks::InlineCtx) {
    acquire_outcome("HIT - the node was found");
}

fn acquire_outcome(what: &str) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(public_kind) = redirect_target() else {
        return;
    };
    static SEEN: AtomicUsize = AtomicUsize::new(0);
    let seen = SEEN.fetch_add(1, Ordering::Relaxed);
    if seen >= PATH_PROBE_LIMIT {
        return;
    }
    crate::dbg_log_public(&format!(
        "[itemslot] clone {public_kind:#x} acquire #{seen}: {what}"
    ));
}

unsafe extern "C" fn match_load_tail(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    if ctx.registers[MATCH_LOAD_COUNTER].x() != ITEM_KIND_TERM {
        return;
    }
    load_clone_slots();
    crate::item_params::report("match-load");
}

macro_rules! basename_hook {
    ($name:ident, $index:expr) => {
        unsafe extern "C" fn $name(ctx: &mut skyline::hooks::InlineCtx) {
            apply_basename(ctx, $index);
        }
    };
}

macro_rules! slot_hook {
    ($name:ident, $index:expr) => {
        unsafe extern "C" fn $name(ctx: &mut skyline::hooks::InlineCtx) {
            apply_slot(ctx, $index);
        }
    };
}

macro_rules! sibling_hook {
    ($name:ident, $index:expr) => {
        unsafe extern "C" fn $name(ctx: &mut skyline::hooks::InlineCtx) {
            apply_sibling(ctx, $index);
        }
    };
}

basename_hook!(basename_0, 0);
basename_hook!(basename_1, 1);

sibling_hook!(sibling_0, 0);
sibling_hook!(sibling_1, 1);
sibling_hook!(sibling_2, 2);

slot_hook!(slot_0, 0);
slot_hook!(slot_1, 1);
slot_hook!(slot_2, 2);
slot_hook!(slot_3, 3);
slot_hook!(slot_4, 4);
slot_hook!(slot_5, 5);
slot_hook!(slot_6, 6);
slot_hook!(slot_7, 7);
slot_hook!(slot_8, 8);
slot_hook!(slot_9, 9);
slot_hook!(slot_10, 10);
slot_hook!(slot_11, 11);
slot_hook!(slot_12, 12);
slot_hook!(slot_13, 13);
slot_hook!(slot_14, 14);
slot_hook!(slot_15, 15);
slot_hook!(slot_16, 16);
slot_hook!(slot_17, 17);
slot_hook!(slot_18, 18);
slot_hook!(slot_19, 19);
slot_hook!(slot_20, 20);
slot_hook!(slot_21, 21);
slot_hook!(slot_22, 22);
slot_hook!(slot_23, 23);
slot_hook!(slot_24, 24);
slot_hook!(slot_25, 25);
slot_hook!(slot_26, 26);
slot_hook!(slot_27, 27);
slot_hook!(slot_28, 28);
slot_hook!(slot_29, 29);
slot_hook!(slot_30, 30);
slot_hook!(slot_31, 31);
slot_hook!(slot_32, 32);
slot_hook!(slot_33, 33);
slot_hook!(slot_34, 34);
slot_hook!(slot_35, 35);
slot_hook!(slot_36, 36);

pub(crate) type InlineHook = unsafe extern "C" fn(&mut skyline::hooks::InlineCtx);

const BASENAME_HOOKS: &[InlineHook] = &[basename_0, basename_1];
const SIBLING_HOOKS: &[InlineHook] = &[sibling_0, sibling_1, sibling_2];
const SLOT_HOOKS: &[InlineHook] = &[
    slot_0, slot_1, slot_2, slot_3, slot_4, slot_5, slot_6, slot_7, slot_8, slot_9, slot_10,
    slot_11, slot_12, slot_13, slot_14, slot_15, slot_16, slot_17, slot_18, slot_19, slot_20,
    slot_21, slot_22, slot_23, slot_24, slot_25, slot_26, slot_27, slot_28, slot_29, slot_30,
    slot_31, slot_32, slot_33, slot_34, slot_35,
];

const _: () = assert!(SLOT_HOOKS.len() == SLOT_SITES.len());
const _: () = assert!(SITE_MARKED.len() >= MARK_SLOT + SLOT_SITES.len());
const _: () = assert!(BASENAME_HOOKS.len() == BASENAME_SITES.len());
const _: () = assert!(SIBLING_HOOKS.len() == SIBLING_SITES.len());

pub(crate) fn install() {
    match unsafe { preflight() } {
        Ok(()) => unsafe {
            PREFLIGHT_OK.store(true, Ordering::Release);
            let text = crate::text_base();

            let mut installed = 0usize;
            let mut failed: Vec<usize> = Vec::new();

            let mut arm = |offset: usize, original: u32, hook: InlineHook| {
                skyline::hooks::A64InlineHook(
                    (text + offset) as *const libc::c_void,
                    hook as *const () as *const libc::c_void,
                );
                if text_word(offset) != original {
                    installed += 1;
                } else {
                    failed.push(offset);
                }
            };

            arm(ROW_SITE.offset, ROW_SITE.expected, descriptor_row);
            arm(PATH_CATEGORY_SITE, PATH_CATEGORY_EXPECTED, path_category);
            for (site, hook) in BASENAME_SITES.iter().zip(BASENAME_HOOKS) {
                arm(site.hook, site.expected_window[0], *hook);
            }
            for (site, hook) in SLOT_SITES.iter().zip(SLOT_HOOKS) {
                arm(site.offset, site.expected, *hook);
            }
            for (site, hook) in SIBLING_SITES.iter().zip(SIBLING_HOOKS) {
                arm(site.offset, site.expected, *hook);
            }
            arm(MATCH_LOAD_TAIL, MATCH_LOAD_TAIL_EXPECTED, match_load_tail);
            arm(POST_ACQUIRE, POST_ACQUIRE_EXPECTED, post_acquire);
            arm(CONTAINER_INSERT, CONTAINER_INSERT_EXPECTED, container_insert);
            arm(ACQUIRE_MISS, ACQUIRE_MISS_EXPECTED, acquire_miss);
            arm(ACQUIRE_HIT, ACQUIRE_HIT_EXPECTED, acquire_hit);

            let total =
                11 + BASENAME_SITES.len() + SLOT_SITES.len() + SIBLING_SITES.len();
            HOOKS_INSTALLED.store(installed > 0, Ordering::Release);
            ROUTER_READY.store(failed.is_empty() && installed == total, Ordering::Release);
            crate::dbg_log_public(&format!(
                "[itemslot] narrow resource backend: {installed} of {total} hooks armed ({} slot sites of {KNOWN_SLOT_SITE_COUNT} known), no geometry change and no text patch",
                SLOT_SITES.len()
            ));
            for offset in &failed {
                crate::dbg_log_public(&format!(
                    "[itemslot] site {offset:#x} did not relocate; its original instruction is intact and it stays inert"
                ));
            }
        },
        Err(error) => crate::dbg_log_public(&format!(
            "[itemslot] preflight failed; nothing installed: {}",
            match error {
                PreflightError::Geometry => "slot geometry disagrees with the engine".to_string(),
                PreflightError::Opcode {
                    offset,
                    expected,
                    actual,
                } => format!(
                    "opcode mismatch at {offset:#x}: expected={expected:#010x} actual={actual:#010x}"
                ),
                PreflightError::Register { offset } =>
                    format!("operand contract failed at {offset:#x}"),
            }
        )),
    }
}

pub(crate) fn ready() -> bool {
    ROUTER_READY.load(Ordering::Acquire)
}
