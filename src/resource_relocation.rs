#![cfg_attr(feature = "native_patch_probe", allow(dead_code))]

use super::native_tables::{
    validate_runtime_opcodes, RESOURCE_ACCESSOR_OFFSETS, RESOURCE_BOUND_PATCHES,
    RESOURCE_CONTAINER_FIELD, RESOURCE_NATIVE_SLOTS, RESOURCE_RESET_FUNCTION,
    RESOURCE_SINGLETON_SLOT, RESOURCE_SLOT_HEADER, RESOURCE_SLOT_STRIDE,
    RESOURCE_TAIL_EXPANDED_OFFSET, RESOURCE_TAIL_NATIVE_OFFSET, RESOURCE_TAIL_PATCHES,
    RESOURCE_TAIL_SIZE, TARGET_SLOTS,
};

use super::text_base;

use std::alloc::{alloc_zeroed, dealloc, Layout};

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const TEMPLATE_SLOT: usize = RESOURCE_NATIVE_SLOTS - 2;

const SACRIFICED_SLOT: usize =
    (RESOURCE_TAIL_NATIVE_OFFSET - RESOURCE_SLOT_HEADER) / RESOURCE_SLOT_STRIDE;
pub(crate) const FIRST_USABLE_SLOT: usize = SACRIFICED_SLOT + 1;

pub(crate) fn last_usable_kind() -> i32 {
    (TARGET_SLOTS - 1) as i32
}

pub(crate) fn capacity_committed() -> bool {
    RELOCATED_BLOCK.load(Ordering::Acquire) != 0
}

static RELOCATED: AtomicBool = AtomicBool::new(false);

static RELOCATED_BLOCK: AtomicUsize = AtomicUsize::new(0);

struct Chain {
    record: usize,

    block: usize,

    siblings: [usize; 3],
}

unsafe fn read_ptr(at: usize) -> Option<usize> {
    if at == 0 || at & 7 != 0 {
        return None;
    }

    let value = core::ptr::read_volatile(at as *const usize);

    (value != 0).then_some(value)
}

unsafe fn read_aligned_ptr(at: usize) -> Option<usize> {
    read_ptr(at).filter(|value| value & 7 == 0)
}

unsafe fn resolve_chain() -> Option<Chain> {
    let singleton = read_aligned_ptr(text_base() + RESOURCE_SINGLETON_SLOT)?;

    let instance = read_aligned_ptr(singleton)?;

    let container = read_aligned_ptr(instance + RESOURCE_CONTAINER_FIELD)?;

    let record = read_aligned_ptr(container)?;

    let block = read_ptr(record)?;

    let mut siblings = [0usize; 3];

    for index in 0..3 {
        let accessor = read_aligned_ptr(record + (index + 1) * 8)?;

        if read_ptr(accessor)? != block + RESOURCE_ACCESSOR_OFFSETS[index + 1] {
            return None;
        }

        siblings[index] = accessor;
    }

    Some(Chain {
        record,

        block,

        siblings,
    })
}

fn expanded_region(slots: usize) -> usize {
    RESOURCE_SLOT_HEADER + slots * RESOURCE_SLOT_STRIDE + RESOURCE_TAIL_SIZE
}

unsafe fn write_instruction(offset: usize, opcode: u32) -> bool {
    crate::text_patch::write_word(text_base() + offset, opcode)
}

unsafe fn instruction_matches(offset: usize, expected: u32) -> bool {
    core::ptr::read_volatile((text_base() + offset) as *const u32) == expected
}

unsafe fn patch_sites_intact() -> bool {
    let mut blocked = 0;

    for (offset, expected, _) in RESOURCE_TAIL_PATCHES.iter().chain(RESOURCE_BOUND_PATCHES) {
        if instruction_matches(*offset, *expected) {
            continue;
        }

        let actual = core::ptr::read_volatile((text_base() + offset) as *const u32);

        skyline::println!(
            "[resreloc] patch site {:#x} is not writable: expected {:#010x}, got {:#010x}{}",
            offset,
            expected,
            actual,
            if super::native_tables::is_foreign_hook(actual) {
                " (another plugin hooked it)"
            } else {
                ""
            }
        );

        blocked += 1;

        if blocked >= 8 {
            break;
        }
    }

    blocked == 0
}

fn patch_runs(patches: &[(usize, u32, u32)]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();

    let mut start = 0usize;

    for index in 1..=patches.len() {
        if index == patches.len() || patches[index].0 != patches[index - 1].0 + 4 {
            runs.push((start, index));

            start = index;
        }
    }

    runs
}

