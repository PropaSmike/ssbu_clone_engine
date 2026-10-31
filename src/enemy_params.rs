use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::enemy_clones::{base_of, name_of, KIND_LIMIT, VANILLA_KINDS};

const OFF_PARAM_LOAD: usize = 0x56_bc90;
const PARAM_LOAD_OPCODE: u32 = 0xd105_43ff;
const OFF_PARAMS_BUILT: usize = 0x14d_fdf4;
const PARAMS_BUILT_OPCODE: u32 = 0xf940_5e69;
const OFF_FILE_BY_HASH: usize = 0x353_d480;
const OFF_FILE_RELEASE: usize = 0x354_0560;
const OFF_FILESYSTEM: usize = 0x533_1f20;
const OFF_ALLOC: usize = 0x392_dce0;
const OFF_PARSE: usize = 0x372_0910;
const OFF_HOLDER: usize = 0x52b_8418;
const HOLDER_PARAMS: usize = 0xb8;
const PARAMS_OBJECTS: usize = 0xd8;
const NO_FILE: u32 = 0xff_ffff;
const OBJECT_ALIGN: usize = 0x10;
const NABBIT: u32 = 4;
const NABBIT_SIZE: usize = 0x198;
const NABBIT_DEFAULTS: usize = 0x158;
const DELETE_SLOT: usize = 1;
const ADDRESS_REGISTER: usize = 8;

type FileByHashFn = unsafe extern "C" fn(*mut [u32; 4], u64);
type FileReleaseFn = unsafe extern "C" fn(usize, u32);
type AllocFn = unsafe extern "C" fn(usize, usize) -> usize;
type ParseFn = unsafe extern "C" fn(usize, u32) -> bool;
type DeleteFn = unsafe extern "C" fn(usize);

const NO_FILE_SLOT: AtomicU32 = AtomicU32::new(NO_FILE);
const NO_OBJECT: AtomicUsize = AtomicUsize::new(0);
static FILES: [AtomicU32; KIND_LIMIT as usize] = [NO_FILE_SLOT; KIND_LIMIT as usize];
static OBJECTS: [AtomicUsize; KIND_LIMIT as usize] = [NO_OBJECT; KIND_LIMIT as usize];

unsafe fn read(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

pub(crate) fn own_object(kind: u32) -> usize {
    if !(VANILLA_KINDS..KIND_LIMIT).contains(&kind) {
        return 0;
    }
    OBJECTS[kind as usize].load(Ordering::Acquire)
}

unsafe fn base_object(base: u32) -> usize {
    let holder = read(crate::game::addr(OFF_HOLDER));
    if holder == 0 {
        return 0;
    }
    let params = read(holder + HOLDER_PARAMS);
    if params == 0 {
        return 0;
    }
    read(params + PARAMS_OBJECTS + base as usize * 8)
}

unsafe fn request_files() {
    let filesystem = read(crate::game::addr(OFF_FILESYSTEM));
    let by_hash: FileByHashFn = core::mem::transmute(crate::game::addr(OFF_FILE_BY_HASH));
    let release: FileReleaseFn = core::mem::transmute(crate::game::addr(OFF_FILE_RELEASE));
    for kind in VANILLA_KINDS..KIND_LIMIT {
        let old = FILES[kind as usize].swap(NO_FILE, Ordering::AcqRel);
        if old != NO_FILE && filesystem != 0 {
            release(filesystem, old);
        }
        if base_of(kind) != Some(NABBIT) || !crate::enemy_spawn::armed(kind) {
            continue;
        }
        let Some(name) = name_of(kind) else {
            continue;
        };
        let mut handle = [NO_FILE; 4];
        by_hash(&mut handle, crate::hash40::hash40(&format!("enemy/{name}/model/body/c00/param.prc")));
        if handle[0] != NO_FILE {
            FILES[kind as usize].store(handle[0], Ordering::Release);
        }
        skyline::println!("[enemyparam] {name} (kind {kind}): own param file {:#x}", handle[0]);
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_PARAM_LOAD))]
unsafe fn param_load() {
    call_original!();
    request_files();
}

