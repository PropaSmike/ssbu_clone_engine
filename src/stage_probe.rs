use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::stage_bounds::{
    StageTable, QUALIFIED_BOUND_COUNT, REFERENCE_COUNT, REFERENCE_SPAN, STAGE_OPCODE_ANCHORS,
    STAGE_TABLES, UNWIDENED_BOUNDS,
};

const BRANCH_REACH: i64 = 1 << 27;

const ADRP_REACH: i64 = 1 << 32;

const REQUIRED_BYTES: u64 = 192 * 0x28 + 192 * 0x20 + 512 * 0x48;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attribute: u32,
    permission: u32,
    ipc_refcount: u32,
    device_refcount: u32,
    padding: u32,
}

const MEMORY_FREE: u32 = 0x00;

unsafe fn query_memory(info: *mut MemoryInfo, address: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x6",
        inout("x0") info as u64 => result,
        out("x1") _,
        in("x2") address,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn map_physical(address: u64, size: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x2c",
        inout("x0") address => result,
        in("x1") size,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn unmap_physical(address: u64, size: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x2d",
        inout("x0") address => result,
        in("x1") size,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn map_memory(destination: u64, source: u64, size: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x4",
        inout("x0") destination => result,
        in("x1") source,
        in("x2") size,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn unmap_memory(destination: u64, source: u64, size: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x5",
        inout("x0") destination => result,
        in("x1") source,
        in("x2") size,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

const CURRENT_PROCESS: u32 = 0xFFFF_8001;

unsafe fn get_info(id: u32, handle: u32, sub_id: u64) -> (u32, u64) {
    let result: u64;
    let value: u64;
    core::arch::asm!(
        "svc 0x29",
        out("x0") result,
        inout("x1") id as u64 => value,
        in("x2") handle as u64,
        in("x3") sub_id,
        clobber_abi("C"),
        options(nostack),
    );
    (result as u32, value)
}

unsafe fn map_process_code(handle: u32, destination: u64, source: u64, size: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x77",
        inout("x0") handle as u64 => result,
        in("x1") destination,
        in("x2") source,
        in("x3") size,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn unmap_process_code(handle: u32, destination: u64, source: u64, size: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x78",
        inout("x0") handle as u64 => result,
        in("x1") destination,
        in("x2") source,
        in("x3") size,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

fn report_memory_regions(site_lo: usize, site_hi: usize) {
    let window_lo = (site_hi as i64 - ADRP_REACH).max(0) as u64;
    let window_hi = (site_lo as i64 + ADRP_REACH) as u64;
    const REGIONS: &[(&str, u32, u32)] = &[
        ("alias", 2, 3),
        ("heap", 4, 5),
        ("aslr", 12, 13),
        ("stack", 14, 15),
    ];
    for (name, address_id, size_id) in REGIONS {
        let (r1, address) = unsafe { get_info(*address_id, CURRENT_PROCESS, 0) };
        let (r2, size) = unsafe { get_info(*size_id, CURRENT_PROCESS, 0) };
        if r1 != 0 || r2 != 0 {
            skyline::println!(
                "[stageprobe] region {}: query failed {:#x}/{:#x}",
                name,
                r1,
                r2
            );
            continue;
        }
        let end = address.saturating_add(size);
        let overlap_lo = address.max(window_lo);
        let overlap_hi = end.min(window_hi);
        let overlap = overlap_hi.saturating_sub(overlap_lo);
        skyline::println!(
            "[stageprobe] region {:<5} {:#x}..{:#x} ({} MiB); ADRP overlap {:#x}..{:#x} = {} MiB {}",
            name,
            address,
            end,
            size / (1024 * 1024),
            overlap_lo,
            overlap_hi,
            overlap / (1024 * 1024),
            if overlap >= REQUIRED_BYTES { "USABLE" } else { "too small" }
        );
    }
}

const STAGE_ID_NAME: usize = 0x10;
const STAGE_PLACE_NAME: usize = 0x08;

const HASH_BATTLEFIELD: u64 = 0x0B_51B7_F6D5;
const HASH_PHOTOSTAGE: u64 = 0x0A_1E68_5186;
const HASH_END: u64 = 0x03_00FC_33B1;

const PHOTOSTAGE_ID: usize = 326;
const PHOTOSTAGE_PLACE: usize = 122;

static RAN: AtomicBool = AtomicBool::new(false);

static SLACK_START: AtomicU64 = AtomicU64::new(0);
static SLACK_LEN: AtomicU64 = AtomicU64::new(0);

unsafe fn read_u32(address: usize) -> u32 {
    core::ptr::read_volatile(address as *const u32)
}

unsafe fn read_u64(address: usize) -> u64 {
    core::ptr::read_volatile(address as *const u64)
}

fn table(name: &str) -> Option<&'static StageTable> {
    STAGE_TABLES.iter().find(|entry| entry.name == name)
}

fn reach(from: usize, to: usize) -> (&'static str, i64) {
    let displacement = to as i64 - from as i64;
    let verdict = if displacement.abs() < BRANCH_REACH {
        "IN RANGE"
    } else {
        "OUT OF RANGE"
    };
    (verdict, displacement)
}

pub(crate) fn run() {
    if RAN.swap(true, Ordering::AcqRel) {
        return;
    }
    let text = crate::text_base();
    skyline::println!(
        "[stageprobe] begin: text={:#x} refs={} bounds={} tables={}",
        text,
        REFERENCE_COUNT,
        QUALIFIED_BOUND_COUNT,
        STAGE_TABLES.len()
    );
    skyline::println!(
        "[stageprobe] bounds not widened by this build: {}",
        UNWIDENED_BOUNDS
    );

    let mut mismatches = 0usize;
    for anchor in STAGE_OPCODE_ANCHORS {
        let actual = unsafe { read_u32(text + anchor.offset) };
        if actual != anchor.opcode {
            if mismatches < 8 {
                skyline::println!(
                    "[stageprobe] OPCODE MISMATCH {:#x} ({}): want {:#010x} got {:#010x}",
                    anchor.offset,
                    anchor.label,
                    anchor.opcode,
                    actual
                );
            }
            mismatches += 1;
        }
    }
    skyline::println!(
        "[stageprobe] opcodes: {} checked, {} mismatched",
        STAGE_OPCODE_ANCHORS.len(),
        mismatches
    );

    for entry in STAGE_TABLES {
        let base = text + entry.address;
        skyline::println!(
            "[stageprobe] {} @{:#x} stride={:#x} len={} -> expanded {} (end {:#x})",
            entry.name,
            base,
            entry.element_size,
            entry.native_length,
            entry.expanded_length,
            base + entry.element_size * entry.native_length
        );
    }

    if let Some(entry) = table("stage_id") {
        let base = text + entry.address;
        let first = unsafe { read_u64(base + STAGE_ID_NAME) };
        let end_row = unsafe { read_u64(base + 3 * entry.element_size + STAGE_ID_NAME) };
        let photo = unsafe { read_u64(base + PHOTOSTAGE_ID * entry.element_size + STAGE_ID_NAME) };
        let photo_place =
            unsafe { read_u32(base + PHOTOSTAGE_ID * entry.element_size + 4) } as usize;
        let last_id = unsafe { read_u32(base + (entry.native_length - 1) * entry.element_size) };
        skyline::println!(
            "[stageprobe] stage_id[0].name={:#x} ({}) [3].name={:#x} ({})",
            first,
            first == HASH_BATTLEFIELD,
            end_row,
            end_row == HASH_END
        );
        skyline::println!(
            "[stageprobe] stage_id[326].name={:#x} ({}) place={} ({}) last_id={} ({})",
            photo,
            photo == HASH_PHOTOSTAGE,
            photo_place,
            photo_place == PHOTOSTAGE_PLACE,
            last_id,
            last_id as usize == entry.native_length - 1
        );
    }

    if let Some(entry) = table("stage_place") {
        let base = text + entry.address;
        let first = unsafe { read_u64(base + STAGE_PLACE_NAME) };
        let photo =
            unsafe { read_u64(base + PHOTOSTAGE_PLACE * entry.element_size + STAGE_PLACE_NAME) };
        skyline::println!(
            "[stageprobe] stage_place[0].name={:#x} ({}) [122].name={:#x} ({})",
            first,
            first == HASH_BATTLEFIELD,
            photo,
            photo == HASH_PHOTOSTAGE
        );
    }

    if let Some(entry) = table("stage_place_aux") {
        let base = text + entry.address;
        let first = unsafe { read_u32(base) };
        let last = unsafe { read_u32(base + (entry.native_length - 1) * entry.element_size) };
        skyline::println!(
            "[stageprobe] stage_place_aux[0].index={} ({}) [137].index={} ({})",
            first,
            first == 0,
            last,
            last as usize == entry.native_length - 1
        );
    }

    let (span_lo, span_hi) = REFERENCE_SPAN;
    let site_lo = text + span_lo;
    let site_hi = text + span_hi;
    let own_code = run as usize;
    let (lo_verdict, lo_delta) = reach(site_lo, own_code);
    let (hi_verdict, hi_delta) = reach(site_hi, own_code);
    skyline::println!(
        "[stageprobe] reference span {:#x}..{:#x} ({} MiB)",
        site_lo,
        site_hi,
        (site_hi - site_lo) / (1024 * 1024)
    );
    skyline::println!(
        "[stageprobe] plugin code @{:#x}: from lo {} ({} MiB), from hi {} ({} MiB)",
        own_code,
        lo_verdict,
        lo_delta / (1024 * 1024),
        hi_verdict,
        hi_delta / (1024 * 1024)
    );

    let probe = vec![0u8; 0x100];
    let heap = probe.as_ptr() as usize;
    let (heap_verdict, heap_delta) = reach(site_lo, heap);
    skyline::println!(
        "[stageprobe] heap @{:#x}: from lo {} ({} MiB)",
        heap,
        heap_verdict,
        heap_delta / (1024 * 1024)
    );
    drop(probe);

    survey_address_space(site_lo, site_hi);

    skyline::println!(
        "[stageprobe] end: {}",
        if mismatches == 0 {
            "census matches the running image"
        } else {
            "CENSUS DISAGREES WITH THE RUNNING IMAGE - do not proceed"
        }
    );
}

fn survey_address_space(site_lo: usize, site_hi: usize) {
    let window_lo = (site_hi as i64 - ADRP_REACH).max(0) as u64;
    let window_hi = (site_lo as i64 + ADRP_REACH) as u64;
    skyline::println!(
        "[stageprobe] need {} bytes within ADRP reach: window {:#x}..{:#x}",
        REQUIRED_BYTES,
        window_lo,
        window_hi
    );

    report_memory_regions(site_lo, site_hi);

    let mut info = MemoryInfo::default();
    let mut address: u64 = 0;
    let mut regions = 0usize;
    let mut free_in_window = 0usize;
    let mut best: Option<(u64, u64)> = None;
    let mut printed = 0usize;
    let mut mapped_printed = 0usize;
    let mut game_rw: Option<(u64, u64)> = None;
    let text = crate::text_base() as u64;

    while regions < 4096 {
        let result = unsafe { query_memory(&mut info, address) };
        if result != 0 {
            skyline::println!(
                "[stageprobe] svcQueryMemory({:#x}) failed: {:#x} after {} regions",
                address,
                result,
                regions
            );
            break;
        }
        if info.size == 0 {
            break;
        }
        regions += 1;

        if info.kind != MEMORY_FREE
            && info.address < window_hi
            && info.address + info.size > window_lo
            && mapped_printed < 24
        {
            skyline::println!(
                "[stageprobe] mapped {:#x}..{:#x} kind={:#x} perm={:#x} attr={:#x}",
                info.address,
                info.address + info.size,
                info.kind,
                info.permission,
                info.attribute
            );
            mapped_printed += 1;
        }

        if info.kind != MEMORY_FREE
            && info.permission == 3
            && info.address >= text
            && info.size >= REQUIRED_BYTES
            && game_rw.is_none()
        {
            game_rw = Some((info.address, info.address + info.size));
        }

        if info.kind == MEMORY_FREE {
            let start = info.address.max(window_lo);
            let end = (info.address + info.size).min(window_hi);
            if end > start {
                let usable = end - start;
                if usable >= REQUIRED_BYTES {
                    free_in_window += 1;
                    if best.map_or(true, |(_, size)| usable > size) {
                        best = Some((start, usable));
                    }
                    if printed < 8 {
                        skyline::println!(
                            "[stageprobe] free {:#x} size {:#x} -> usable {:#x} ({} MiB) IN REACH",
                            info.address,
                            info.size,
                            usable,
                            usable / (1024 * 1024)
                        );
                        printed += 1;
                    }
                }
            }
        }

        let next = info.address.wrapping_add(info.size);
        if next <= address {
            break;
        }
        address = next;
        if address >= window_hi {
            break;
        }
    }

    skyline::println!(
        "[stageprobe] surveyed {} regions; {} free regions can host the tables",
        regions,
        free_in_window
    );
    match best {
        Some((start, size)) => {
            skyline::println!(
                "[stageprobe] VERDICT: in-place ADRP rewrite is VIABLE; best candidate {:#x} ({} MiB)",
                start,
                size / (1024 * 1024)
            );
            attempt_mapping(start);
        }
        None => skyline::println!(
            "[stageprobe] VERDICT: no free region within ADRP reach; relocation needs another home"
        ),
    }

    match game_rw {
        Some((start, end)) => scan_game_rw_for_slack(start, end),
        None => skyline::println!("[stageprobe] game RW region not found; nothing to scan"),
    }
}

fn scan_game_rw_for_slack(start: u64, end: u64) {
    const STEP: usize = 0x1000;

    let mut info = MemoryInfo::default();
    let result = unsafe { query_memory(&mut info, start) };
    if result != 0 || info.kind == MEMORY_FREE || info.permission & 1 == 0 {
        skyline::println!(
            "[stageprobe] slack: {:#x} is not readable (query {:#x} kind {:#x} perm {:#x}); skipped",
            start,
            result,
            info.kind,
            info.permission
        );
        return;
    }
    let limit = (info.address + info.size).min(end);
    let length = limit.saturating_sub(start) as usize;
    if length < STEP {
        skyline::println!("[stageprobe] slack: mapped range too short; skipped");
        return;
    }

    let mut best_start = 0usize;
    let mut best_len = 0usize;
    let mut run_start = 0usize;
    let mut run_len = 0usize;
    let mut zero_pages = 0usize;

    let mut offset = 0usize;
    while offset + STEP <= length {
        let mut page_zero = true;
        let mut at = 0usize;
        while at < STEP {
            if unsafe { read_u64(start as usize + offset + at) } != 0 {
                page_zero = false;
                break;
            }
            at += 8;
        }
        if page_zero {
            if run_len == 0 {
                run_start = offset;
            }
            run_len += STEP;
            zero_pages += 1;
            if run_len > best_len {
                best_len = run_len;
                best_start = run_start;
            }
        } else {
            run_len = 0;
        }
        offset += STEP;
    }

    skyline::println!(
        "[stageprobe] game RW {:#x}..{:#x} ({:#x}): {} zero pages, longest run {:#x} at {:#x}",
        start,
        limit,
        length,
        zero_pages,
        best_len,
        start as usize + best_start
    );
    if best_len as u64 >= REQUIRED_BYTES {
        SLACK_START.store(start + best_start as u64, Ordering::Release);
        SLACK_LEN.store(best_len as u64, Ordering::Release);
        spawn_slack_monitor();
    }
    skyline::println!(
        "[stageprobe] slack verdict: {} (need {:#x}) -- boot-time only, recheck after a match",
        if best_len as u64 >= REQUIRED_BYTES {
            "sufficient at boot"
        } else {
            "INSUFFICIENT"
        },
        REQUIRED_BYTES
    );
}

fn attempt_mapping(candidate: u64) {
    let address = (candidate + 0xFFF) & !0xFFF;
    const SIZE: u64 = 0x10000;

    let result = unsafe { map_physical(address, SIZE) };
    if result == 0 {
        let ok = verify_mapping(address);
        let unmapped = unsafe { unmap_physical(address, SIZE) };
        skyline::println!(
            "[stageprobe] MAP svcMapPhysicalMemory({:#x}, {:#x}) OK; readback {}; unmap {:#x}",
            address,
            SIZE,
            if ok { "correct" } else { "WRONG" },
            unmapped
        );
        return;
    }
    skyline::println!(
        "[stageprobe] MAP svcMapPhysicalMemory({:#x}, {:#x}) failed {:#x}",
        address,
        SIZE,
        result
    );

    let mut backing = vec![0u8; SIZE as usize + 0x1000];
    let source = (backing.as_mut_ptr() as u64 + 0xFFF) & !0xFFF;
    let result = unsafe { map_memory(address, source, SIZE) };
    if result == 0 {
        let ok = verify_mapping(address);
        let unmapped = unsafe { unmap_memory(address, source, SIZE) };
        skyline::println!(
            "[stageprobe] MAP svcMapMemory({:#x} <- {:#x}) OK; readback {}; unmap {:#x}",
            address,
            source,
            if ok { "correct" } else { "WRONG" },
            unmapped
        );
    } else {
        skyline::println!(
            "[stageprobe] MAP svcMapMemory({:#x} <- {:#x}) failed {:#x}",
            address,
            source,
            result
        );

        let result = unsafe { map_process_code(CURRENT_PROCESS, address, source, SIZE) };
        if result == 0 {
            let ok = verify_mapping(address);
            let unmapped = unsafe { unmap_process_code(CURRENT_PROCESS, address, source, SIZE) };
            skyline::println!(
                "[stageprobe] MAP svcMapProcessCodeMemory({:#x} <- {:#x}) OK; readback {}; unmap {:#x}",
                address,
                source,
                if ok { "correct" } else { "WRONG" },
                unmapped
            );
        } else {
            skyline::println!(
                "[stageprobe] MAP svcMapProcessCodeMemory({:#x} <- {:#x}) failed {:#x}",
                address,
                source,
                result
            );
            skyline::println!(
                "[stageprobe] MAP: no call backed the address; see the region report above"
            );
        }
    }
    drop(backing);
}

fn verify_mapping(address: u64) -> bool {
    const PATTERN: u64 = 0x5347_5F50_524F_4245;
    unsafe {
        let cell = address as *mut u64;
        cell.write_volatile(PATTERN);
        let last = (address + 0xFFF8) as *mut u64;
        last.write_volatile(!PATTERN);
        cell.read_volatile() == PATTERN && last.read_volatile() == !PATTERN
    }
}

fn spawn_slack_monitor() {
    std::thread::spawn(|| {
        for (label, delay) in [("t+60s", 60u64), ("t+150s", 90), ("t+300s", 150)] {
            std::thread::sleep(std::time::Duration::from_secs(delay));
            recheck_slack(label);
        }
    });
}

fn recheck_slack(label: &str) {
    let start = SLACK_START.load(Ordering::Acquire);
    let len = SLACK_LEN.load(Ordering::Acquire);
    if start == 0 || len == 0 {
        return;
    }

    let mut info = MemoryInfo::default();
    let result = unsafe { query_memory(&mut info, start) };
    if result != 0 || info.kind == MEMORY_FREE || info.permission & 1 == 0 {
        skyline::println!(
            "[stageprobe] slack {}: range no longer readable (query {:#x} kind {:#x})",
            label,
            result,
            info.kind
        );
        return;
    }
    let limit = (info.address + info.size).min(start + len);

    let mut first_dirty: Option<u64> = None;
    let mut dirty_pages = 0usize;
    let mut best_len = 0usize;
    let mut best_start = start;
    let mut run_len = 0usize;
    let mut run_start = start;

    let mut at = start;
    while at + 0x1000 <= limit {
        let mut page_zero = true;
        let mut offset = 0usize;
        while offset < 0x1000 {
            if unsafe { read_u64((at + offset as u64) as usize) } != 0 {
                page_zero = false;
                if first_dirty.is_none() {
                    first_dirty = Some(at + offset as u64);
                }
                break;
            }
            offset += 8;
        }
        if page_zero {
            if run_len == 0 {
                run_start = at;
            }
            run_len += 0x1000;
            if run_len > best_len {
                best_len = run_len;
                best_start = run_start;
            }
        } else {
            dirty_pages += 1;
            run_len = 0;
        }
        at += 0x1000;
    }

    skyline::println!(
        "[stageprobe] slack {}: {:#x} pages dirtied; longest still-zero run {:#x} at {:#x}; first dirty {:#x}; {}",
        label,
        dirty_pages,
        best_len,
        best_start,
        first_dirty.unwrap_or(0),
        if best_len as u64 >= REQUIRED_BYTES { "still sufficient" } else { "NO LONGER SUFFICIENT" }
    );
}
