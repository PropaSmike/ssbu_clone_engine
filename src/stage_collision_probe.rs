#![allow(dead_code)]

const SUBSYSTEM: usize = 0x738;
const ELEMENT_SLOT: usize = 0x208;
const VECTOR: usize = 0x1e8;
const ELEMENT_GATE: usize = 0x25;
const ELEMENT_INDEX: usize = 0x28;

const STAGE_ID: usize = 0x8;

const ELEMENTS_TO_WALK: usize = 1;

const REPORT_CALLS: u32 = 6;

const OFF_SUBOBJECT_INIT: usize = 0x261f760;

const SINGLETON_SLOT: usize = 0x53299d8;
const SINGLETON_MAP: usize = 0x1c0;

const SUBOBJECT_KEY: usize = 0x8;
const SUBOBJECT_VECTOR: usize = 0xc8;
const DESCRIPTOR_SETTING: usize = 0x18;

const REPORT_INITS: u32 = 8;

const OFF_PICT_NAME_INIT: usize = 0x2d9b220;
const PICT_NAME_INIT_OPCODE: u32 = 0xd10303ff;

const PICT_LANGUAGE_INDEX: usize = 0x523c00c;

const PICT_LANGUAGE_VECTOR: usize = 0x450;
const PICT_LANGUAGE_RECORD_SIZE: usize = 0x2c;
const REPORT_PICT_CALLS: u32 = 4;

const OFF_PICT_SCHEMA_RESOLVER: usize = 0x2d9f170;
const PICT_SCHEMA_RESOLVER_OPCODE: u32 = 0xaa0103e9;
const PICT_LANGUAGE_FIELD: u64 = 0x122943f952;
const REPORT_SCHEMA_CALLS: u32 = 24;

const OFF_STDAT_LIST_READY: usize = 0x25ffa7c;
const STDAT_LIST_READY_OPCODE: u32 = 0xa943e3f3;
const OFF_STDAT_EXTENSION_RESULT: usize = 0x25ffac0;
const STDAT_EXTENSION_RESULT_OPCODE: u32 = 0x36000220;
const OFF_STDAT_REGISTER_CALL: usize = 0x25ffb00;
const STDAT_REGISTER_CALL_OPCODE: u32 = 0x94006514;
const OFF_STDAT_REGISTER_RETURN: usize = 0x25ffb04;
const STDAT_REGISTER_RETURN_OPCODE: u32 = 0x6b1902bf;
const OFF_STDAT_SCAN_FINISH: usize = 0x25ffc38;
const STDAT_SCAN_FINISH_OPCODE: u32 = 0x9100e3e0;

const RESOURCE_CATEGORY_BASE: usize = 0xd0;
const RESOURCE_CATEGORY_STRIDE: usize = 0x30;
const STDAT_RESOURCE_CATEGORY: usize = 3;
const REPORT_STDAT_CANDIDATES: u32 = 12;

const ARC_SERVICE_GLOBAL: usize = 0x5331f20;
const ARC_SERVICE_ARC: usize = 0x78;
const ARC_FS_HEADER: usize = 0x40;
const ARC_FILE_PATHS: usize = 0x60;
const FS_HEADER_PATH_COUNT: usize = 0x04;
const FILE_PATH_STRIDE: usize = 0x20;
const VANILLA_FILE_COUNT: usize = 590_711;

#[cfg(all(not(test), feature = "stage_collision_probe"))]
static LAST_STAGE: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static CALLS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

unsafe fn vector_of(subsystem: *const u8) -> (usize, usize, i64) {
    let begin = core::ptr::read_volatile(subsystem.add(VECTOR) as *const usize);
    let end = core::ptr::read_volatile(subsystem.add(VECTOR + 8) as *const usize);
    (begin, end, (end as i64 - begin as i64) / 4)
}

