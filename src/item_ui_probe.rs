use core::sync::atomic::{AtomicU32, Ordering};

const OFF_TRAINING_POKEMON_LIST_DONE: usize = 0x1BB8374;
const OFF_TRAINING_ASSIST_LIST_DONE: usize = 0x1BB83E0;
const OFF_TRAINING_MASTER_LIST_DONE: usize = 0x1BB844C;
const OFF_TRAINING_SELECTION_TOKEN: usize = 0x1BC2304;
const OFF_TRAINING_QUEUE_REQUEST: usize = 0x1559EE8;
const OFF_RULES_LIST_COUNT: usize = 0x1C18E38;
const OFF_RULES_ROW_TOKEN: usize = 0x1C18ED0;

const SITES: &[(usize, u32, &str)] = &[
    (
        OFF_TRAINING_POKEMON_LIST_DONE,
        0xF94073E0,
        "training-pokemon-list",
    ),
    (
        OFF_TRAINING_ASSIST_LIST_DONE,
        0xF9405BE0,
        "training-assist-list",
    ),
    (
        OFF_TRAINING_MASTER_LIST_DONE,
        0xF94043E0,
        "training-master-list",
    ),
    (
        OFF_TRAINING_SELECTION_TOKEN,
        0x17FFFEF9,
        "training-selection",
    ),
    (
        OFF_TRAINING_QUEUE_REQUEST,
        0x9402C732,
        "training-queue-request",
    ),
    (OFF_RULES_LIST_COUNT, 0xF9400409, "rules-count"),
    (OFF_RULES_ROW_TOKEN, 0x2A1A03E2, "rules-row"),
];

const TRAINING_POKEMON_VECTOR: usize = 0xA70;
const TRAINING_ASSIST_VECTOR: usize = 0xA88;
const TRAINING_MASTER_VECTOR: usize = 0xAA0;
const MAX_VECTOR_ENTRIES: usize = 4096;
const MAX_SELECTION_LOGS: u32 = 128;
const MAX_RULE_ROW_LOGS: u32 = 256;

static TRAINING_SELECTION_LOGS: AtomicU32 = AtomicU32::new(0);
static TRAINING_REQUEST_LOGS: AtomicU32 = AtomicU32::new(0);
static RULE_ROW_LOGS: AtomicU32 = AtomicU32::new(0);

#[inline]
fn log(message: String) {
    crate::dbg_log_public(&message);
}

unsafe fn vector_summary(object: usize, offset: usize) -> Option<(usize, u64, u64)> {
    if object == 0 || object & 7 != 0 {
        return None;
    }
    let vector = (object + offset) as *const usize;
    let begin = vector.read();
    let end = vector.add(1).read();
    if end < begin || (end - begin) & 7 != 0 {
        return None;
    }
    let count = (end - begin) / 8;
    if count > MAX_VECTOR_ENTRIES {
        return None;
    }
    let first = if count == 0 {
        0
    } else {
        (begin as *const u64).read()
    };
    let last = if count == 0 {
        0
    } else {
        ((end - 8) as *const u64).read()
    };
    Some((count, first, last))
}

unsafe fn report_training_list(ctx: &skyline::hooks::InlineCtx, offset: usize, name: &str) {
    let object = ctx.registers[19].x() as usize;
    match vector_summary(object, offset) {
        Some((count, first, last)) => log(format!(
            "[itemui] {name} object={object:#x} count={count} first={first:#x} last={last:#x}"
        )),
        None => log(format!(
            "[itemui] {name} object={object:#x} invalid-vector=+{offset:#x}"
        )),
    }
}

#[skyline::hook(offset = OFF_TRAINING_POKEMON_LIST_DONE, inline)]
unsafe fn training_pokemon_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    report_training_list(ctx, TRAINING_POKEMON_VECTOR, "training-pokemon-list");
}

#[skyline::hook(offset = OFF_TRAINING_ASSIST_LIST_DONE, inline)]
unsafe fn training_assist_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    report_training_list(ctx, TRAINING_ASSIST_VECTOR, "training-assist-list");
}