unsafe fn apply_patches(patches: &[(usize, u32, u32)], label: &str) -> bool {
    let runs = patch_runs(patches);

    skyline::println!(
        "[resreloc] {}: {} sites in {} runs, {:#x}..{:#x}",
        label,
        patches.len(),
        runs.len(),
        patches[0].0,
        patches[patches.len() - 1].0
    );

    for (number, (start, end)) in runs.iter().enumerate() {
        let words: Vec<u32> = patches[*start..*end].iter().map(|entry| entry.2).collect();

        let offset = patches[*start].0;

        if number % 32 == 0 {
            skyline::println!(
                "[resreloc] {} run {}/{} @{:#x} x{}",
                label,
                number,
                runs.len(),
                offset,
                words.len()
            );
        }

        if crate::text_patch::write_words(text_base() + offset, &words) {
            continue;
        }

        skyline::println!(
            "[resreloc] {} write failed at {:#x}; reverting {} earlier sites",
            label,
            offset,
            start
        );

        for (revert_offset, original, _) in &patches[..*start] {
            write_instruction(*revert_offset, *original);
        }

        return false;
    }

    skyline::println!("[resreloc] {}: all {} sites written", label, patches.len());

    true
}

unsafe fn build_buffer(block: usize, slots: usize) -> Option<(usize, Layout)> {
    let size = expanded_region(slots);

    let layout = Layout::from_size_align(size, 64).ok()?;

    let buffer = alloc_zeroed(layout);

    if buffer.is_null() {
        return None;
    }

    let base = buffer as usize;

    core::ptr::copy_nonoverlapping(block as *const u8, buffer, RESOURCE_TAIL_NATIVE_OFFSET);

    let template = block + RESOURCE_SLOT_HEADER + TEMPLATE_SLOT * RESOURCE_SLOT_STRIDE;

    for slot in RESOURCE_NATIVE_SLOTS..slots {
        core::ptr::copy_nonoverlapping(
            template as *const u8,
            (base + RESOURCE_SLOT_HEADER + slot * RESOURCE_SLOT_STRIDE) as *mut u8,
            RESOURCE_SLOT_STRIDE,
        );
    }

    core::ptr::copy_nonoverlapping(
        (block + RESOURCE_TAIL_NATIVE_OFFSET) as *const u8,
        (base + RESOURCE_TAIL_EXPANDED_OFFSET) as *mut u8,
        RESOURCE_TAIL_SIZE,
    );

    core::ptr::copy_nonoverlapping(
        (block + RESOURCE_TAIL_NATIVE_OFFSET) as *const u8,
        (base + RESOURCE_TAIL_NATIVE_OFFSET) as *mut u8,
        RESOURCE_TAIL_SIZE,
    );

    Some((base, layout))
}

#[skyline::hook(offset = RESOURCE_RESET_FUNCTION)]

unsafe fn resource_reset_post_hook(object: u64) {
    call_original!(object);

    clear_new_slots();
}

pub(crate) unsafe fn clear_new_slots() {
    let base = RELOCATED_BLOCK.load(Ordering::Acquire);

    if base == 0 {
        return;
    }

    for slot in RESOURCE_NATIVE_SLOTS..TARGET_SLOTS {
        core::ptr::write_volatile((base + slot * RESOURCE_SLOT_STRIDE + 0xd00) as *mut u8, 0);
    }
}

#[allow(dead_code)]

pub(crate) fn relocated_block() -> usize {
    RELOCATED_BLOCK.load(Ordering::Acquire)
}