unsafe fn follow(base: *const u8, offset: usize) -> Option<*const u8> {
    if base.is_null() {
        return None;
    }
    let value = core::ptr::read_volatile(base.add(offset) as *const usize);
    (value != 0).then_some(value as *const u8)
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = 0x2cb45b0)]
unsafe fn stage_update_probe(stage: *const u8, delta: f32) -> u64 {
    use core::sync::atomic::Ordering::Relaxed;
    if LAST_STAGE.swap(stage as usize, Relaxed) != stage as usize {
        CALLS.store(0, Relaxed);
    }
    let call = CALLS.fetch_add(1, Relaxed);
    if call >= REPORT_CALLS {
        return call_original!(stage, delta);
    }

    report(call, stage);
    let result = call_original!(stage, delta);
    if let Some(subsystem) = follow(stage, SUBSYSTEM) {
        let (begin, end, len) = vector_of(subsystem);
        skyline::println!("[stagecol] #{call} after: vector[{len}] begin={begin:#x} end={end:#x}");
    }
    result
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
unsafe fn report(call: u32, stage: *const u8) {
    let id = if stage.is_null() {
        u32::MAX
    } else {
        core::ptr::read_volatile(stage.add(STAGE_ID) as *const u32)
    };
    let Some(subsystem) = follow(stage, SUBSYSTEM) else {
        skyline::println!("[stagecol] #{call} stage={stage:p} id?={id} has no subsystem at +0x738");
        return;
    };
    let (begin, end, len) = vector_of(subsystem);
    skyline::println!(
        "[stagecol] #{call} stage={stage:p} id?={id} subsystem={subsystem:p} \
         vector[{len}] begin={begin:#x} end={end:#x}"
    );

    let Some(slot) = follow(subsystem, ELEMENT_SLOT) else {
        skyline::println!("[stagecol] #{call} no element slot at +0x208");
        return;
    };
    for step in 0..ELEMENTS_TO_WALK {
        let Some(element) = follow(slot, step * 8) else {
            continue;
        };
        let gate = core::ptr::read_volatile(element.add(ELEMENT_GATE));
        let index = core::ptr::read_volatile(element.add(ELEMENT_INDEX) as *const i32);
        let verdict = if gate == 0 {
            "gate closed, at() not reached"
        } else if len < 0 {
            "*** VECTOR CORRUPT (end < begin) ***"
        } else if (index as i64) < 0 || index as i64 >= len {
            "*** OUT OF RANGE -- this element throws ***"
        } else {
            "in range"
        };
        skyline::println!(
            "[stagecol] #{call}   [{step}] element={element:p} gate={gate} index={index} {verdict}"
        );
    }
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
static INITS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(all(not(test), feature = "stage_collision_probe"))]
static LAST_PICT_STAGE: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static PICT_CALLS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static PICT_SCHEMA_CALLS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(all(not(test), feature = "stage_collision_probe"))]
static STDAT_SCAN_ARMED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static STDAT_SCAN_MINTED: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static STDAT_SCAN_DONOR: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static STDAT_CANDIDATES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static STDAT_MATCHES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(all(not(test), feature = "stage_collision_probe"))]
static STDAT_REGISTRATIONS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(all(not(test), feature = "stage_collision_probe"))]
pub(crate) fn arm_stdat_scan(minted: u32, donor: u32) {
    use core::sync::atomic::Ordering::{Relaxed, Release};

    STDAT_SCAN_MINTED.store(minted, Relaxed);
    STDAT_SCAN_DONOR.store(donor, Relaxed);
    STDAT_CANDIDATES.store(0, Relaxed);
    STDAT_MATCHES.store(0, Relaxed);
    STDAT_REGISTRATIONS.store(0, Relaxed);
    STDAT_SCAN_ARMED.store(true, Release);
}

