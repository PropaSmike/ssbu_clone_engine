use crate::shared_hook_core::{self, RawFn};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub use crate::shared_hook_core::HookCall;

pub const MAX_ADDRESSES: usize = 32;
pub const MAX_CALLBACKS: usize = 8;
pub const PROLOGUE_WORDS: usize = 4;
pub const MAIN_TEXT_END: usize = 0x39c_7e90;

pub const STATUS_INITIALIZED: u32 = 1 << 0;
pub const STATUS_SELF_TEST_OK: u32 = 1 << 1;
pub const STATUS_INSTALL_FAILED: u32 = 1 << 2;
pub const STATUS_READY: u32 = 1 << 31;

#[derive(Clone, Copy)]
pub struct HookSpec {
    pub offset: u64,
    pub expected_opcodes: [u32; PROLOGUE_WORDS],
    pub abi: u32,
    pub argument_count: u32,
}

pub fn legacy_spec(offset: u64) -> Option<HookSpec> {
    (offset == 0xaa_6990).then_some(HookSpec {
        offset,
        expected_opcodes: [0xd101_83ff, 0xa903_57f6, 0xa904_4ff4, 0xa905_7bfd],
        abi: shared_hook_core::ABI_GPR_X0,
        argument_count: 3,
    })
}

struct Slot {
    address: AtomicUsize,
    original: AtomicUsize,
    count: AtomicUsize,
    callbacks: [AtomicUsize; MAX_CALLBACKS],
    expected_opcodes: [AtomicU32; PROLOGUE_WORDS],
    abi: AtomicU32,
    argument_count: AtomicU32,
}

static SLOTS: [Slot; MAX_ADDRESSES] = [const {
    Slot {
        address: AtomicUsize::new(0),
        original: AtomicUsize::new(0),
        count: AtomicUsize::new(0),
        callbacks: [const { AtomicUsize::new(0) }; MAX_CALLBACKS],
        expected_opcodes: [const { AtomicU32::new(0) }; PROLOGUE_WORDS],
        abi: AtomicU32::new(0),
        argument_count: AtomicU32::new(0),
    }
}; MAX_ADDRESSES];

static REGISTRATION: std::sync::Mutex<()> = std::sync::Mutex::new(());
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static SELF_TEST_OK: AtomicBool = AtomicBool::new(false);
static INSTALL_FAILED: AtomicBool = AtomicBool::new(false);

fn find_slot(address: usize) -> Option<usize> {
    SLOTS
        .iter()
        .position(|slot| slot.address.load(Ordering::Acquire) == address)
}

fn slot_matches(slot: &Slot, spec: HookSpec) -> bool {
    slot.abi.load(Ordering::Acquire) == spec.abi
        && slot.argument_count.load(Ordering::Acquire) == spec.argument_count
        && slot
            .expected_opcodes
            .iter()
            .zip(spec.expected_opcodes)
            .all(|(stored, expected)| stored.load(Ordering::Acquire) == expected)
}

fn validated_address(text_base: usize, spec: HookSpec) -> Result<usize, Register> {
    if !shared_hook_core::valid_abi(spec.abi, spec.argument_count) {
        return Err(Register::InvalidAbi);
    }
    if !shared_hook_core::valid_spec(
        spec.offset,
        &spec.expected_opcodes,
        spec.abi,
        spec.argument_count,
        MAIN_TEXT_END,
    ) {
        return Err(Register::Invalid);
    }
    let offset = usize::try_from(spec.offset).map_err(|_| Register::Invalid)?;
    text_base.checked_add(offset).ok_or(Register::Invalid)
}

unsafe fn live_opcodes_match(address: usize, expected: &[u32; PROLOGUE_WORDS]) -> bool {
    expected.iter().enumerate().all(|(index, expected)| {
        core::ptr::read_volatile((address as *const u32).add(index)) == *expected
    })
}

unsafe fn dispatch(index: usize, args: [u64; 6]) -> u64 {
    let slot = &SLOTS[index];
    let count = slot.count.load(Ordering::Acquire).min(MAX_CALLBACKS);
    let mut callbacks = [0usize; MAX_CALLBACKS];
    for (output, stored) in callbacks.iter_mut().zip(slot.callbacks.iter()).take(count) {
        *output = stored.load(Ordering::Acquire);
    }
    shared_hook_core::run_chain(
        &callbacks[..count],
        slot.argument_count.load(Ordering::Acquire),
        args,
        slot.original.load(Ordering::Acquire),
    )
}

unsafe extern "C" fn stub<const N: usize>(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
) -> u64 {
    dispatch(N, [a0, a1, a2, a3, a4, a5])
}

static STUBS: [RawFn; MAX_ADDRESSES] = [
    stub::<0>, stub::<1>, stub::<2>, stub::<3>, stub::<4>, stub::<5>, stub::<6>, stub::<7>,
    stub::<8>, stub::<9>, stub::<10>, stub::<11>, stub::<12>, stub::<13>, stub::<14>, stub::<15>,
    stub::<16>, stub::<17>, stub::<18>, stub::<19>, stub::<20>, stub::<21>, stub::<22>, stub::<23>,
    stub::<24>, stub::<25>, stub::<26>, stub::<27>, stub::<28>, stub::<29>, stub::<30>, stub::<31>,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Register {
    Ok,
    Duplicate,
    Full,
    Unavailable,
    Invalid,
    InvalidAbi,
    Preflight,
    Conflict,
    InstallFailed,
}

pub fn initialize() -> Result<(), &'static str> {
    INITIALIZED.store(true, Ordering::Release);
    match self_test() {
        Ok(()) => {
            SELF_TEST_OK.store(true, Ordering::Release);
            Ok(())
        }
        Err(reason) => {
            SELF_TEST_OK.store(false, Ordering::Release);
            Err(reason)
        }
    }
}

