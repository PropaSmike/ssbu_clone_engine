use core::sync::atomic::{AtomicU32, Ordering};

const MODEL_BUILD: usize = 0x35c22e0;
const CHECKS_FAILED: usize = 0x35c23a4;
const CHECKS_FAILED_WORD: u32 = 0xaa1f_03e8;
const EMPTY_AND_RELEASE: usize = 0x35c2464;
const FILESYSTEM: usize = 0x5331f20;
const NOT_FOUND: u32 = 0xffffff;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const RECORD_STRIDE: usize = 0x18;
const MODEL_NUMDLB: &[u8] = b"model.numdlb\0";

const CONTEXT_REPORTS: u32 = 48;
const EMPTY_REPORTS: u32 = 24;

static CONTEXT_LOG: AtomicU32 = AtomicU32::new(0);
static EMPTY_LOG: AtomicU32 = AtomicU32::new(0);

#[skyline::from_offset(0x35c2040)]
fn resolve_relative_path(search_path: u32, relative: *const u8) -> u32;

#[skyline::from_offset(0x353e4e0)]
fn file_path_of_search_path(search_path: u32) -> u32;

struct Residency {
    file_path: u32,
    count_a: u32,
    slot: usize,
    flag: u8,
    entry: u32,
    count_b: u32,
    linked: u32,
    record: usize,
    record_flag: u8,
    stop: &'static str,
}

impl Residency {
    const fn blank() -> Self {
        Self {
            file_path: NOT_FOUND,
            count_a: 0,
            slot: 0,
            flag: 0,
            entry: NOT_FOUND,
            count_b: 0,
            linked: NOT_FOUND,
            record: 0,
            record_flag: 0,
            stop: "unresolved",
        }
    }

    fn loads(&self) -> bool {
        self.stop == "resident" || self.stop == "linkable"
    }
}

unsafe fn service() -> Result<usize, &'static str> {
    let service = core::ptr::read_volatile((crate::text_base() + FILESYSTEM) as *const usize);
    if service < LOWEST_PLAUSIBLE_POINTER {
        return Err("service");
    }
    if core::ptr::read_volatile((service + 0x78) as *const usize) < LOWEST_PLAUSIBLE_POINTER {
        return Err("tables");
    }
    Ok(service)
}

unsafe fn resident_record(out: &mut Residency, table_b: usize, entry: u32) -> bool {
    if entry == NOT_FOUND {
        out.stop = "entry";
        return false;
    }
    if entry >= out.count_b {
        out.stop = "count_b";
        return false;
    }
    out.record = table_b + entry as usize * RECORD_STRIDE;
    out.record_flag = core::ptr::read_volatile((out.record + 0xc) as *const u8);
    if out.record_flag == 0 {
        out.stop = "record";
        return false;
    }
    true
}

unsafe fn linked_index(service: usize, file_path: u32) -> Option<u32> {
    let arc = core::ptr::read_volatile(core::ptr::read_volatile((service + 0x78) as *const usize) as *const usize);
    if arc < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let paths = core::ptr::read_volatile((arc + 0x60) as *const usize);
    let counts = core::ptr::read_volatile((arc + 0x40) as *const usize);
    if paths < LOWEST_PLAUSIBLE_POINTER
        || counts < LOWEST_PLAUSIBLE_POINTER
        || file_path >= core::ptr::read_volatile((counts + 4) as *const u32)
    {
        return None;
    }
    Some((core::ptr::read_volatile((paths + file_path as usize * 0x20) as *const u64) >> 40) as u32)
}

