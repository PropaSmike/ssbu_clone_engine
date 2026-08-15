#[cfg(test)]
#[path = "stage_ledger.rs"]
mod stage_ledger;
#[cfg(not(test))]
use crate::stage_ledger::{hash40, CloneStage, StageResources};
#[cfg(test)]
use stage_ledger::{hash40, CloneStage, StageResources};

#[derive(Debug, PartialEq, Eq)]
pub enum RegistrationError {
    DispOrderTooLarge(i32),
    NotSelectableAndNotHidden,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDbRequest {
    pub ui_stage_id: u64,
    pub name_id: String,
    pub stage_place_id: u64,
    pub secret_stage_place_id: u64,
    pub ui_series_id: u64,
    pub disp_order: i8,
    pub can_select: bool,
    pub is_dlc: bool,
    pub save_no: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub db: StageDbRequest,
    pub place_hash: u64,
    pub stage_hash: u64,
    pub normal: StageResources,
    pub end: StageResources,
    pub battle: StageResources,
    pub deferred_disp_order: Option<u16>,
}

pub fn plan(
    stage: &CloneStage,
    ui_series_id: u64,
    disp_order: i32,
) -> Result<Registration, RegistrationError> {
    if disp_order > u8::MAX as i32 {
        return Err(RegistrationError::DispOrderTooLarge(disp_order));
    }
    if disp_order < -1 {
        return Err(RegistrationError::NotSelectableAndNotHidden);
    }
    let deferred = (disp_order > i8::MAX as i32).then_some(disp_order as u16);
    let registered_order = if deferred.is_some() { -1 } else { disp_order };

    let place_hash = hash40(&stage.place_name);
    let stage_hash = hash40(&format!("ui_stage_{}", stage.place_name));
    let resources = stage.resource_set();

    Ok(Registration {
        db: StageDbRequest {
            ui_stage_id: stage_hash,
            name_id: stage.name_id().to_string(),
            stage_place_id: place_hash,
            secret_stage_place_id: place_hash,
            ui_series_id,
            disp_order: registered_order as i8,
            can_select: disp_order >= 0,
            is_dlc: stage.distribution == 1,
            save_no: -1,
        },
        place_hash,
        stage_hash,
        normal: resources.normal,
        end: resources.end,
        battle: resources.battle,
        deferred_disp_order: deferred,
    })
}

#[cfg(feature = "stage_slot")]
pub fn register(registration: &Registration) {
    use the_csk_collection_api::{
        BoolType, CStrCSK, Hash40Map, Hash40Type, ShortType, SignedByteType, StageDatabaseEntry,
        StringType, UiStageData, UiStageResources,
    };

    fn resources(source: &StageResources) -> UiStageResources {
        UiStageResources {
            stage_load_group_hash: source.stage_load_group_hash,
            effect_load_group_hash: source.effect_load_group_hash,
            nus3bank_path_hash: source.nus3bank_path_hash,
            sqb_path_hash: source.sqb_path_hash,
            nus3audio_path_hash: source.nus3audio_path_hash,
            tonelabel_path_hash: source.tonelabel_path_hash,
        }
    }

    let entry = StageDatabaseEntry {
        ui_stage_id: registration.db.ui_stage_id,
        clone_from_ui_stage_id: None,
        name_id: StringType::Overwrite(CStrCSK::new(&registration.db.name_id)),
        save_no: ShortType::Overwrite(registration.db.save_no),
        ui_series_id: Hash40Type::Overwrite(registration.db.ui_series_id),
        can_select: BoolType::Overwrite(registration.db.can_select),
        disp_order: SignedByteType::Overwrite(registration.db.disp_order),
        stage_place_id: Hash40Type::Overwrite(registration.db.stage_place_id),
        secret_stage_place_id: Hash40Type::Overwrite(registration.db.secret_stage_place_id),
        can_demo: BoolType::Overwrite(false),
        is_8player_stage: BoolType::Overwrite(false),
        is_usable_flag: BoolType::Overwrite(true),
        is_usable_amiibo: BoolType::Overwrite(true),
        secret_command_id: Hash40Type::Overwrite(0),
        secret_command_id_joycon: Hash40Type::Overwrite(0),
        bgm_set_id: Hash40Type::Overwrite(0),
        bgm_setting_no: Default::default(),
        bgm_selector: BoolType::Overwrite(false),
        is_dlc: BoolType::Overwrite(registration.db.is_dlc),
        is_patch: BoolType::Overwrite(false),
        dlc_chara_id: Hash40Type::Overwrite(0),
        extra_hash_maps: Hash40Map::default(),
    };

    let data = UiStageData {
        normal: resources(&registration.normal),
        end: resources(&registration.end),
        battle: resources(&registration.battle),
    };

    the_csk_collection_api::add_ui_stage_db_resources_entry(
        registration.place_hash,
        registration.stage_hash,
        &data,
    );
    the_csk_collection_api::add_stage_db_entry(&entry);

    skyline::println!(
        "[stagereg] registered {:#x} (place {:#x}) with CSK, disp_order {}",
        registration.stage_hash,
        registration.place_hash,
        registration.db.disp_order
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use stage_ledger::Form;

    fn declared(name: &str, battle_tree: bool) -> CloneStage {
        let mut stage = CloneStage::new(name);
        stage.ships_battle_tree = battle_tree;
        stage.forms = vec![Form::Normal, Form::Omega, Form::Battlefield];
        stage
    }

    #[test]
    fn carries_the_hashes_the_image_proved() {
        let stage = declared("photostage", false);
        let registration = plan(&stage, 0, 0).unwrap();
        assert_eq!(
            registration.normal.effect_load_group_hash,
            hash40("effect/stage/photostage")
        );
        assert_eq!(registration.place_hash, hash40("photostage"));
        assert_eq!(registration.stage_hash, hash40("ui_stage_photostage"));
    }

    #[test]
    fn a_value_past_csk_registers_hidden_and_defers() {
        let stage = declared("pumpkin_hill", false);
        let deferred = plan(&stage, 0, 200).unwrap();
        assert_eq!(deferred.db.disp_order, -1);
        assert_eq!(deferred.deferred_disp_order, Some(200));
        assert!(deferred.db.can_select);
    }

    #[test]
    fn a_value_csk_can_carry_is_not_deferred() {
        let stage = declared("pumpkin_hill", false);
        let direct = plan(&stage, 0, 127).unwrap();
        assert_eq!(direct.db.disp_order, 127);
        assert_eq!(direct.deferred_disp_order, None);
    }

    #[test]
    fn refuses_past_what_any_backend_can_express() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(
            plan(&stage, 0, 256),
            Err(RegistrationError::DispOrderTooLarge(256))
        );
    }

    #[test]
    fn minus_one_hides_the_stage_and_clears_can_select() {
        let stage = declared("pumpkin_hill", false);
        let hidden = plan(&stage, 0, -1).unwrap();
        assert_eq!(hidden.db.disp_order, -1);
        assert!(!hidden.db.can_select);
        let shown = plan(&stage, 0, 0).unwrap();
        assert!(shown.db.can_select);
    }

    #[test]
    fn refuses_a_negative_that_is_not_the_hidden_sentinel() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(
            plan(&stage, 0, -7),
            Err(RegistrationError::NotSelectableAndNotHidden)
        );
    }

