use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const LOADING_SHOW_CALL: usize = 0x22d_369c;
const LOADING_SHOW_CALL_WORD: u32 = 0x9452_885d;
const LOADING_SHOW_RETURN: usize = 0x22d_36a0;
const LINK_REGISTER: usize = 30;
const LAYOUT_SHOWN: usize = 0x18;
const PANE_FLAGS: usize = 0x58;
const PANE_VISIBLE: u8 = 1;
const TIP_PANES: [&str; 4] = ["tips", "matchup_tips_00", "matchup_tips_01", "gradation"];

pub(crate) const ARMED_OK: i32 = 0;
pub(crate) const UNAVAILABLE: i32 = 1;

static INSTALLED: AtomicBool = AtomicBool::new(false);
static ICON_ONLY: AtomicBool = AtomicBool::new(false);

struct Loading {
    layout: usize,
    panes: Vec<(usize, u8)>,
    hiding: bool,
    seen_shown: bool,
}

static LOADING: Mutex<Loading> = Mutex::new(Loading {
    layout: 0,
    panes: Vec::new(),
    hiding: false,
    seen_shown: false,
});

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value);
}

impl Loading {
    unsafe fn hide(&self) {
        for &(pane, _) in &self.panes {
            write(pane + PANE_FLAGS, read::<u8>(pane + PANE_FLAGS) & !PANE_VISIBLE);
        }
    }

    unsafe fn save(&mut self) {
        for (pane, saved) in self.panes.iter_mut() {
            *saved = read::<u8>(*pane + PANE_FLAGS) & PANE_VISIBLE;
        }
    }

    unsafe fn restore(&mut self) {
        for &(pane, saved) in &self.panes {
            write(pane + PANE_FLAGS, (read::<u8>(pane + PANE_FLAGS) & !PANE_VISIBLE) | saved);
        }
        self.hiding = false;
        self.seen_shown = false;
    }
}

pub(crate) fn icon_only() -> i32 {
    if !INSTALLED.load(Ordering::Acquire) {
        return UNAVAILABLE;
    }
    ICON_ONLY.store(true, Ordering::Release);
    skyline::println!("[menuloading] the next loading screen shows only its corner icon");
    ARMED_OK
}

pub(crate) unsafe fn place_layout(ctx: &skyline::hooks::InlineCtx) {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let caller = crate::game::offset_of(ctx.registers[LINK_REGISTER].x() as usize);
    if caller != LOADING_SHOW_RETURN {
        return;
    }
    let layout = ctx.registers[0].x() as usize;
    if layout == 0 {
        return;
    }
    let Ok(mut loading) = LOADING.lock() else {
        return;
    };
    if loading.layout != layout {
        if loading.hiding {
            loading.restore();
        }
        loading.layout = layout;
        loading.panes = TIP_PANES
            .iter()
            .filter_map(|name| crate::hud_layouts::find_pane(layout, name))
            .map(|(pane, _)| (pane, PANE_VISIBLE))
            .collect();
        skyline::println!(
            "[menuloading] loading layout {layout:#x}: {} of {} tip parts found",
            loading.panes.len(),
            TIP_PANES.len()
        );
    }
    if ICON_ONLY.swap(false, Ordering::AcqRel) {
        if !loading.hiding {
            loading.save();
        }
        loading.hiding = true;
        loading.seen_shown = false;
        loading.hide();
        skyline::println!("[menuloading] tips hidden for this load");
    } else if loading.hiding {
        loading.restore();
    }
}

pub(crate) unsafe fn follow() {
    let Ok(mut loading) = LOADING.try_lock() else {
        return;
    };
    if !loading.hiding || loading.layout == 0 {
        return;
    }
    if read::<u8>(loading.layout + LAYOUT_SHOWN) != 0 {
        loading.seen_shown = true;
        loading.hide();
    } else if loading.seen_shown {
        loading.restore();
        skyline::println!("[menuloading] the loading screen closed; tips restored");
    }
}

pub(crate) fn install() {
    let word = unsafe { crate::game::text_word(LOADING_SHOW_CALL) };
    if word != LOADING_SHOW_CALL_WORD {
        skyline::println!(
            "[menuloading] REFUSED loading show call {LOADING_SHOW_CALL:#x}: expected {LOADING_SHOW_CALL_WORD:#010x}, found {word:#010x}"
        );
        return;
    }
    INSTALLED.store(true, Ordering::Release);
}