unsafe fn file_residency(service: usize, out: &mut Residency) {
    out.count_a = core::ptr::read_volatile((service + 0x18) as *const u32);
    if out.file_path >= out.count_a {
        out.stop = "count_a";
        return;
    }
    let table_a = core::ptr::read_volatile((service + 0x08) as *const usize);
    let table_b = core::ptr::read_volatile((service + 0x10) as *const usize);
    if table_a < LOWEST_PLAUSIBLE_POINTER || table_b < LOWEST_PLAUSIBLE_POINTER {
        out.stop = "loaded_tables";
        return;
    }
    out.count_b = core::ptr::read_volatile((service + 0x1c) as *const u32);
    out.slot = table_a + out.file_path as usize * 8;
    out.flag = core::ptr::read_volatile((out.slot + 4) as *const u8);
    out.entry = core::ptr::read_volatile(out.slot as *const u32);
    if out.flag != 0 && resident_record(out, table_b, out.entry) {
        out.stop = "resident";
        return;
    }
    let Some(linked) = linked_index(service, out.file_path) else {
        out.stop = "file_table";
        return;
    };
    out.linked = linked;
    if resident_record(out, table_b, out.linked) {
        out.stop = "linkable";
    }
}

unsafe fn inspect(search_path: u32) -> Residency {
    let mut out = Residency::blank();
    if search_path == NOT_FOUND {
        return out;
    }
    let service = match service() {
        Ok(service) => service,
        Err(stop) => {
            out.stop = stop;
            return out;
        }
    };
    out.file_path = resolve_relative_path(search_path, MODEL_NUMDLB.as_ptr());
    if out.file_path != NOT_FOUND {
        file_residency(service, &mut out);
    }
    out
}

pub(crate) unsafe fn model_residency(search_path: u32) -> (bool, &'static str, u32) {
    let state = inspect(search_path);
    (state.loads(), state.stop, state.file_path)
}

pub(crate) unsafe fn search_path_loaded(search_path: u32) -> bool {
    if search_path == NOT_FOUND {
        return false;
    }
    let Ok(service) = service() else {
        return false;
    };
    let mut out = Residency::blank();
    out.file_path = file_path_of_search_path(search_path);
    if out.file_path == NOT_FOUND {
        return false;
    }
    file_residency(service, &mut out);
    out.loads()
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
    let result = call_original!(out, search_path, variant);
    let empty = !out.is_null()
        && core::ptr::read_volatile(out) == 0
        && core::ptr::read_volatile(out.add(1)) == 0;
    let rejected = empty
        && search != NOT_FOUND
        && resolve_relative_path(search, MODEL_NUMDLB.as_ptr()) != NOT_FOUND;
    let slip = if rejected {
        ticket(&EMPTY_LOG, EMPTY_REPORTS)
    } else if copy_kind.is_some() {
        ticket(&CONTEXT_LOG, CONTEXT_REPORTS)
    } else {
        None
    };
    if let Some(n) = slip {
        let state = inspect(search);
        let tag = if rejected { "EMPTY" } else { "context" };
        crate::dbg_log_public(&format!(
            "[copymodel] #{n} {tag} search={search:#x} file={:#x} stop={} copy_kind={copy_kind:?} variant={variant} count_a={} slot={:#x} flag={} entry={:#x} linked={:#x} count_b={} record={:#x} record_flag={} out0={:#x} thread={:#x}",
            state.file_path,
            state.stop,
            state.count_a,
            state.slot,
            state.flag,
            state.entry,
            state.linked,
            state.count_b,
            state.record,
            state.record_flag,
            if out.is_null() { 0 } else { core::ptr::read_volatile(out) },
            crate::current_thread_key()
        ));
    }
    result
}

pub(crate) fn install() {
    skyline::install_hooks!(model_build_probe);
    let site = crate::text_base() + CHECKS_FAILED;
    let wanted = clone_engine_core::hook_site::branch(CHECKS_FAILED, EMPTY_AND_RELEASE);
    let live = unsafe { core::ptr::read_volatile(site as *const u32) };
    let armed = live == wanted
        || (live == CHECKS_FAILED_WORD && unsafe { crate::text_patch::write_word(site, wanted) });
    if armed {
        crate::dbg_log_public(
            "[copymodel] 0x35c23a4 branches to 0x35c2464: a model the game rejects after its own request builds empty instead of reading address 0x20",
        );
    } else {
        crate::dbg_log_public(&format!(
            "[copymodel] 0x35c23a4 holds {live:#010x}, not mov x8, xzr; left alone, so a rejected model build still faults"
        ));
    }
}
