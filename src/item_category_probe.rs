use core::sync::atomic::{AtomicU32, Ordering};

const OFF_CREATE_BOSS: usize = 0x15C8D80;
const OFF_CREATE_WEAPON: usize = 0x15C8ED0;
const OFF_CREATE_WEAPON_WITH_VARIATION: usize = 0x15C8EE0;

const CREATE_BOSS_WORDS: [u32; 4] = [0xD10283FF, 0x6D0523E9, 0xF90033F7, 0xA90757F6];
const CREATE_WEAPON_WORDS: [u32; 2] = [0x12800002, 0x14000003];
const CREATE_WEAPON_WITH_VARIATION_WORDS: [u32; 4] =
    [0xD10343FF, 0x6D0823E9, 0xF9004BF7, 0xA90A57F6];

const MAX_LOGS_PER_SITE: u32 = 128;
const KOOPAG_KIND: i32 = 398;

static CREATE_BOSS_LOGS: AtomicU32 = AtomicU32::new(0);
static CREATE_WEAPON_LOGS: AtomicU32 = AtomicU32::new(0);
static CREATE_WEAPON_VARIATION_LOGS: AtomicU32 = AtomicU32::new(0);

#[inline(always)]
unsafe fn caller_lr() -> usize {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr, options(nomem, nostack, preserves_flags));
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    lr
}

#[inline]
unsafe fn text_base() -> usize {
    skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize
}

#[inline]
fn log(message: String) {
    crate::dbg_log_public(&message);
}

#[inline]
fn category_note(kind: i32) -> &'static str {
    match kind {
        KOOPAG_KIND => "koopag-external-lua2cpp_koopag",
        0xA6..=0x118 => "assist",
        0x119..=0x15F => "pokemon",
        0x160..=0x1A8 => "boss",
        _ => "non-special-or-child-boundary",
    }
}

#[inline]
fn next(counter: &AtomicU32) -> Option<u32> {
    let sequence = counter.fetch_add(1, Ordering::Relaxed);
    (sequence < MAX_LOGS_PER_SITE).then_some(sequence)
}

unsafe fn words_match(
    text: usize,
    offset: usize,
    expected: &[u32],
) -> Result<(), (usize, u32, u32)> {
    for (index, wanted) in expected.iter().copied().enumerate() {
        let address = text + offset + index * 4;
        let found = (address as *const u32).read();
        if found != wanted {
            return Err((address, found, wanted));
        }
    }
    Ok(())
}

#[skyline::hook(offset = OFF_CREATE_BOSS)]
unsafe fn create_boss_probe(state: *mut libc::c_void, kind: i32, argument: i32) -> u32 {
    let lr = caller_lr();
    let caller = lr.wrapping_sub(text_base());
    let sequence = next(&CREATE_BOSS_LOGS);
    if let Some(sequence) = sequence {
        log(format!(
            "[itemcat] boss-create ENTER #{sequence} state={state:p} kind={kind} category={} argument={argument} caller=@{caller:#x} lr_raw={lr:#x}",
            category_note(kind)
        ));
        if kind == KOOPAG_KIND {
            log(format!(
                "[itemcat] boss-create #{sequence} Koopag 398 is externally owned by lua2cpp_koopag; no lua2cpp_item status assumption is valid"
            ));
        }
    }

    let result = call_original!(state, kind, argument);

    if let Some(sequence) = sequence {
        log(format!(
            "[itemcat] boss-create EXIT #{sequence} kind={kind} object_id={result:#x}"
        ));
    }
    result
}

#[skyline::hook(offset = OFF_CREATE_WEAPON)]
unsafe fn create_weapon_probe(
    state: *mut libc::c_void,
    kind: i32,
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    lr_value: f32,
) -> u32 {
    let caller = caller_lr();
    let caller_offset = caller.wrapping_sub(text_base());
    let sequence = next(&CREATE_WEAPON_LOGS);
    if let Some(sequence) = sequence {
        log(format!(
            "[itemcat] boss-weapon ENTER #{sequence} state={state:p} kind={kind} category={} pos=({pos_x:.3},{pos_y:.3},{pos_z:.3}) lr={lr_value:.3} caller=@{caller_offset:#x} lr_raw={caller:#x}",
            category_note(kind)
        ));
    }

    let result = call_original!(state, kind, pos_x, pos_y, pos_z, lr_value);

    if let Some(sequence) = sequence {
        log(format!(
            "[itemcat] boss-weapon EXIT #{sequence} kind={kind} object_id={result:#x}"
        ));
    }
    result
}

#[skyline::hook(offset = OFF_CREATE_WEAPON_WITH_VARIATION)]
unsafe fn create_weapon_with_variation_probe(
    state: *mut libc::c_void,
    kind: i32,
    variation: i32,
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    lr_value: f32,
) -> u32 {
    let caller = caller_lr();
    let caller_offset = caller.wrapping_sub(text_base());
    let sequence = next(&CREATE_WEAPON_VARIATION_LOGS);
    if let Some(sequence) = sequence {
        log(format!(
            "[itemcat] boss-weapon-var ENTER #{sequence} state={state:p} kind={kind} category={} variation={variation} pos=({pos_x:.3},{pos_y:.3},{pos_z:.3}) lr={lr_value:.3} caller=@{caller_offset:#x} lr_raw={caller:#x}",
            category_note(kind)
        ));
    }

    let result = call_original!(state, kind, variation, pos_x, pos_y, pos_z, lr_value);

    if let Some(sequence) = sequence {
        log(format!(
            "[itemcat] boss-weapon-var EXIT #{sequence} kind={kind} variation={variation} object_id={result:#x}"
        ));
    }
    result
}

pub(crate) fn install() {
    unsafe {
        let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize;
        let sites: &[(usize, &[u32], &str)] = &[
            (OFF_CREATE_BOSS, &CREATE_BOSS_WORDS, "create_boss"),
            (OFF_CREATE_WEAPON, &CREATE_WEAPON_WORDS, "create_weapon"),
            (
                OFF_CREATE_WEAPON_WITH_VARIATION,
                &CREATE_WEAPON_WITH_VARIATION_WORDS,
                "create_weapon_with_variation",
            ),
        ];

        for (offset, words, name) in sites {
            if let Err((address, found, wanted)) = words_match(text, *offset, words) {
                skyline::println!(
                    "[itemcat] REFUSED all probes: {name} fingerprint mismatch at {address:#x}: found {found:#010x}, expected {wanted:#010x}"
                );
                return;
            }
        }

        skyline::install_hooks!(
            create_boss_probe,
            create_weapon_probe,
            create_weapon_with_variation_probe
        );
        skyline::println!(
            "[itemcat] installed 3 observation-only boss-family probes (SSBU 13.0.4 exact fingerprints; cap {MAX_LOGS_PER_SITE}/site)"
        );
        skyline::println!(
            "[itemcat] Koopag kind 398 remains external-module-owned (lua2cpp_koopag) and is only labeled, never routed"
        );
    }
}
