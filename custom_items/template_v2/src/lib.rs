use clone_engine_api::v2::{hook, item, slot, ItemManifest};
use clone_engine_api::ItemStatusLine;
use core::sync::atomic::{AtomicBool, Ordering};
use smash::lib::L2CValue;
use smash::lua2cpp::L2CFighterCommon;

// TEMPLATE_ITEM: the handle the crate uses.
// "template_item": the resource name, the same string as ItemManifest::new's.
item!(TEMPLATE_ITEM, "template_item");

/// The engine features this pack uses. `runtime_capabilities()` is what the
/// installed engine has AND passed its checks for on this boot.
const NEEDED: u64 = clone_engine_api::CAP_ITEM_IDENTITY
    | clone_engine_api::CAP_ITEM_RESOURCES
    | clone_engine_api::CAP_ITEM_PARAMS
    | clone_engine_api::CAP_ITEM_ANIMCMD
    | clone_engine_api::CAP_ITEM_TRAINING_UI;
/// Ready only once lua2cpp_item.nro loads in a match, so at startup it can
/// only be checked as built in.
const NEEDED_LATER: u64 = clone_engine_api::CAP_ITEM_STATUS;

static ENTERED: AtomicBool = AtomicBool::new(false);
static COMMON_SEEN: AtomicBool = AtomicBool::new(false);

// A status callback: the line and status name are given at registration,
// the function gets the item's script agent.
unsafe extern "C" fn throw_init(_agent: &mut L2CFighterCommon) -> L2CValue {
    if !ENTERED.swap(true, Ordering::AcqRel) {
        skyline::println!("[template_item_v2] THROW init entered");
    }
    L2CValue::new_int(0)
}

// slot: an item vtable entry; item: the item! static. item: the item object
// (no class object first). UPDATE_9 is the last of the nine per-frame phases,
// so a value set after the original is the frame's last word.
#[hook(slot = slot::item::UPDATE_9, item = TEMPLATE_ITEM)]
unsafe fn update_9(item: *mut smash::app::BattleObject) {
    call_original!(item);
}

// common_slot: an entry of the agent vtable, written into a copy private to
// this item's agent. An item shares the first TEN slots with a fighter,
// set_status_scripts included; 10 to 14 are refused because on an item those
// indices are the item class's own virtuals, not the system lines the names
// describe. agent: the agent object, then the entry's own arguments.
#[hook(common_slot = slot::common::START_COROUTINE, item = TEMPLATE_ITEM)]
unsafe fn start_coroutine(agent: u64, index: i32, name: u64, state: *mut i32) -> u32 {
    if !COMMON_SEEN.swap(true, Ordering::AcqRel) {
        skyline::println!("[template_item_v2] the item's own common vtable entry ran");
    }
    call_original!(agent, index, name, state)
}

#[skyline::main(name = "clone_engine_item_template_v2")]
pub fn main() {
    let missing = (NEEDED & !clone_engine_api::runtime_capabilities())
        | (NEEDED_LATER & !clone_engine_api::compiled_capabilities());
    if missing != 0 {
        skyline::println!("[template_item_v2] disabled: the engine lacks capabilities {missing:#x}");
        return;
    }

    // The same declaration can be an item.toml beside config.json; the engine
    // then registers at boot and register() below just returns that kind.
    let manifest = ItemManifest::new(
        "template_item", // resource name: files under item/template_item, script name
        0x1ae,           // base kind: the vanilla item it is built on (0x1ae = Steve's block)
    )
    .base_item("pickelobject")             // the base's name, for the log
    .training_order(0)                     // position in the Training menu's item list
    .spawn_per(30)                         // natural drop weight; leave out for no natural drops
    .common("throw_speed_mul", 0.75)       // a float field of item/common/param/param.prc
    .owner_param_int("pickel", "life", 600)   // owner fighter (Steve), an integer field of its vl.prc, value
    .owner_param("pickel", "auto_damage", 0.0); // a float field; set every field that decides the same outcome
    let kind = match TEMPLATE_ITEM.register(manifest) {
        Ok(kind) => kind,
        Err(error) => {
            skyline::println!("[template_item_v2] registration refused: {error:?}");
            return;
        }
    };

    // status line, a status name of the base item, function; at startup, by name
    TEMPLATE_ITEM.status(ItemStatusLine::Init, "THROW", throw_init);
    // every #[hook] in the crate, after registration
    clone_engine_api::install_hooks!(update_9, start_coroutine);

    skyline::println!("[template_item_v2] installed; kind {kind:#x}");
}
