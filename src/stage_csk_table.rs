#![allow(dead_code)]

pub use clone_engine_core::csk_stage_vec::{decode, plan, Growth, VecFields, ROW};

#[cfg(not(test))]
pub unsafe fn apply(growth: &Growth, buffer: *mut u8) -> *mut u8 {
    if buffer.is_null() {
        skyline::println!("[stagecsk] no storage reserved for the grown stage_id table");
        return core::ptr::null_mut();
    }

    core::ptr::copy_nonoverlapping(growth.source as *const u8, buffer, growth.copy_rows * ROW);

    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::ptr::write_volatile(growth.fields.pointer_at as *mut usize, buffer as usize);
    core::ptr::write_volatile(
        growth.fields.capacity_at as *mut usize,
        growth.capacity_rows,
    );
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::ptr::write_volatile(growth.fields.length_at as *mut usize, growth.copy_rows);

    skyline::println!(
        "[stagecsk] grew CSK's stage_id table: {} rows copied into {} rows at {:p} \
         (was {:#x}); length stays {} until rows are published",
        growth.copy_rows,
        growth.capacity_rows,
        buffer,
        growth.source,
        growth.copy_rows,
    );
    buffer
}

#[cfg(not(test))]
pub unsafe fn publish_length(fields: VecFields, rows: usize) {
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::ptr::write_volatile(fields.length_at as *mut usize, rows);
}

#[cfg(not(test))]
pub unsafe fn resolve_csk() -> Option<usize> {
    let mut address = 0usize;
    let result = skyline::nn::ro::LookupSymbol(
        &mut address as *mut usize,
        b"setup_stage_offseted\0".as_ptr(),
    );
    (result == 0 && address != 0).then_some(address)
}
