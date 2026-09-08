use clone_engine_api::{
    allocate_item, compiled_capabilities, item_owner_param_set_f32, item_owner_param_set_i32,
    register_item_ui, runtime_capabilities, CloneItemKind, ItemCloneRegistration,
    ItemUiRegistration, CAP_ITEM_IDENTITY, CAP_ITEM_PARAMS, CAP_ITEM_RESOURCES,
    CAP_ITEM_TRAINING_UI,
};

const BASE_ITEM_KIND: i32 = 0x1ae;
const OWNER_FIGHTER_KIND: i32 = 0x58;
const RESOURCE_NAME: &str = "template_block";
const UI_ID: &str = "ui_item_template_block";

const PICKELOBJECT_LIFE: u32 = 0x518;
const PICKELOBJECT_AUTO_DAMAGE: u32 = 0x520;

const BLOCK_LIFE_FRAMES: i32 = 600;
const BLOCK_AUTO_DAMAGE: f32 = 0.0;

const MAIN_REQUIRED: u64 =
    CAP_ITEM_IDENTITY | CAP_ITEM_RESOURCES | CAP_ITEM_PARAMS | CAP_ITEM_TRAINING_UI;

static ITEM_KIND: CloneItemKind = CloneItemKind::new(RESOURCE_NAME);

pub fn item_kind() -> Option<i32> {
    ITEM_KIND.get()
}

fn install() -> Result<(), String> {
    let compiled = compiled_capabilities();
    let runtime = runtime_capabilities();
    if compiled & MAIN_REQUIRED != MAIN_REQUIRED {
        return Err(format!(
            "missing compiled capabilities {:#x}",
            MAIN_REQUIRED & !compiled
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
    skyline::println!("[template_block] allocated item kind {kind:#x}");

    register_item_ui(&ItemUiRegistration::training(kind, UI_ID))
        .map_err(|error| format!("Training UI registration: {error:?}"))?;

    item_owner_param_set_i32(
        kind,
        OWNER_FIGHTER_KIND,
        PICKELOBJECT_LIFE,
        BLOCK_LIFE_FRAMES,
    )
    .map_err(|error| format!("life override: {error:?}"))?;
    item_owner_param_set_f32(
        kind,
        OWNER_FIGHTER_KIND,
        PICKELOBJECT_AUTO_DAMAGE,
        BLOCK_AUTO_DAMAGE,
    )
    .map_err(|error| format!("auto_damage override: {error:?}"))?;
    skyline::println!(
        "[template_block] lasts {BLOCK_LIFE_FRAMES} frames, auto_damage {BLOCK_AUTO_DAMAGE}, \
         while a real Steve keeps vanilla values"
    );

    Ok(())
}

#[skyline::main(name = "clone_engine_fighter_owned_item_template")]
pub fn main() {
    if let Err(error) = install() {
        skyline::println!("[template_block] disabled: {error}");
    }
}
