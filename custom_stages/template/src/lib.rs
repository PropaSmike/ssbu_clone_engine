use clone_engine_api::{
    allocate_stage, compiled_capabilities, register_stage, runtime_capabilities,
    set_stage_behaviour, stage_capacity, stage_id_for, StageAllocation, StageForm,
    StageRegistration, CAP_STAGE_CONFIG, CAP_STAGE_CSK, CAP_STAGE_MINT, CAP_STAGE_SELECT_EXTENDED,
    STAGE_FORM_BATTLEFIELD, STAGE_FORM_NORMAL, STAGE_FORM_OMEGA,
};
const PLACE: &str = "template_stage";
const RESOURCE_PLACE: Option<&str> = Some("dk_jungle");
const BEHAVIOUR_PLACE: Option<&str> = Some("dk_jungle");
const SHIPS_BATTLE_TREE: bool = false;
const ID_NAME: &str = "Template_Stage";
const FORMS: u32 = STAGE_FORM_NORMAL | STAGE_FORM_OMEGA | STAGE_FORM_BATTLEFIELD;
const DISP_ORDER: i32 = 118;
const REQUIRED: u64 = CAP_STAGE_MINT | CAP_STAGE_CONFIG | CAP_STAGE_SELECT_EXTENDED | CAP_STAGE_CSK;

fn install() -> Result<(), String> {
    let compiled = compiled_capabilities();
    let runtime = runtime_capabilities();
    if compiled & REQUIRED != REQUIRED {
        return Err(format!(
            "missing compiled capabilities {:#x}",
            REQUIRED & !compiled
        ));
    }
    if runtime & REQUIRED != REQUIRED {
        return Err(format!(
            "runtime preflight failed for {:#x}",
            REQUIRED & !runtime
        ));
    }
    let capacity = stage_capacity().map_err(|error| format!("capacity: {error:?}"))?;
    if !capacity.can_mint || capacity.places == 0 || capacity.stage_ids < FORMS.count_ones() {
        return Err(format!("no capacity: {capacity:?}"));
    }
    let place_index = allocate_stage(&StageAllocation {
        place_name: PLACE,
        resource_place: RESOURCE_PLACE,
        ships_battle_tree: SHIPS_BATTLE_TREE,
        forms: FORMS,
    })
    .map_err(|error| format!("allocate: {error:?}"))?;
    if let Some(donor) = BEHAVIOUR_PLACE {
        set_stage_behaviour(PLACE, donor).map_err(|error| format!("behaviour: {error:?}"))?;
    }
    let ids = [
        stage_id_for(PLACE, StageForm::Normal),
        stage_id_for(PLACE, StageForm::Omega),
        stage_id_for(PLACE, StageForm::Battlefield),
    ];
    register_stage(&StageRegistration {
        place_name: PLACE,
        name_id: ID_NAME,
        ships_battle_tree: SHIPS_BATTLE_TREE,
        ui_series_id: smash::hash40("smash"),
        display_order: DISP_ORDER,
    })
    .map_err(|error| format!("register: {error:?}"))?;
    println!("[{PLACE}] place={place_index} ids={ids:?}");
    Ok(())
}

#[skyline::main(name = "clone_engine_stage_template")]
pub fn main() {
    if let Err(error) = install() {
        println!("[{PLACE}] disabled: {error}");
    }
}