unsafe fn build(kind: u32) -> usize {
    let file = FILES[kind as usize].load(Ordering::Acquire);
    let base = base_object(NABBIT);
    if file == NO_FILE || base == 0 {
        return 0;
    }
    let alloc: AllocFn = core::mem::transmute(crate::game::addr(OFF_ALLOC));
    let object = alloc(OBJECT_ALIGN, NABBIT_SIZE);
    if object == 0 {
        return 0;
    }
    core::ptr::write_bytes(object as *mut u8, 0, NABBIT_SIZE);
    core::ptr::copy_nonoverlapping(
        (base + NABBIT_DEFAULTS) as *const u8,
        (object + NABBIT_DEFAULTS) as *mut u8,
        NABBIT_SIZE - NABBIT_DEFAULTS,
    );
    core::ptr::write_volatile(object as *mut usize, read(base));
    let parse: ParseFn = core::mem::transmute(crate::game::addr(OFF_PARSE));
    if !parse(object, file) {
        delete(object);
        skyline::println!("[enemyparam] kind {kind}: param file {file:#x} not readable yet");
        return 0;
    }
    object
}

pub(crate) fn ensure(kind: u32) {
    if !(VANILLA_KINDS..KIND_LIMIT).contains(&kind) || OBJECTS[kind as usize].load(Ordering::Acquire) != 0 {
        return;
    }
    let object = unsafe { build(kind) };
    if object != 0 {
        OBJECTS[kind as usize].store(object, Ordering::Release);
        skyline::println!("[enemyparam] kind {kind}: own params at {object:#x}");
    }
}

unsafe fn delete(object: usize) {
    let delete: DeleteFn = core::mem::transmute(read(read(object) + DELETE_SLOT * 8));
    delete(object);
}

#[skyline::hook(offset = crate::game::rel(OFF_PARAMS_BUILT), inline)]
unsafe fn params_built(_ctx: &mut skyline::hooks::InlineCtx) {
    for kind in VANILLA_KINDS..KIND_LIMIT {
        let old = OBJECTS[kind as usize].swap(0, Ordering::AcqRel);
        if old != 0 {
            delete(old);
        }
        ensure(kind);
    }
}

macro_rules! own_param_sites {
    ($($name:ident = $offset:literal, $object:literal, $field:literal, $opcode:literal;)*) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let object = ctx.registers[$object].x() as usize;
                if object == 0 {
                    return;
                }
                let kind = core::ptr::read_volatile((object + $field) as *const u32);
                if own_object(kind) != 0 {
                    let slot = &OBJECTS[kind as usize] as *const AtomicUsize as u64;
                    ctx.registers[ADDRESS_REGISTER].set_x(slot - PARAMS_OBJECTS as u64);
                }
            }
        )*

        const OWN_PARAM_SITES: &[(usize, u32)] = &[$(($offset, $opcode)),*];

        fn install_own_param_sites() {
            skyline::install_hooks!($($name),*);
        }
    };
}

own_param_sites! {
    object_scale_params = 0x565d48, 19, 0xc, 0xf9406d15;
    object_search_params = 0x565f70, 19, 0xc, 0xf9406d14;
    object_table_params_a = 0x568364, 19, 0xc, 0xf9406d14;
    object_table_params_b = 0x5685f0, 19, 0xc, 0xf9406d14;
    object_setup_params = 0x566ac0, 19, 0xc, 0xf9406d14;
    descriptor_params = 0x571f5c, 19, 0x9c, 0xf9406d16;
    descriptor_tail_params = 0x57211c, 19, 0x9c, 0xf9406d15;
    foreign_params_a = 0x34b0290, 26, 0xc, 0xf9406d1b;
    foreign_params_b = 0x34b2868, 25, 0xc, 0xf9406d18;
}

pub(crate) fn install() {
    let sites = OWN_PARAM_SITES
        .iter()
        .chain([(OFF_PARAM_LOAD, PARAM_LOAD_OPCODE), (OFF_PARAMS_BUILT, PARAMS_BUILT_OPCODE)].iter());
    for &(offset, expected) in sites {
        let observed = unsafe { crate::game::text_word(offset) };
        if observed != expected {
            skyline::println!("[enemyparam] REFUSED {offset:#x}: expected {expected:#010x}, found {observed:#010x}");
            return;
        }
    }
    install_own_param_sites();
    skyline::install_hooks!(param_load, params_built);
}