#[cfg(not(all(not(test), feature = "stage_collision_probe")))]
pub(crate) fn arm_stdat_scan(_minted: u32, _donor: u32) {}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
unsafe fn stdat_stack_word(ctx: &skyline::hooks::InlineCtx, offset: usize) -> usize {
    let sp = ctx.sp.x() as *const u8;
    if sp.is_null() {
        0
    } else {
        core::ptr::read_unaligned(sp.add(offset) as *const usize)
    }
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
unsafe fn stdat_category_vector(aggregate: usize) -> (usize, usize, i64) {
    if aggregate == 0 {
        return (0, 0, 0);
    }
    let vector =
        aggregate + RESOURCE_CATEGORY_BASE + STDAT_RESOURCE_CATEGORY * RESOURCE_CATEGORY_STRIDE;
    let begin = core::ptr::read_volatile(vector as *const usize);
    let end = core::ptr::read_volatile((vector + 8) as *const usize);
    let bytes = end as i64 - begin as i64;
    let len = if bytes >= 0 && bytes % 4 == 0 {
        bytes / 4
    } else {
        -1
    };
    (begin, end, len)
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
unsafe fn stdat_file_path_hash(index: u32) -> Option<u64> {
    let service =
        core::ptr::read_volatile((crate::text_base() + ARC_SERVICE_GLOBAL) as *const usize);
    if service == 0 {
        return None;
    }
    let holder = core::ptr::read_volatile((service + ARC_SERVICE_ARC) as *const usize);
    if holder == 0 {
        return None;
    }
    let arc = core::ptr::read_volatile(holder as *const usize);
    if arc == 0 {
        return None;
    }
    let header = core::ptr::read_volatile((arc + ARC_FS_HEADER) as *const usize);
    let paths = core::ptr::read_volatile((arc + ARC_FILE_PATHS) as *const usize);
    if header == 0 || paths == 0 {
        return None;
    }
    let count = core::ptr::read_volatile((header + FS_HEADER_PATH_COUNT) as *const u32) as usize;
    let index = index as usize;
    if !(VANILLA_FILE_COUNT..=VANILLA_FILE_COUNT * 4).contains(&count) || index >= count {
        return None;
    }
    let entry = paths + index * FILE_PATH_STRIDE;
    let low = core::ptr::read_volatile(entry as *const u32) as u64;
    let length = core::ptr::read_volatile((entry + 4) as *const u8) as u64;
    Some((length << 32) | low)
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_STDAT_LIST_READY, inline)]
unsafe fn stdat_list_ready_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::Ordering::Acquire;
    if !STDAT_SCAN_ARMED.load(Acquire) {
        return;
    }
    let begin = stdat_stack_word(ctx, 0x38);
    let end = stdat_stack_word(ctx, 0x40);
    let bytes = end as i64 - begin as i64;
    let count = if bytes >= 0 && bytes % 4 == 0 {
        bytes / 4
    } else {
        -1
    };
    let group = stdat_stack_word(ctx, 0x18) as u32;
    skyline::println!(
        "[stdatflow] list minted={} donor={} group={group:#x} files={count} begin={begin:#x} end={end:#x}",
        STDAT_SCAN_MINTED.load(Acquire),
        STDAT_SCAN_DONOR.load(Acquire),
    );
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_STDAT_EXTENSION_RESULT, inline)]
unsafe fn stdat_extension_result_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed};
    if !STDAT_SCAN_ARMED.load(Acquire) {
        return;
    }
    let ordinal = STDAT_CANDIDATES.fetch_add(1, Relaxed);
    let file = ctx.registers[21].x() as u32;
    let matched = ctx.registers[0].x() & 1 != 0;
    if matched {
        STDAT_MATCHES.fetch_add(1, AcqRel);
    }
    if matched || ordinal < REPORT_STDAT_CANDIDATES {
        let hash = stdat_file_path_hash(file).unwrap_or(0);
        skyline::println!(
            "[stdatflow] candidate #{ordinal} file={file:#x} hash={hash:#x} .stdat={matched}"
        );
    }
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_STDAT_REGISTER_CALL, inline)]
unsafe fn stdat_register_call_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::Ordering::{AcqRel, Acquire};
    if !STDAT_SCAN_ARMED.load(Acquire) {
        return;
    }
    let aggregate = ctx.registers[0].x() as usize;
    let category = ctx.registers[1].x() as u32;
    let file_ptr = ctx.registers[2].x() as *const u32;
    let file = if file_ptr.is_null() {
        u32::MAX
    } else {
        core::ptr::read_unaligned(file_ptr)
    };
    let (begin, end, len) = stdat_category_vector(aggregate);
    STDAT_REGISTRATIONS.fetch_add(1, AcqRel);
    skyline::println!(
        "[stdatflow] register file={file:#x} category={category} aggregate={aggregate:#x} before[{len}]={begin:#x}..{end:#x}"
    );
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_STDAT_REGISTER_RETURN, inline)]
unsafe fn stdat_register_return_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::Ordering::Acquire;
    if !STDAT_SCAN_ARMED.load(Acquire) {
        return;
    }
    let aggregate = stdat_stack_word(ctx, 0x10);
    let file = stdat_stack_word(ctx, 0x30) as u32;
    let (begin, end, len) = stdat_category_vector(aggregate);
    skyline::println!(
        "[stdatflow] registered file={file:#x} aggregate={aggregate:#x} after[{len}]={begin:#x}..{end:#x}"
    );
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_STDAT_SCAN_FINISH, inline)]
unsafe fn stdat_scan_finish_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::Ordering::{AcqRel, Acquire};
    if !STDAT_SCAN_ARMED.swap(false, AcqRel) {
        return;
    }
    let aggregate = stdat_stack_word(ctx, 0x10);
    let (begin, end, len) = stdat_category_vector(aggregate);
    skyline::println!(
        "[stdatflow] finish minted={} donor={} candidates={} matches={} registrations={} aggregate={aggregate:#x} category3[{len}]={begin:#x}..{end:#x}",
        STDAT_SCAN_MINTED.load(Acquire),
        STDAT_SCAN_DONOR.load(Acquire),
        STDAT_CANDIDATES.load(Acquire),
        STDAT_MATCHES.load(Acquire),
        STDAT_REGISTRATIONS.load(Acquire),
    );
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_PICT_SCHEMA_RESOLVER, inline)]
unsafe fn pict_schema_resolver_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use core::sync::atomic::Ordering::Relaxed;

    let call = PICT_SCHEMA_CALLS.fetch_add(1, Relaxed);
    let object = ctx.registers[0].x();
    let hash = ctx.registers[1].x() & 0xffffffffff;
    if call < REPORT_SCHEMA_CALLS || hash == PICT_LANGUAGE_FIELD {
        skyline::println!(
            "[pictschema] #{call} object={object:#x} field={hash:#x}{}",
            if hash == PICT_LANGUAGE_FIELD {
                " LOCALIZED-NAMES"
            } else {
                ""
            }
        );
    }
}

