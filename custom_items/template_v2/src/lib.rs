use clone_engine_api::v2::{hook, item, slot, ItemManifest};
use clone_engine_api::ItemStatusLine;
use core::sync::atomic::{AtomicBool, Ordering};
use smash::lib::L2CValue;
use smash::lua2cpp::L2CFighterCommon;

// TEMPLATE_ITEM: the handle the crate uses.
// "template_item": the resource name, the same string as ItemManifest::new's.
item!(TEMPLATE_ITEM, "template_item");

static ENTERED: AtomicBool = AtomicBool::new(false);

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

#[skyline::main(name = "clone_engine_item_template_v2")]
pub fn main() {
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
    .owner_param("pickel", "life", 600.0)  // owner fighter (Steve), a field of its vl.prc, value
    .owner_param("pickel", "auto_damage", 0.0); // set every field that decides the same outcome
    let kind = match TEMPLATE_ITEM.register(manifest) {
        Ok(kind) => kind,
        Err(error) => {
            skyline::println!("[template_item_v2] registration refused: {error:?}");
            return;
        }
    };

    // status line, a status name of the base item, function; at startup, by name
    TEMPLATE_ITEM.status(ItemStatusLine::Init, "THROW", throw_init);
    clone_engine_api::install_hooks!(update_9); // every #[hook] in the crate, after registration

    skyline::println!("[template_item_v2] installed; kind {kind:#x}");
}
