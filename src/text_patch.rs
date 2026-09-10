#![allow(dead_code)]

pub(crate) use clone_engine_core::{plan_chunks, Chunk, CHECK_WINDOW, PAGE};

pub(crate) unsafe fn write_bytes(address: usize, bytes: &[u8]) -> bool {
    let mut done = 0usize;
    for chunk in plan_chunks(address, bytes.len()) {
        let mut buffer = Vec::with_capacity(chunk.lead + chunk.len);
        buffer.extend_from_slice(core::slice::from_raw_parts(
            chunk.start as *const u8,
            chunk.lead,
        ));
        buffer.extend_from_slice(&bytes[done..done + chunk.len]);
        let result =
            skyline::patching::sky_memcpy(chunk.start as _, buffer.as_ptr() as _, buffer.len());
        if result.0.is_some() {
            return false;
        }
        done += chunk.len;
    }
    flush_instruction_cache(address, bytes.len());
    true
}

const CACHE_LINE: usize = 64;

#[cfg(target_arch = "aarch64")]
unsafe fn flush_instruction_cache(address: usize, len: usize) {
    if len == 0 {
        return;
    }
    let start = address & !(CACHE_LINE - 1);
    let end = address + len;
    let mut line = start;
    while line < end {
        core::arch::asm!("dc cvau, {line}", line = in(reg) line, options(nostack, preserves_flags));
        line += CACHE_LINE;
    }
    core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    let mut line = start;
    while line < end {
        core::arch::asm!("ic ivau, {line}", line = in(reg) line, options(nostack, preserves_flags));
        line += CACHE_LINE;
    }
    core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn flush_instruction_cache(_address: usize, _len: usize) {}

pub(crate) unsafe fn write_words(address: usize, words: &[u32]) -> bool {
    write_bytes(
        address,
        core::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 4),
    )
}

pub(crate) unsafe fn write_word(address: usize, word: u32) -> bool {
    write_words(address, &[word])
}