unsafe fn pointer_vector(object: *const u8, offset: usize) -> (usize, usize, i64) {
    let begin = core::ptr::read_volatile(object.add(offset) as *const usize);
    let end = core::ptr::read_volatile(object.add(offset + 8) as *const usize);
    (begin, end, (end as i64 - begin as i64) / 8)
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_SUBOBJECT_INIT)]
unsafe fn subobject_init_probe(
    object: *mut u8,
    descriptor: *const u8,
    third: u64,
    fourth: u64,
    fifth: u64,
    sixth: u64,
) -> u64 {
    use core::sync::atomic::Ordering::Relaxed;
    let call = INITS.fetch_add(1, Relaxed);
    if call >= REPORT_INITS {
        return call_original!(object, descriptor, third, fourth, fifth, sixth);
    }

    report_init(call, object, descriptor);
    let result = call_original!(object, descriptor, third, fourth, fifth, sixth);
    let (begin, end, len) = pointer_vector(object, SUBOBJECT_VECTOR);
    let first = if begin != 0 && begin == end {
        "*** EMPTY -- element 0 is about to be dereferenced ***"
    } else if begin == 0 {
        "*** NULL BEGIN ***"
    } else {
        "populated"
    };
    skyline::println!(
        "[stagesetup] #{call} returned; vector[{len}] begin={begin:#x} end={end:#x} {first}"
    );
    result
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
unsafe fn report_init(call: u32, object: *const u8, descriptor: *const u8) {
    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize;

    let class = match follow(object, 0) {
        Some(vtable) => (vtable as usize).wrapping_sub(text),
        None => 0,
    };
    let key = if object.is_null() {
        0
    } else {
        core::ptr::read_volatile(object.add(SUBOBJECT_KEY) as *const u64)
    };

    let singleton = follow(text as *const u8, SINGLETON_SLOT);
    let inner = singleton.and_then(|s| follow(s, 0));
    let map = inner.and_then(|i| follow(i, SINGLETON_MAP));

    let setting_id = match follow(descriptor, DESCRIPTOR_SETTING) {
        Some(setting) => core::ptr::read_volatile(setting as *const u32) as i64,
        None => -1,
    };

    skyline::println!(
        "[stagesetup] #{call} obj={object:p} class={class:#x} key={key:#x} \
         setting_id={setting_id} singleton={:#x} inner={:#x} map={:#x}",
        singleton.map_or(0, |p| p as usize),
        inner.map_or(0, |p| p as usize),
        map.map_or(0, |p| p as usize),
    );
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
#[skyline::hook(offset = OFF_PICT_NAME_INIT)]
unsafe fn pict_name_init_probe(
    output: *mut u8,
    stage: *const u8,
    subsystem: *const u8,
    scale: f32,
) -> u64 {
    use core::sync::atomic::Ordering::Relaxed;

    let caller: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) caller);
    #[cfg(not(target_arch = "aarch64"))]
    {
        caller = 0;
    }
    if LAST_PICT_STAGE.swap(stage as usize, Relaxed) != stage as usize {
        PICT_CALLS.store(0, Relaxed);
    }
    let call = PICT_CALLS.fetch_add(1, Relaxed);
    if call < REPORT_PICT_CALLS {
        report_pict_name_init(call, output, stage, subsystem, scale, caller);
    }

    call_original!(output, stage, subsystem, scale)
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
unsafe fn report_pict_name_init(
    call: u32,
    output: *const u8,
    stage: *const u8,
    subsystem: *const u8,
    scale: f32,
    caller: usize,
) {
    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u8;
    let locale = core::ptr::read_volatile(text.add(PICT_LANGUAGE_INDEX) as *const u32);
    let stage_id = if stage.is_null() {
        u32::MAX
    } else {
        core::ptr::read_volatile(stage.add(STAGE_ID) as *const u32)
    };

    if subsystem.is_null() {
        skyline::println!(
            "[pictlang] #{call} stage={stage:p} id?={stage_id} out={output:p} scale={scale} \
             locale={locale} subsystem=NULL caller={caller:#x}"
        );
        return;
    }

    let begin = core::ptr::read_volatile(subsystem.add(PICT_LANGUAGE_VECTOR) as *const usize);
    let end = core::ptr::read_volatile(subsystem.add(PICT_LANGUAGE_VECTOR + 8) as *const usize);
    let bytes = end as i64 - begin as i64;
    let aligned = bytes >= 0 && bytes % PICT_LANGUAGE_RECORD_SIZE as i64 == 0;
    let len = if aligned {
        bytes / PICT_LANGUAGE_RECORD_SIZE as i64
    } else {
        -1
    };
    let relation = (subsystem as usize).wrapping_sub(stage as usize);

    skyline::println!(
        "[pictlang] #{call} stage={stage:p} id?={stage_id} out={output:p} subsystem={subsystem:p} \
         delta={relation:#x} scale={scale} locale={locale} records={len} bytes={bytes} \
         begin={begin:#x} end={end:#x} caller={caller:#x}"
    );

    if begin != 0 && aligned && (locale as i64) < len {
        let words_per_row = PICT_LANGUAGE_RECORD_SIZE / core::mem::size_of::<u32>();
        let row = (begin as *const u32).add(locale as usize * words_per_row);
        skyline::println!(
            "[pictlang] #{call} row[{locale}]={:08x},{:08x},{:08x},{:08x},{:08x},{:08x},\
             {:08x},{:08x},{:08x},{:08x},{:08x}",
            core::ptr::read_volatile(row.add(0)),
            core::ptr::read_volatile(row.add(1)),
            core::ptr::read_volatile(row.add(2)),
            core::ptr::read_volatile(row.add(3)),
            core::ptr::read_volatile(row.add(4)),
            core::ptr::read_volatile(row.add(5)),
            core::ptr::read_volatile(row.add(6)),
            core::ptr::read_volatile(row.add(7)),
            core::ptr::read_volatile(row.add(8)),
            core::ptr::read_volatile(row.add(9)),
            core::ptr::read_volatile(row.add(10)),
        );
    }
}

#[cfg(all(not(test), feature = "stage_collision_probe"))]
pub(crate) fn install() {
    unsafe {
        skyline::install_hook!(stage_update_probe);
        skyline::install_hook!(subobject_init_probe);
    }

    let text =
        unsafe { skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u8 };
    let observed = unsafe { core::ptr::read_volatile(text.add(OFF_PICT_NAME_INIT) as *const u32) };
    if observed == PICT_NAME_INIT_OPCODE {
        skyline::install_hook!(pict_name_init_probe);
        skyline::println!(
            "[pictlang] probe armed at {OFF_PICT_NAME_INIT:#x}; expected opcode \
             {PICT_NAME_INIT_OPCODE:#010x}, first {REPORT_PICT_CALLS} call(s) per stage"
        );
    } else {
        skyline::println!(
            "[pictlang] REFUSED at {OFF_PICT_NAME_INIT:#x}: expected \
             {PICT_NAME_INIT_OPCODE:#010x}, found {observed:#010x}"
        );
    }

    let schema_observed =
        unsafe { core::ptr::read_volatile(text.add(OFF_PICT_SCHEMA_RESOLVER) as *const u32) };
    if schema_observed == PICT_SCHEMA_RESOLVER_OPCODE {
        skyline::install_hook!(pict_schema_resolver_probe);
        skyline::println!(
            "[pictschema] probe armed at {OFF_PICT_SCHEMA_RESOLVER:#x}; expected opcode \
             {PICT_SCHEMA_RESOLVER_OPCODE:#010x}, first {REPORT_SCHEMA_CALLS} fields plus \
             localized-name field"
        );
    } else {
        skyline::println!(
            "[pictschema] REFUSED at {OFF_PICT_SCHEMA_RESOLVER:#x}: expected \
             {PICT_SCHEMA_RESOLVER_OPCODE:#010x}, found {schema_observed:#010x}"
        );
    }

    let mut stdat_flow_ok = true;
    for (site, expected, label) in [
        (OFF_STDAT_LIST_READY, STDAT_LIST_READY_OPCODE, "list result"),
        (
            OFF_STDAT_EXTENSION_RESULT,
            STDAT_EXTENSION_RESULT_OPCODE,
            "extension result",
        ),
        (
            OFF_STDAT_REGISTER_CALL,
            STDAT_REGISTER_CALL_OPCODE,
            "registration call",
        ),
        (
            OFF_STDAT_REGISTER_RETURN,
            STDAT_REGISTER_RETURN_OPCODE,
            "registration return",
        ),
        (
            OFF_STDAT_SCAN_FINISH,
            STDAT_SCAN_FINISH_OPCODE,
            "scan finish",
        ),
    ] {
        let observed = unsafe { core::ptr::read_volatile(text.add(site) as *const u32) };
        if observed != expected {
            stdat_flow_ok = false;
            skyline::println!(
                "[stdatflow] REFUSED {label} at {site:#x}: expected {expected:#010x}, found {observed:#010x}"
            );
        }
    }
    if stdat_flow_ok {
        unsafe {
            skyline::install_hooks!(
                stdat_list_ready_probe,
                stdat_extension_result_probe,
                stdat_register_call_probe,
                stdat_register_return_probe,
                stdat_scan_finish_probe
            );
        }
        skyline::println!(
            "[stdatflow] probes armed at {OFF_STDAT_LIST_READY:#x}/{OFF_STDAT_EXTENSION_RESULT:#x}/\
             {OFF_STDAT_REGISTER_CALL:#x}/{OFF_STDAT_REGISTER_RETURN:#x}/{OFF_STDAT_SCAN_FINISH:#x}"
        );
    }
    skyline::println!(
        "[stagecol] probe armed on the stage update at 0x2cb45b0; \
         {REPORT_CALLS} call(s) per stage, {ELEMENTS_TO_WALK} element(s) each"
    );
    skyline::println!(
        "[stagesetup] probe armed on the pre-setup subobject init at {OFF_SUBOBJECT_INIT:#x}; \
         first {REPORT_INITS} call(s), key and vector bracketed"
    );
}

#[cfg(not(all(not(test), feature = "stage_collision_probe")))]
pub(crate) fn install() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offsets_match_the_disassembly() {
        assert_eq!(VECTOR, 8 + 0x1e0);
        assert_eq!(ELEMENT_INDEX, 0x28);
        assert_eq!(ELEMENT_GATE, 0x25);
        assert_eq!(SUBSYSTEM, 0x738);
        assert_eq!(ELEMENT_SLOT, 0x208);
    }

    #[test]
    fn length_is_derived_the_way_the_binary_derives_it() {
        let (begin, end) = (0x1000i64, 0x1000 + 9 * 4);
        assert_eq!((end - begin) / 4, 9);
    }

    #[test]
    fn a_reversed_vector_is_negative_not_huge() {
        let (begin, end) = (0x2000i64, 0x1000i64);
        assert!((end - begin) / 4 < 0);
    }

    #[test]
    fn an_empty_vector_is_out_of_range_for_every_index() {
        let len: i64 = 0;
        for index in [0i32, 1, 2088643136] {
            assert!((index as i64) < 0 || index as i64 >= len);
        }
    }

    #[test]
    fn the_stage_id_offset_is_the_one_stage_config_reads() {
        assert_eq!(STAGE_ID, 0x8);
    }

    #[test]
    fn the_stdat_flow_sites_match_the_fingerprinted_disassembly() {
        assert_eq!(OFF_STDAT_LIST_READY, 0x25ffa7c);
        assert_eq!(STDAT_LIST_READY_OPCODE, 0xa943e3f3);
        assert_eq!(OFF_STDAT_EXTENSION_RESULT, 0x25ffac0);
        assert_eq!(STDAT_EXTENSION_RESULT_OPCODE, 0x36000220);
        assert_eq!(OFF_STDAT_REGISTER_CALL, 0x25ffb00);
        assert_eq!(STDAT_REGISTER_CALL_OPCODE, 0x94006514);
        assert_eq!(OFF_STDAT_REGISTER_RETURN, 0x25ffb04);
        assert_eq!(STDAT_REGISTER_RETURN_OPCODE, 0x6b1902bf);
        assert_eq!(OFF_STDAT_SCAN_FINISH, 0x25ffc38);
        assert_eq!(STDAT_SCAN_FINISH_OPCODE, 0x9100e3e0);
        assert_eq!(
            RESOURCE_CATEGORY_BASE + STDAT_RESOURCE_CATEGORY * RESOURCE_CATEGORY_STRIDE,
            0x160
        );
    }

    #[test]
    fn the_pre_setup_offsets_match_the_disassembly() {
        assert_eq!(SINGLETON_SLOT, 0x5329000 + 0x9d8);
        assert_eq!(SINGLETON_MAP, 0x1c0);
        assert_eq!(SUBOBJECT_KEY, 0x8);
        assert_eq!(SUBOBJECT_VECTOR, 0xc8);
        assert_eq!(DESCRIPTOR_SETTING, 0x18);
    }

    #[test]
    fn the_pre_setup_vector_has_pointer_stride() {
        let (begin, end) = (0x1000i64, 0x1000 + 7 * 8);
        assert_eq!((end - begin) / 8, 7);
        assert_ne!((end - begin) / 4, 7);
    }

    #[test]
    fn an_empty_pointer_vector_still_gets_element_zero_read() {
        let (begin, end) = (0x1000i64, 0x1000i64);
        assert_eq!((end - begin) / 8, 0);
        assert_eq!(begin, end);
    }

    #[test]
    fn the_subobject_init_is_hooked_at_its_entry() {
        assert_eq!(OFF_SUBOBJECT_INIT, 0x261f760);
    }

    #[test]
    fn the_pictochat_offsets_match_the_disassembly() {
        assert_eq!(OFF_PICT_NAME_INIT, 0x2d9b220);
        assert_eq!(PICT_NAME_INIT_OPCODE, 0xd10303ff);
        assert_eq!(PICT_LANGUAGE_INDEX, 0x523c00c);
        assert_eq!(PICT_LANGUAGE_VECTOR, 0x450);
        assert_eq!(PICT_LANGUAGE_RECORD_SIZE, 0x2c);
        assert_eq!(SUBSYSTEM + PICT_LANGUAGE_VECTOR, 0xb88);
        assert_eq!(OFF_PICT_SCHEMA_RESOLVER, 0x2d9f170);
        assert_eq!(PICT_SCHEMA_RESOLVER_OPCODE, 0xaa0103e9);
        assert_eq!(PICT_LANGUAGE_FIELD, 0x122943f952);
    }

    #[test]
    fn pictochat_vanilla_language_table_has_fourteen_records() {
        let begin = 0x1000i64;
        let end = begin + 14 * PICT_LANGUAGE_RECORD_SIZE as i64;
        assert_eq!((end - begin) / PICT_LANGUAGE_RECORD_SIZE as i64, 14);
        assert_eq!((end - begin) % PICT_LANGUAGE_RECORD_SIZE as i64, 0);
    }
}