pub fn status() -> u32 {
    let mut value = 0;
    if INITIALIZED.load(Ordering::Acquire) {
        value |= STATUS_INITIALIZED;
    }
    if SELF_TEST_OK.load(Ordering::Acquire) {
        value |= STATUS_SELF_TEST_OK;
    }
    if INSTALL_FAILED.load(Ordering::Acquire) {
        value |= STATUS_INSTALL_FAILED;
    }
    if value & (STATUS_INITIALIZED | STATUS_SELF_TEST_OK)
        == (STATUS_INITIALIZED | STATUS_SELF_TEST_OK)
        && value & STATUS_INSTALL_FAILED == 0
    {
        value |= STATUS_READY;
    }
    value
}

fn ready() -> bool {
    status() & STATUS_READY != 0
}

pub unsafe fn register(text_base: usize, spec: HookSpec, callback: usize) -> Register {
    if !ready() {
        return Register::Unavailable;
    }
    let address = match validated_address(text_base, spec) {
        Ok(address) => address,
        Err(error) => return error,
    };

    let _guard = match REGISTRATION.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let index = match find_slot(address) {
        Some(index) => {
            if !slot_matches(&SLOTS[index], spec) {
                return Register::Conflict;
            }
            index
        }
        None => {
            let Some(index) = SLOTS
                .iter()
                .position(|slot| slot.address.load(Ordering::Acquire) == 0)
            else {
                return Register::Full;
            };
            if !live_opcodes_match(address, &spec.expected_opcodes) {
                return Register::Preflight;
            }

            let mut original: *mut libc::c_void = core::ptr::null_mut();
            skyline::hooks::A64HookFunction(
                address as *const libc::c_void,
                STUBS[index] as *const () as *const libc::c_void,
                &mut original,
            );
            if original.is_null() {
                INSTALL_FAILED.store(true, Ordering::Release);
                return Register::InstallFailed;
            }

            let slot = &SLOTS[index];
            slot.original.store(original as usize, Ordering::Release);
            for (stored, expected) in slot.expected_opcodes.iter().zip(spec.expected_opcodes) {
                stored.store(expected, Ordering::Release);
            }
            slot.abi.store(spec.abi, Ordering::Release);
            slot.argument_count
                .store(spec.argument_count, Ordering::Release);
            slot.address.store(address, Ordering::Release);
            index
        }
    };

    let slot = &SLOTS[index];
    let count = slot.count.load(Ordering::Acquire);
    if slot
        .callbacks
        .iter()
        .take(count.min(MAX_CALLBACKS))
        .any(|entry| entry.load(Ordering::Acquire) == callback)
    {
        return Register::Duplicate;
    }
    if count >= MAX_CALLBACKS {
        return Register::Full;
    }

    slot.callbacks[count].store(callback, Ordering::Release);
    slot.count.store(count + 1, Ordering::Release);
    Register::Ok
}

fn self_test() -> Result<(), &'static str> {
    static SEEN: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn decline(call: *mut HookCall) -> u32 {
        SEEN.fetch_or(1, Ordering::Relaxed);
        (*call).args[0] = (*call).args[0].wrapping_add(1);
        0
    }
    unsafe extern "C" fn handle(call: *mut HookCall) -> u32 {
        SEEN.fetch_or(2, Ordering::Relaxed);
        (*call).result = (*call).args[0];
        1
    }
    unsafe extern "C" fn original(a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
        a0 ^ a1 ^ a2 ^ a3 ^ a4 ^ a5
    }

    SEEN.store(0, Ordering::Relaxed);
    let handled = unsafe {
        shared_hook_core::run_chain(
            &[decline as *const () as usize, handle as *const () as usize],
            1,
            [10, 99, 99, 99, 99, 99],
            original as *const () as usize,
        )
    };
    if handled != 11 || SEEN.load(Ordering::Relaxed) != 3 {
        return Err("callback order or handled result is wrong");
    }

    SEEN.store(0, Ordering::Relaxed);
    let fallback = unsafe {
        shared_hook_core::run_chain(
            &[decline as *const () as usize],
            1,
            [10, 99, 99, 99, 99, 99],
            original as *const () as usize,
        )
    };
    if fallback != 11 || SEEN.load(Ordering::Relaxed) != 1 {
        return Err("decline, argument rewrite, or original fallback is wrong");
    }
    Ok(())
}

pub unsafe fn original(text_base: usize, offset: u64, args: &[u64; 6]) -> u64 {
    let Ok(relative) = usize::try_from(offset) else {
        return 0;
    };
    if relative == 0 || relative >= MAIN_TEXT_END {
        return 0;
    }
    let Some(address) = text_base.checked_add(relative) else {
        return 0;
    };
    let Some(index) = find_slot(address) else {
        return 0;
    };
    let slot = &SLOTS[index];
    shared_hook_core::run_chain(
        &[],
        slot.argument_count.load(Ordering::Acquire),
        *args,
        slot.original.load(Ordering::Acquire),
    )
}
