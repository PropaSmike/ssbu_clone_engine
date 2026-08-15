pub const ABI_GPR_X0: u32 = 1;
pub const MAX_GPR_ARGS: usize = 6;

#[repr(C)]
pub struct HookCall {
    pub args: [u64; MAX_GPR_ARGS],
    pub result: u64,
}

const _: [(); 56] = [(); core::mem::size_of::<HookCall>()];

pub type HookFn = unsafe extern "C" fn(*mut HookCall) -> u32;

pub type RawFn = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64;

pub fn valid_abi(abi: u32, argument_count: u32) -> bool {
    abi == ABI_GPR_X0 && argument_count <= MAX_GPR_ARGS as u32
}

pub fn valid_spec(
    offset: u64,
    expected_opcodes: &[u32; 4],
    abi: u32,
    argument_count: u32,
    text_end: usize,
) -> bool {
    if offset == 0
        || offset & 3 != 0
        || expected_opcodes.iter().any(|word| *word == 0)
        || !valid_abi(abi, argument_count)
    {
        return false;
    }
    let Ok(offset) = usize::try_from(offset) else {
        return false;
    };
    offset
        .checked_add(expected_opcodes.len() * core::mem::size_of::<u32>())
        .is_some_and(|last| last <= text_end)
}

fn clear_unused_args(args: &mut [u64; MAX_GPR_ARGS], argument_count: u32) {
    let used = (argument_count as usize).min(MAX_GPR_ARGS);
    for value in &mut args[used..] {
        *value = 0;
    }
}

pub unsafe fn run_chain(
    callbacks: &[usize],
    argument_count: u32,
    mut args: [u64; MAX_GPR_ARGS],
    original: usize,
) -> u64 {
    clear_unused_args(&mut args, argument_count);
    let mut call = HookCall { args, result: 0 };

    for &address in callbacks {
        if address == 0 {
            continue;
        }
        let callback: HookFn = core::mem::transmute(address);
        if callback(&mut call) != 0 {
            return call.result;
        }
    }

    if original == 0 {
        return 0;
    }
    clear_unused_args(&mut call.args, argument_count);
    let original: RawFn = core::mem::transmute(original);
    original(
        call.args[0],
        call.args[1],
        call.args[2],
        call.args[3],
        call.args[4],
        call.args[5],
    )
}