    #[test]
    fn a_minted_stage_never_claims_a_save_slot() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(plan(&stage, 0, 0).unwrap().db.save_no, -1);
    }

    #[test]
    fn the_label_is_the_enum_name_not_the_place_name() {
        let mut stage = declared("template_stage", false);
        stage.ui_name_id = Some("TemplateStage".into());
        assert_eq!(plan(&stage, 0, 118).unwrap().db.name_id, "TemplateStage");
    }

    #[test]
    fn an_unset_label_still_falls_back_to_the_place_name() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(plan(&stage, 0, 0).unwrap().db.name_id, "pumpkin_hill");
    }

    #[test]
    fn the_battle_tree_reaches_the_registration() {
        let stage = declared("pumpkin_hill", true);
        let registration = plan(&stage, 0, 0).unwrap();
        assert_eq!(
            registration.end.stage_load_group_hash,
            hash40("stage/pumpkin_hill/battle")
        );
        assert_ne!(
            registration.normal.stage_load_group_hash,
            registration.end.stage_load_group_hash
        );
    }
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_stage(
    place_name: *const core::ffi::c_char,
    name_id: *const core::ffi::c_char,
    ships_battle_tree: bool,
    ui_series_id: u64,
    disp_order: i32,
) -> i32 {
    if place_name.is_null() {
        return -1;
    }
    let name = match core::ffi::CStr::from_ptr(place_name).to_str() {
        Ok(name) if !name.is_empty() => name,
        _ => return -2,
    };

    let mut stage = CloneStage::new(name);
    stage.ships_battle_tree = ships_battle_tree;
    if !name_id.is_null() {
        match core::ffi::CStr::from_ptr(name_id).to_str() {
            Ok(label) if !label.is_empty() => stage.ui_name_id = Some(label.to_string()),
            _ => return -2,
        }
    }

    if let Ok(registry) = crate::stage_registry::registry().lock() {
        if let Some(minted) = registry.by_name(&stage.place_name) {
            if minted.asset_place != stage.place_name {
                stage.resource_place = Some(minted.asset_place.clone());
                skyline::println!(
                    "[stagereg] {name} registers with assets from {}, as minted",
                    minted.asset_place
                );
            }
        }
    }

    let claimed = crate::stage_registry::registry()
        .lock()
        .map(|mut registry| registry.claim_row(&stage.place_name))
        .unwrap_or(false);
    if !claimed {
        skyline::println!("[stagereg] {name} already has a grid row; not registering a second");
        return 0;
    }

    match plan(&stage, ui_series_id, disp_order) {
        Ok(_registration) => {
            skyline::println!(
                "[stagereg] {name} label will be nam_stg1_{}",
                stage.name_id()
            );
            #[cfg(feature = "stage_slot")]
            {
                register(&_registration);
                if let Some(wanted) = _registration.deferred_disp_order {
                    crate::stage_db_rows::request_disp_order(_registration.stage_hash, wanted);
                    skyline::println!(
                        "[stagereg] {name} registered hidden; disp_order {wanted} queued for the                          row backend (CSK's SignedByteType stops at 127)"
                    );
                }
                0
            }
            #[cfg(not(feature = "stage_slot"))]
            {
                skyline::println!(
                    "[stagereg] refused {name}: this engine build has no stage backend \
                     (rebuild with --features stage_slot)"
                );
                -3
            }
        }
        Err(RegistrationError::DispOrderTooLarge(value)) => {
            skyline::println!(
                "[stagereg] refused {name}: disp_order {value} exceeds 127, which is all \
                 CSK's SignedByteType can carry. Author the row into ui_stage_db.prc with \
                 tools/stage_disp_order.py instead."
            );
            -4
        }
        Err(RegistrationError::NotSelectableAndNotHidden) => {
            skyline::println!(
                "[stagereg] refused {name}: disp_order {disp_order} is negative but not the \
                 -1 hidden sentinel"
            );
            -5
        }
    }
}