pub(crate) unsafe fn try_relocate() -> bool {
    if RELOCATED.load(Ordering::Acquire) {
        return true;
    }

    let Some(chain) = resolve_chain() else {
        return false;
    };

    if RELOCATED.swap(true, Ordering::AcqRel) {
        return true;
    }

    if !validate_runtime_opcodes() {
        skyline::println!("[resreloc] aborted: this is not the audited 13.0.4 image");

        return false;
    }

    if !patch_sites_intact() {
        skyline::println!("[resreloc] aborted: a patch site was modified by something else");

        return false;
    }

    let text_len =
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as usize - text_base();

    if let Some((offset, _, _)) = RESOURCE_TAIL_PATCHES
        .iter()
        .chain(RESOURCE_BOUND_PATCHES)
        .find(|(offset, _, _)| *offset + 4 > text_len)
    {
        skyline::println!(
            "[resreloc] aborted: patch offset {:#x} is outside .text (len {:#x})",
            offset,
            text_len
        );

        return false;
    }

    skyline::println!(
        "[resreloc] text={:#x} len={:#x} record={:#x} block={:#x}",
        text_base(),
        text_len,
        chain.record,
        chain.block
    );

    let Some((base, layout)) = build_buffer(chain.block, TARGET_SLOTS) else {
        skyline::println!("[resreloc] aborted: could not allocate the enlarged block");

        return false;
    };

    skyline::println!(
        "[resreloc] staged: old={:#x} new={:#x} slots={}->{} size={:#x}",
        chain.block,
        base,
        RESOURCE_NATIVE_SLOTS,
        TARGET_SLOTS,
        expanded_region(TARGET_SLOTS)
    );

    if !apply_patches(RESOURCE_BOUND_PATCHES, "bound") {
        dealloc(base as *mut u8, layout);

        return false;
    }

    core::ptr::write_volatile(chain.record as *mut usize, base);

    for (index, accessor) in chain.siblings.iter().enumerate() {
        core::ptr::write_volatile(
            *accessor as *mut usize,
            base + RESOURCE_ACCESSOR_OFFSETS[index + 1],
        );
    }

    RELOCATED_BLOCK.store(base, Ordering::Release);

    skyline::println!("[resreloc] pointers committed; patching tail offsets");

    clear_new_slots();
    skyline::println!(
        "[resreloc] COMMITTED: block={:#x} slots={} tail stays at {:#x} in slot {}; custom kinds {}..{} are now allocatable",
        base,
        TARGET_SLOTS,
        RESOURCE_TAIL_NATIVE_OFFSET,
        SACRIFICED_SLOT,
        FIRST_USABLE_SLOT,
        TARGET_SLOTS - 1
    );
    true
}

#[cfg(feature = "native_patch_probe")]

pub(crate) fn start() {
    skyline::println!("[pp] armed; first wake at 20s, then 45s and 90s");

    std::thread::spawn(|| {
        for (attempt, gap) in [20u64, 25, 45].into_iter().enumerate() {
            std::thread::sleep(std::time::Duration::from_secs(gap));

            skyline::println!("[pp] {} wake", attempt);

            let first = RESOURCE_TAIL_PATCHES[0];

            let value = unsafe { core::ptr::read_volatile((text_base() + first.0) as *const u32) };

            skyline::println!(
                "[pp] {} one read @{:#x} = {:#010x}",
                attempt,
                first.0,
                value
            );

            let mut untouched = 0usize;

            for (index, (offset, expected, _)) in RESOURCE_TAIL_PATCHES.iter().enumerate() {
                if index % 64 == 0 {
                    skyline::println!(
                        "[pp] {} read {}/{}",
                        attempt,
                        index,
                        RESOURCE_TAIL_PATCHES.len()
                    );
                }

                if unsafe { core::ptr::read_volatile((text_base() + offset) as *const u32) }
                    == *expected
                {
                    untouched += 1;
                }
            }

            skyline::println!("[pp] {} reads done, {} untouched", attempt, untouched);

            let buffer = vec![0u8; expanded_region(TARGET_SLOTS)];

            skyline::println!("[pp] {} alloc ok @{:#x}", attempt, buffer.as_ptr() as usize);

            drop(buffer);

            skyline::println!("[pp] {} free ok", attempt);

            let ok = unsafe { write_instruction(first.0, value) };

            skyline::println!("[pp] {} single write ok={}", attempt, ok);

            let identity: Vec<(usize, u32, u32)> = RESOURCE_TAIL_PATCHES
                .iter()
                .filter(|(offset, expected, _)| unsafe { instruction_matches(*offset, *expected) })
                .map(|(offset, expected, _)| (*offset, *expected, *expected))
                .collect();

            skyline::println!("[pp] {} burst: {} inert sites", attempt, identity.len());

            let burst = unsafe { apply_patches(&identity, "identity") };

            skyline::println!("[pp] {} burst ok={}", attempt, burst);
        }

        skyline::println!("[pp] all attempts survived");
    });
}

#[cfg(not(feature = "native_patch_probe"))]

pub(crate) fn start() {
    skyline::println!(
        "[resreloc] armed: {} -> {} slots, driven from the resource-manager hook on a game thread",
        RESOURCE_NATIVE_SLOTS,
        TARGET_SLOTS
    );

    skyline::install_hook!(resource_reset_post_hook);
}

#[cfg(not(feature = "native_patch_probe"))]

pub(crate) unsafe fn on_resource_manager_insert() {
    if RELOCATED.load(Ordering::Acquire) {
        return;
    }

    try_relocate();
}
