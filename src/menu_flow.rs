use clone_engine_core::menu::{self, FlowHost};
use core::sync::atomic::{AtomicU32, Ordering};

const OFF_SMASH_UPDATE: usize = 0x2337060;
const SMASH_UPDATE_OPCODE: u32 = 0xa9be4ff4;
const SCENE_STEPS: usize = 0xe8;
const LOG_BUDGET: u32 = 64;

static LOGS: AtomicU32 = AtomicU32::new(0);

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn steer(scene: usize, host: &FlowHost) {
    let flow = crate::menu_sequences::active_flow();
    if flow.is_empty() || scene == 0 {
        return;
    }
    let steps: usize = read(scene + SCENE_STEPS);
    if steps == 0 {
        return;
    }
    let layout = menu::step_layout(host.steps);
    let selector: i32 = read(steps + layout.selector);
    if !(0..menu::STEP_SELECTORS).contains(&selector) {
        return;
    }
    let slot = steps + layout.next + selector as usize * 4;
    let next: i32 = read(slot);
    let current: i32 = read(steps + layout.current);
    if next == host.battle && current != host.battle {
        crate::menu_battle::reapply(flow.skip_rules, flow.skip_stage_select);
    }
    let Some(target) = menu::redirect(host, flow, current, next) else {
        return;
    };
    core::ptr::write_volatile(slot as *mut i32, target);
    if LOGS.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        skyline::println!("[menuflow] step {current} -> {next} becomes {current} -> {target}");
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_SMASH_UPDATE))]
unsafe fn smash_update(scene: usize, frame: usize) -> u64 {
    steer(scene, &menu::SMASH_FLOW);
    call_original!(scene, frame)
}

pub(crate) fn install() {
    let observed = unsafe { crate::game::text_word(OFF_SMASH_UPDATE) };
    if observed != SMASH_UPDATE_OPCODE {
        skyline::println!(
            "[menuflow] REFUSED Smash step update {OFF_SMASH_UPDATE:#x}: expected {SMASH_UPDATE_OPCODE:#010x}, found {observed:#010x}; mode flows stay off"
        );
        return;
    }
    skyline::install_hook!(smash_update);
}
