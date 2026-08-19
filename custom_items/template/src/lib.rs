use clone_engine_api::{
    allocate_item, compiled_capabilities, item_common_has, item_common_set, item_status_named,
    register_item_ui, runtime_capabilities, CloneItemKind, ItemCloneRegistration, ItemStatusLine,
    ItemUiRegistration, CAP_ITEM_ANIMCMD, CAP_ITEM_IDENTITY, CAP_ITEM_PARAMS, CAP_ITEM_RESOURCES,
    CAP_ITEM_STATUS, CAP_ITEM_TRAINING_UI,
};
use core::sync::atomic::{AtomicBool, Ordering};
use smash::lib::L2CValue;
use smash::lua2cpp::L2CFighterCommon;

const BASE_ITEM_KIND: i32 = 0x32;
const RESOURCE_NAME: &str = "template_item";
const UI_ID: &str = "ui_item_template_item";
const MAIN_REQUIRED: u64 = CAP_ITEM_IDENTITY
    | CAP_ITEM_RESOURCES
    | CAP_ITEM_PARAMS
    | CAP_ITEM_ANIMCMD
    | CAP_ITEM_TRAINING_UI;

static ITEM_KIND: CloneItemKind = CloneItemKind::new(RESOURCE_NAME);
static STATUS_ENTERED: AtomicBool = AtomicBool::new(false);

pub fn item_kind() -> Option<i32> {
    ITEM_KIND.get()
}

unsafe extern "C" fn throw_init(_agent: &mut L2CFighterCommon) -> L2CValue {
    if !STATUS_ENTERED.swap(true, Ordering::AcqRel) {
        skyline::println!("[template_item] clone-only THROW init entered");
    }
    L2CValue::new_int(0)
}

fn install() -> Result<(), String> {
    let compiled = compiled_capabilities();
    let runtime = runtime_capabilities();
    if compiled & (MAIN_REQUIRED | CAP_ITEM_STATUS) != MAIN_REQUIRED | CAP_ITEM_STATUS {
        return Err(format!(
            "missing compiled capabilities {:#x}",
            (MAIN_REQUIRED | CAP_ITEM_STATUS) & !compiled
        ));
    }
    if runtime & MAIN_REQUIRED != MAIN_REQUIRED {
        return Err(format!(
            "runtime preflight failed for {:#x}",
            MAIN_REQUIRED & !runtime
        ));
    }
    let kind = allocate_item(&ItemCloneRegistration::new(
        clone_engine_api::KIND_AUTO,
        BASE_ITEM_KIND,
        RESOURCE_NAME,
        RESOURCE_NAME,
    ))
    .map_err(|error| format!("item registration: {error:?}"))?;
    ITEM_KIND.store(kind);
    skyline::println!("[template_item] allocated item kind {kind:#x}");
    register_item_ui(&ItemUiRegistration::training(kind, UI_ID))
        .map_err(|error| format!("Training UI registration: {error:?}"))?;
    let throw_speed = smash::hash40("throw_speed_mul");
    if !item_common_has(throw_speed) {
        return Err("throw_speed_mul is absent from the measured common-param map".into());
    }
    item_common_set(kind, throw_speed, 0.75)
        .map_err(|error| format!("common parameter: {error:?}"))?;
    item_status_named(kind, ItemStatusLine::Init, "THROW", throw_init as *const ())
        .map_err(|error| format!("status callback: {error:?}"))?;
    Ok(())
}

#[skyline::main(name = "clone_engine_item_template")]
pub fn main() {
    if let Err(error) = install() {
        skyline::println!("[template_item] disabled: {error}");
    }
}
