pub const OP_SET: u32 = 0;
pub const OP_MUL: u32 = 1;
pub const ANY_SLOT: i32 = -1;

fn valid_request(_kind: i32, slots: &[i32], key: (u64, u64)) -> bool {
    !slots.is_empty() && slots.iter().all(|slot| *slot >= ANY_SLOT) && key.0 != 0
}

pub fn available() -> bool {
    unsafe {
        crate::lookup_symbol(b"update_float\0").is_some()
            && crate::lookup_symbol(b"update_attribute_mul\0").is_some()
            && crate::lookup_symbol(b"update_int\0").is_some()
    }
}

pub unsafe fn push_to_param_config(
    kind: i32,
    slots: &[i32],
    key: (u64, u64),
    op: u32,
    value: f64,
) -> bool {
    if !valid_request(kind, slots, key) || !value.is_finite() {
        return false;
    }

    type UpdateF32 = unsafe extern "C" fn(i32, Vec<i32>, (u64, u64), f32);

    let symbol: &[u8] = match op {
        OP_SET => b"update_float\0",
        OP_MUL => b"update_attribute_mul\0",
        _ => return false,
    };
    let Some(address) = crate::lookup_symbol(symbol) else {
        return false;
    };
    let update: UpdateF32 = core::mem::transmute(address);
    update(kind, slots.to_vec(), key, value as f32);
    true
}

pub unsafe fn push_int_to_param_config(
    kind: i32,
    slots: &[i32],
    key: (u64, u64),
    value: i32,
) -> bool {
    if !valid_request(kind, slots, key) {
        return false;
    }

    type UpdateInt = unsafe extern "C" fn(i32, Vec<i32>, (u64, u64), i32);

    let Some(address) = crate::lookup_symbol(b"update_int\0") else {
        return false;
    };
    let update: UpdateInt = core::mem::transmute(address);
    update(kind, slots.to_vec(), key, value);
    true
}