#[skyline::hook(offset = OFF_TRAINING_MASTER_LIST_DONE, inline)]
unsafe fn training_master_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    report_training_list(ctx, TRAINING_MASTER_VECTOR, "training-master-list");
}

#[skyline::hook(offset = OFF_TRAINING_SELECTION_TOKEN, inline)]
unsafe fn training_selection_token(ctx: &mut skyline::hooks::InlineCtx) {
    let sequence = TRAINING_SELECTION_LOGS.fetch_add(1, Ordering::Relaxed);
    if sequence >= MAX_SELECTION_LOGS {
        return;
    }
    let object = ctx.registers[19].x() as usize;
    let token = ctx.registers[8].x();
    let index = ctx.registers[20].x() as u32;
    let (screen, submenu) = if object != 0 && object & 3 == 0 {
        (
            ((object + 0xB70) as *const u32).read(),
            ((object + 0xB74) as *const u32).read(),
        )
    } else {
        (u32::MAX, u32::MAX)
    };
    log(format!(
        "[itemui] training-selection #{sequence} object={object:#x} index={index} token={token:#x} screen={screen} submenu={submenu}"
    ));
}

#[skyline::hook(offset = OFF_TRAINING_QUEUE_REQUEST, inline)]
unsafe fn training_queue_request(ctx: &mut skyline::hooks::InlineCtx) {
    let sequence = TRAINING_REQUEST_LOGS.fetch_add(1, Ordering::Relaxed);
    if sequence >= MAX_SELECTION_LOGS {
        return;
    }
    let queue = ctx.registers[0].x();
    let lane = ctx.registers[1].x() as u32;
    let carrier_kind = ctx.registers[2].x() as u32 as i32;
    let actor_kind = ctx.registers[3].x() as u32 as i32;
    log(format!(
        "[itemui] training-request #{sequence} queue={queue:#x} lane={lane} carrier_kind={carrier_kind} actor_kind={actor_kind}"
    ));
}

#[skyline::hook(offset = OFF_RULES_LIST_COUNT, inline)]
unsafe fn rules_list_count(ctx: &mut skyline::hooks::InlineCtx) {
    let count = ctx.registers[9].x() as u32;
    let visible = ctx.registers[10].x() as u32;
    let object = ctx.registers[28].x();
    log(format!(
        "[itemui] rules-count object={object:#x} rows={count} visible={visible} native_cap=84"
    ));
}

#[skyline::hook(offset = OFF_RULES_ROW_TOKEN, inline)]
unsafe fn rules_row_token(ctx: &mut skyline::hooks::InlineCtx) {
    let sequence = RULE_ROW_LOGS.fetch_add(1, Ordering::Relaxed);
    if sequence >= MAX_RULE_ROW_LOGS {
        return;
    }
    let row = ctx.registers[22].x() as u32;
    let token = ctx.registers[1].x();
    log(format!(
        "[itemui] rules-row #{sequence} row={row} token={token:#x}"
    ));
}

unsafe fn preflight() -> Result<(), (usize, u32, u32, &'static str)> {
    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize;
    for &(offset, expected, name) in SITES {
        let found = ((text + offset) as *const u32).read();
        if found != expected {
            return Err((offset, found, expected, name));
        }
    }
    Ok(())
}

pub(crate) fn install() {
    unsafe {
        if let Err((offset, found, expected, name)) = preflight() {
            skyline::println!(
                "[itemui] REFUSED all probes: {name} at {offset:#x} found={found:#010x} expected={expected:#010x}"
            );
            return;
        }
        skyline::install_hooks!(
            training_pokemon_list_done,
            training_assist_list_done,
            training_master_list_done,
            training_selection_token,
            training_queue_request,
            rules_list_count,
            rules_row_token
        );
        skyline::println!(
            "[itemui] installed 7 observation-only Training/Rules probes (SSBU 13.0.4 exact opcodes)"
        );
    }
}
