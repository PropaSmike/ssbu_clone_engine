use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const EXPORT: &[u8] = b"hdr_pages_start_page\0";
const EXPORT_OFFSET: usize = 0x2fb48;
const SUBMIT_SLOT: usize = 0x329d88;
const CALL_SITE: usize = 0x45100;
const CALL_SITE_WORDS: [u32; 9] = [
    0x90001720, 0x91362000, 0xf9400003, 0xb94037e0, 0x11000400, 0xf94037e2, 0x2a0003e1, 0xf9401fe0,
    0xd63f0060,
];

static FINISHED: AtomicBool = AtomicBool::new(false);
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static REPORTED: AtomicBool = AtomicBool::new(false);

type Submit = unsafe extern "C" fn(usize, i32, *const usize);

unsafe extern "C" fn guarded_submit(queue: usize, count: i32, list: *const usize) {
    let original: Submit = core::mem::transmute(ORIGINAL.load(Ordering::Acquire));
    let last_is_null = count > 0
        && !list.is_null()
        && core::ptr::read_volatile(list.add(count as usize - 1)) == 0;
    if !last_is_null {
        original(queue, count, list);
        return;
    }
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public("[starlight] project-starlight submitted an empty overlay; that frame goes without it");
    }
    original(queue, count - 1, list);
}

unsafe fn same_build(base: usize) -> bool {
    CALL_SITE_WORDS.iter().enumerate().all(|(index, word)| {
        core::ptr::read_volatile((base + CALL_SITE + index * 4) as *const u32) == *word
    })
}

pub(crate) unsafe fn arm() {
    if FINISHED.load(Ordering::Acquire) {
        return;
    }
    let Some(export) = crate::css_registration::lookup_symbol(EXPORT) else {
        FINISHED.store(true, Ordering::Release);
        return;
    };
    let base = export - EXPORT_OFFSET;
    if !same_build(base) {
        FINISHED.store(true, Ordering::Release);
        crate::dbg_log_public("[starlight] another build of project-starlight; its overlay is not guarded");
        return;
    }
    let slot = (base + SUBMIT_SLOT) as *mut usize;
    let original = core::ptr::read_volatile(slot);
    if original == 0 {
        return;
    }
    FINISHED.store(true, Ordering::Release);
    ORIGINAL.store(original, Ordering::Release);
    core::ptr::write_volatile(slot, guarded_submit as *const () as usize);
    crate::dbg_log_public("[starlight] project-starlight's overlay submit is guarded against an empty overlay");
}
