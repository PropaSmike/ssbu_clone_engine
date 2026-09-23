use core::sync::atomic::{AtomicU32, Ordering};

const MODEL_BUILD: usize = 0x35c5610;
const FILESYSTEM: usize = 0x533af20;
const NOT_FOUND: u32 = 0xffffff;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const RECORD_STRIDE: usize = 0x18;
const MODEL_NUMDLB: &[u8] = b"model.numdlb\0";

const CONTEXT_REPORTS: u32 = 48;
const FATAL_REPORTS: u32 = 24;

static CONTEXT_LOG: AtomicU32 = AtomicU32::new(0);
static FATAL_LOG: AtomicU32 = AtomicU32::new(0);
static GUARDED: AtomicU32 = AtomicU32::new(0);

#[skyline::from_offset(0x35c5370)]
fn resolve_relative_path(search_path: u32, relative: *const u8) -> u32;

#[skyline::from_offset(0x3543780)]
fn add_to_res_service(service: usize, file_path: u32);

struct Residency {
    file_path: u32,
    service: usize,
    count_a: u32,
    slot: usize,
    flag: u8,
    entry: u32,
    count_b: u32,
    record: usize,
    record_flag: u8,
    stop: &'static str,
}

impl Residency {
    const fn blank() -> Self {
        Self {
            file_path: NOT_FOUND,
            service: 0,
            count_a: 0,
            slot: 0,
            flag: 0,
            entry: NOT_FOUND,
            count_b: 0,
            record: 0,
            record_flag: 0,
            stop: "unresolved",
        }
    }

    fn resident(&self) -> bool {
        self.stop == "resident"
    }

    fn fatal(&self) -> bool {
        self.file_path != NOT_FOUND && !self.resident()
    }
}

unsafe fn inspect(search_path: u32) -> Residency {
    let mut out = Residency::blank();
    if search_path == NOT_FOUND {
        return out;
    }
    let service = core::ptr::read_volatile((crate::text_base() + FILESYSTEM) as *const usize);
    out.service = service;
    if service < LOWEST_PLAUSIBLE_POINTER {
        out.stop = "service";
        return out;
    }
    if core::ptr::read_volatile((service + 0x78) as *const usize) < LOWEST_PLAUSIBLE_POINTER {
        out.stop = "tables";
        return out;
    }
    out.file_path = resolve_relative_path(search_path, MODEL_NUMDLB.as_ptr());
    if out.file_path == NOT_FOUND {
        return out;
    }
    out.count_a = core::ptr::read_volatile((service + 0x18) as *const u32);
    if out.file_path >= out.count_a {
        out.stop = "count_a";
        return out;
    }
    let table_a = core::ptr::read_volatile((service + 0x08) as *const usize);
    if table_a < LOWEST_PLAUSIBLE_POINTER {
        out.stop = "table_a";
        return out;
    }
    let slot = table_a + out.file_path as usize * 8;
    out.slot = slot;
    out.flag = core::ptr::read_volatile((slot + 4) as *const u8);
    if out.flag == 0 {
        out.stop = "flag";
        return out;
    }
    out.entry = core::ptr::read_volatile(slot as *const u32);
    if out.entry == NOT_FOUND {
        out.stop = "entry";
        return out;
    }
    out.count_b = core::ptr::read_volatile((service + 0x1c) as *const u32);
    if out.entry >= out.count_b {
        out.stop = "count_b";
        return out;
    }
    let table_b = core::ptr::read_volatile((service + 0x10) as *const usize);
    if table_b < LOWEST_PLAUSIBLE_POINTER {
        out.stop = "table_b";
        return out;
    }
    let record = table_b + out.entry as usize * RECORD_STRIDE;
    out.record = record;
    out.record_flag = core::ptr::read_volatile((record + 0xc) as *const u8);
    if out.record_flag == 0 {
        out.stop = "record";
        return out;
    }
    out.stop = "resident";
    out
}

pub(crate) unsafe fn model_residency(search_path: u32) -> (&'static str, u32) {
    let state = inspect(search_path);
    (state.stop, state.file_path)
}

fn ticket(counter: &AtomicU32, cap: u32) -> Option<u32> {
    let n = counter.fetch_add(1, Ordering::Relaxed);
    (n < cap).then_some(n)
}

#[skyline::hook(offset = MODEL_BUILD)]
pub(crate) unsafe fn model_build_probe(
    out: *mut u64,
    search_path: *const u32,
    variant: u32,
) -> u64 {
    let search = if search_path.is_null() {
        NOT_FOUND
    } else {
        core::ptr::read_volatile(search_path)
    };
    let copy_kind = crate::kirby_copy::active_kirby_copy_kind();
    let state = inspect(search);
    let doomed = state.fatal();
    let slip = if doomed {
        ticket(&FATAL_LOG, FATAL_REPORTS)
    } else if copy_kind.is_some() {
        ticket(&CONTEXT_LOG, CONTEXT_REPORTS)
    } else {
        None
    };
    if let Some(n) = slip {
        let tag = if doomed { "GUARDED" } else { "context" };
        crate::dbg_log_public(&format!(
            "[copymodel] #{n} {tag} search={search:#x} file={:#x} stop={} copy_kind={copy_kind:?} variant={variant} count_a={} slot={:#x} flag={} entry={:#x} count_b={} record={:#x} record_flag={} thread={:#x}",
            state.file_path,
            state.stop,
            state.count_a,
            state.slot,
            state.flag,
            state.entry,
            state.count_b,
            state.record,
            state.record_flag,
            crate::current_thread_key()
        ));
    }
    if doomed {
        GUARDED.fetch_add(1, Ordering::Relaxed);
        if state.service >= LOWEST_PLAUSIBLE_POINTER {
            add_to_res_service(state.service, state.file_path);
        }
        if !out.is_null() {
            core::ptr::write_volatile(out, 0);
            core::ptr::write_volatile(out.add(1), 0);
        }
        return state.file_path as u64;
    }
    let result = call_original!(out, search_path, variant);
    if let Some(n) = slip {
        crate::dbg_log_public(&format!(
            "[copymodel] #{n} exit search={search:#x} result={result:#x} out0={:#x} out1={:#x}",
            core::ptr::read_volatile(out),
            core::ptr::read_volatile(out.add(1))
        ));
    }
    result
}

pub(crate) fn guarded() -> u32 {
    GUARDED.load(Ordering::Relaxed)
}

pub(crate) fn install() {
    skyline::install_hooks!(model_build_probe);
    crate::dbg_log_public(
        "[copymodel] armed at 0x35c5610. Its not-found path at 0x35c56d4 sets x8 to zero and the \
         very next instruction loads from x8+0x20, so a model.numdlb that resolves to a file path \
         whose resource record is not resident is an unconditional null dereference with no \
         recovery. add_to_res_service at 0x3543780 only queues an async load and never populates \
         the record, so the residency verdict taken here is the one the game itself reaches. A \
         resolved but non resident build is guarded: the load is queued, the out struct is zeroed \
         exactly as 0x35c565c does for an unresolved path, and stop= names the check that failed.",
    );
}
