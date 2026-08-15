#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
#[path = "stage_ledger.rs"]
mod stage_ledger;

#[cfg(not(test))]
use crate::stage_ledger::{
    hash40, AllocationError, CloneStage, Form, InstallPlan, StageAllocator, VANILLA_PLACES,
    VANILLA_STAGE_IDS,
};
#[cfg(test)]
use stage_ledger::{
    hash40, AllocationError, CloneStage, Form, InstallPlan, StageAllocator, VANILLA_PLACES,
    VANILLA_STAGE_IDS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredStage {
    pub place_name: String,
    pub asset_place: String,
    pub behaviour_place: String,
    pub place_hash: u64,
    pub place: u32,
    pub forms: Vec<(u32, Form)>,
    pub plan: InstallPlan,
    pub installed: bool,
    pub row_registered: bool,
}

impl RegisteredStage {
    pub fn stage_id_for(&self, form: Form) -> Option<u32> {
        self.forms
            .iter()
            .find(|(_, minted)| *minted == form)
            .map(|(stage_id, _)| *stage_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    NoBackend,
    Duplicate(String),
    Allocation(AllocationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacities {
    pub places: u32,
    pub stage_ids: u32,
}

impl Capacities {
    pub const fn vanilla() -> Self {
        Self {
            places: VANILLA_PLACES,
            stage_ids: VANILLA_STAGE_IDS,
        }
    }

    pub fn can_mint(&self) -> bool {
        self.places > VANILLA_PLACES && self.stage_ids > VANILLA_STAGE_IDS
    }
}

pub struct Registry {
    allocator: StageAllocator,
    capacities: Capacities,
    by_name: HashMap<String, usize>,
    by_place_hash: HashMap<u64, usize>,
    stages: Vec<RegisteredStage>,
}

impl Registry {
    pub fn new(capacities: Capacities) -> Self {
        Self {
            allocator: StageAllocator::new(capacities.places, capacities.stage_ids),
            capacities,
            by_name: HashMap::new(),
            by_place_hash: HashMap::new(),
            stages: Vec::new(),
        }
    }

    pub fn capacities(&self) -> Capacities {
        self.capacities
    }

    pub fn len(&self) -> usize {
        self.stages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    pub fn set_behaviour(&mut self, place_name: &str, donor: &str) -> bool {
        let name = place_name.to_ascii_lowercase();
        match self.by_name.get(&name).copied() {
            Some(index) => {
                self.stages[index].behaviour_place = donor.to_ascii_lowercase();
                true
            }
            None => false,
        }
    }

    pub fn claim_row(&mut self, place_name: &str) -> bool {
        let name = place_name.to_ascii_lowercase();
        match self.by_name.get(&name).copied() {
            Some(index) if !self.stages[index].row_registered => {
                self.stages[index].row_registered = true;
                true
            }
            _ => false,
        }
    }

    pub fn stages(&self) -> &[RegisteredStage] {
        &self.stages
    }

    pub fn register(&mut self, stage: &CloneStage) -> Result<&RegisteredStage, RegistryError> {
        if !self.capacities.can_mint() {
            return Err(RegistryError::NoBackend);
        }
        if self.by_name.contains_key(&stage.place_name) {
            return Err(RegistryError::Duplicate(stage.place_name.clone()));
        }

        let plan = self
            .allocator
            .allocate(stage)
            .map_err(RegistryError::Allocation)?;

        let place_hash = hash40(&stage.place_name);
        let forms = plan
            .stage_ids
            .iter()
            .map(|minted| (minted.stage_id, minted.form))
            .collect();
        let index = self.stages.len();
        self.by_name.insert(stage.place_name.clone(), index);
        self.by_place_hash.insert(place_hash, index);
        self.stages.push(RegisteredStage {
            place_name: stage.place_name.clone(),
            asset_place: stage.asset_place().to_string(),
            behaviour_place: stage.asset_place().to_string(),
            place_hash,
            place: plan.place,
            forms,
            plan,
            installed: false,
            row_registered: false,
        });
        Ok(&self.stages[index])
    }

    pub fn by_name(&self, place_name: &str) -> Option<&RegisteredStage> {
        self.by_name
            .get(place_name)
            .map(|&index| &self.stages[index])
    }

    pub fn by_place_hash(&self, hash: u64) -> Option<&RegisteredStage> {
        self.by_place_hash
            .get(&hash)
            .map(|&index| &self.stages[index])
    }

    pub fn by_place(&self, place: u32) -> Option<&RegisteredStage> {
        self.stages.iter().find(|stage| stage.place == place)
    }

    pub fn pending(&self) -> impl Iterator<Item = &RegisteredStage> {
        self.stages.iter().filter(|stage| !stage.installed)
    }

    pub fn required_stage_id_length(&self) -> Option<u32> {
        self.stages
            .iter()
            .flat_map(|stage| stage.forms.iter().map(|(stage_id, _)| *stage_id))
            .max()
            .map(|highest| highest + 1)
    }

    pub fn required_place_length(&self) -> Option<u32> {
        self.stages
            .iter()
            .map(|stage| stage.place)
            .max()
            .map(|highest| highest + 1)
    }

    pub fn mark_installed(&mut self, place_name: &str) -> bool {
        match self.by_name.get(place_name) {
            Some(&index) => {
                self.stages[index].installed = true;
                true
            }
            None => false,
        }
    }
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

#[cfg(not(test))]
pub fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::new(declared_capacities())))
}

#[cfg(not(test))]
fn declared_capacities() -> Capacities {
    let declared = |name: &str| {
        crate::stage_bounds::STAGE_TABLES
            .iter()
            .find(|table| table.name == name)
            .map(|table| table.expanded_length as u32)
            .unwrap_or(0)
    };

    let capacities = Capacities {
        places: if cfg!(feature = "stage_mint_places") {
            declared("stage_place")
        } else {
            VANILLA_PLACES
        },
        stage_ids: if cfg!(feature = "stage_mint") {
            declared("stage_id")
        } else {
            VANILLA_STAGE_IDS
        },
    };
    skyline::println!(
        "[stagereg] capacities: {} places, {} stage ids -> minting {}",
        capacities.places,
        capacities.stage_ids,
        if capacities.can_mint() {
            "available"
        } else {
            "UNAVAILABLE"
        },
    );
    capacities
}

#[cfg(all(not(test), feature = "stage_mint"))]
pub unsafe fn install_stage_id_backend() -> Option<(crate::stage_csk_table::VecFields, *mut u8)> {
    use crate::stage_backend::StageIdBackend;
    use crate::stage_csk_table as csk;

    let Some(plan) = crate::stage_backend::observed() else {
        skyline::println!("[stagecsk] the ownership survey has not been taken; standing down");
        return None;
    };
    if plan.stage_ids != StageIdBackend::ExtendForeign {
        return None;
    }

    let Some(base) = csk::resolve_csk() else {
        skyline::println!(
            "[stagecsk] stage_id is foreign-patched but `setup_stage_offseted` does not resolve, \
             so the owner is not a CSK we know. Standing down."
        );
        return None;
    };

    let read = |address: usize| core::ptr::read_volatile(address as *const u32);
    let fields = match csk::decode(base, &read) {
        Ok(fields) => fields,
        Err(error) => {
            skyline::println!(
                "[stagecsk] CSK at {base:#x} does not match the fingerprint: {error:?}"
            );
            return None;
        }
    };

    let pointer = core::ptr::read_volatile(fields.pointer_at as *const usize);
    let length = core::ptr::read_volatile(fields.length_at as *const usize);
    let wanted = crate::stage_bounds::STAGE_TABLES
        .iter()
        .find(|table| table.name == "stage_id")
        .map(|table| table.expanded_length)
        .unwrap_or(0);

    let growth = match csk::plan(fields, pointer, length, wanted) {
        Ok(growth) => growth,
        Err(error) => {
            skyline::println!("[stagecsk] not growing CSK's stage_id table: {error:?}");
            return None;
        }
    };

    let Some(block) = crate::stage_transaction::foreign_stage_id_block() else {
        skyline::println!(
            "[stagecsk] no RW block was reserved for the grown table; not growing. The \
             [stagereloc] lines say why the transaction did not reserve one."
        );
        return None;
    };
    let buffer = csk::apply(&growth, block as *mut u8);
    if buffer.is_null() {
        return None;
    }

    let live_capacity = core::ptr::read_volatile(fields.capacity_at as *const usize);
    let live_pointer = core::ptr::read_volatile(fields.pointer_at as *const usize);
    let live_length = core::ptr::read_volatile(fields.length_at as *const usize);
    skyline::println!(
        "[stagecsk] foreign Vec now reads cap={} ptr={:#x} len={} -- ptr {} our block {:#x}",
        live_capacity,
        live_pointer,
        live_length,
        if live_pointer == block { "==" } else { "!=" },
        block,
    );

    let stragglers: Vec<usize> = plan
        .verdicts
        .iter()
        .find(|verdict| verdict.table == "stage_id")
        .map(|verdict| verdict.vanilla_at.clone())
        .unwrap_or_default();
    let (retaken, skipped) = crate::stage_transaction::retake_foreign_sites(block);
    if retaken == 0 {
        skyline::println!(
            "[stagecsk] could not retake any stage_id site ({skipped} unrecognised);              minted ids would be invisible, so the bounds stay narrow"
        );
        *FOREIGN_STAGE_ID
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((fields, buffer as usize));
        return Some((fields, buffer));
    }
    if skipped != 0 {
        skyline::println!(
            "[stagecsk] {skipped} stage_id site(s) were left alone, so they still read the              old table; a minted id reaching one of them will be rejected"
        );
    }

    crate::stage_transaction::widen_bounds(&["stage_id"]);
    *FOREIGN_STAGE_ID
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some((fields, buffer as usize));
    Some((fields, buffer))
}

#[cfg(all(not(test), feature = "stage_mint"))]
static FOREIGN_STAGE_ID: Mutex<Option<(crate::stage_csk_table::VecFields, usize)>> =
    Mutex::new(None);

#[cfg(all(not(test), feature = "stage_mint"))]
fn warn_once(message: &str) {
    static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !SAID.swap(true, std::sync::atomic::Ordering::AcqRel) {
        skyline::println!("[stagereg] stages are minted but not installed: {message}");
    }
}

#[cfg(all(not(test), feature = "stage_mint"))]
pub unsafe fn install_pending() {
    use crate::stage_ledger::{PLACE_AUX_ROW, PLACE_ROW, STAGE_ID_ROW};

    let Ok(mut registry) = registry().lock() else {
        return;
    };
    if registry.pending().next().is_none() {
        return;
    }

    let place_base = crate::stage_transaction::table_base("stage_place");
    let aux_base = crate::stage_transaction::table_base("stage_place_aux");
    let (Some(place_base), Some(aux_base)) = (place_base, aux_base) else {
        warn_once("the place tables are not relocated, so no minted stage can be installed");
        return;
    };

    let foreign = *FOREIGN_STAGE_ID
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let stage_id_base = match foreign {
        Some((_, buffer)) => buffer,
        None => match crate::stage_transaction::table_base("stage_id") {
            Some(base) => base,
            None => {
                warn_once(
                    "there is nowhere to write a stage_id row: we did not relocate that table \
                     and the foreign one was not grown. See the [stagecsk] line.",
                );
                return;
            }
        },
    };

    let mut installed = Vec::new();
    let mut highest = 0u32;
    for stage in registry.pending() {
        let plan = &stage.plan;
        core::ptr::copy_nonoverlapping(
            plan.place_row.as_ptr(),
            (place_base + plan.place as usize * PLACE_ROW) as *mut u8,
            PLACE_ROW,
        );
        core::ptr::copy_nonoverlapping(
            plan.place_aux_row.as_ptr(),
            (aux_base + plan.place as usize * PLACE_AUX_ROW) as *mut u8,
            PLACE_AUX_ROW,
        );
        for minted in &plan.stage_ids {
            core::ptr::copy_nonoverlapping(
                minted.row.as_ptr(),
                (stage_id_base + minted.stage_id as usize * STAGE_ID_ROW) as *mut u8,
                STAGE_ID_ROW,
            );
            highest = highest.max(minted.stage_id);
        }
        skyline::println!(
            "[stagereg] installed {}: place row at index {}, {} stage id row(s)",
            stage.place_name,
            plan.place,
            plan.stage_ids.len(),
        );
        installed.push(stage.place_name.clone());
    }

    if let Some((fields, _)) = foreign {
        crate::stage_csk_table::publish_length(fields, highest as usize + 1);
        skyline::println!(
            "[stagecsk] published length {} on CSK's stage_id table",
            highest + 1
        );
    }

    for stage in registry.stages() {
        if !installed.contains(&stage.place_name) {
            continue;
        }
        let donor_hash = crate::stage_ledger::hash40(&stage.behaviour_place);
        if donor_hash == stage.place_hash {
            continue;
        }
        let donor_place = (0..VANILLA_PLACES).find(|index| {
            core::ptr::read_volatile(
                (place_base + *index as usize * PLACE_ROW + 0x08) as *const u64,
            ) & 0xFF_FFFF_FFFF
                == donor_hash
        });
        let Some(donor_place) = donor_place else {
            skyline::println!(
                "[stagedisp] {}: behaviour donor {:?} is not a vanilla place; stays generic",
                stage.place_name,
                stage.behaviour_place,
            );
            continue;
        };
        for (stage_id, form) in &stage.forms {
            let donor_form =
                if donor_hash == crate::stage_ledger::hash40("end") && *form == Form::Omega {
                    Form::Normal
                } else {
                    *form
                };
            let wanted = donor_form as u64;
            let donor_id = (0..VANILLA_STAGE_IDS).find(|index| {
                let row = stage_id_base + *index as usize * STAGE_ID_ROW;
                core::ptr::read_volatile((row + 0x04) as *const u32) == donor_place
                    && core::ptr::read_volatile((row + 0x08) as *const u64) == wanted
            });
            match donor_id {
                Some(donor_id) => {
                    let recorded = crate::stage_dispatch::set_donor_kind(*stage_id, donor_id);
                    skyline::println!(
                        "[stagedisp] stage id {} will be built as StageID {}{}{}",
                        stage_id,
                        donor_id,
                        if donor_form != *form {
                            " (End Omega aliases Normal)"
                        } else {
                            ""
                        },
                        if recorded { "" } else { "  <-- NO FREE DONOR SLOT" },
                    );
                }
                None => skyline::println!(
                    "[stagedisp] stage id {}: donor place {} has no form {} row; behaviour stays                      generic",
                    stage_id,
                    donor_place,
                    wanted,
                ),
            }
        }
    }

    for stage in registry.stages() {
        if !installed.contains(&stage.place_name) {
            continue;
        }
        let place_name_hash = core::ptr::read_volatile(
            (place_base + stage.place as usize * PLACE_ROW + 0x08) as *const u64,
        );
        skyline::println!(
            "[stagereadback] place[{}] +0x08 = {:#x} (want {:#x}) {}",
            stage.place,
            place_name_hash,
            stage.place_hash,
            if place_name_hash == stage.place_hash {
                "OK"
            } else {
                "MISMATCH"
            },
        );
        for (stage_id, _) in &stage.forms {
            let row = stage_id_base + *stage_id as usize * STAGE_ID_ROW;
            skyline::println!(
                "[stagereadback] stage_id[{}] place={} form={} load_group={:#x}",
                stage_id,
                core::ptr::read_volatile((row + 0x04) as *const u32),
                core::ptr::read_volatile((row + 0x08) as *const u64),
                core::ptr::read_volatile((row + 0x18) as *const u64),
            );
        }
    }

    for name in installed {
        registry.mark_installed(&name);
    }
}

const FORM_BITS: [(u32, Form); 3] = [
    (1 << 0, Form::Normal),
    (1 << 1, Form::Omega),
    (1 << 2, Form::Battlefield),
];

#[cfg(not(test))]
fn forms_from_mask(mask: u32) -> Vec<Form> {
    FORM_BITS
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, form)| *form)
        .collect()
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_stage_capacity(places: *mut u32, stage_ids: *mut u32) -> i32 {
    let capacities = registry().lock().map(|registry| registry.capacities());
    let Ok(capacities) = capacities else {
        return -1;
    };
    if !places.is_null() {
        places.write(capacities.places);
    }
    if !stage_ids.is_null() {
        stage_ids.write(capacities.stage_ids);
    }
    skyline::println!(
        "[stagereg] capacity query: {} places, {} stage ids (vanilla {}, {})",
        capacities.places,
        capacities.stage_ids,
        VANILLA_PLACES,
        VANILLA_STAGE_IDS,
    );
    i32::from(capacities.can_mint())
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_allocate_stage(
    place_name: *const core::ffi::c_char,
    resource_place: *const core::ffi::c_char,
    ships_battle_tree: bool,
    forms_mask: u32,
) -> i32 {
    if place_name.is_null() {
        return -1;
    }
    let Ok(name) = core::ffi::CStr::from_ptr(place_name).to_str() else {
        return -2;
    };
    if name.is_empty() {
        return -2;
    }
    let forms = forms_from_mask(forms_mask);
    if forms.is_empty() {
        skyline::println!("[stagereg] refused {name}: forms_mask {forms_mask:#x} selects no form");
        return -3;
    }

    let mut stage = CloneStage::new(name);
    stage.ships_battle_tree = ships_battle_tree;
    stage.forms = forms;
    if !resource_place.is_null() {
        match core::ffi::CStr::from_ptr(resource_place).to_str() {
            Ok(place) if !place.is_empty() => {
                stage.resource_place = Some(place.to_ascii_lowercase())
            }
            _ => return -2,
        }
    }

    let Ok(mut registry) = registry().lock() else {
        return -4;
    };
    if let Some(minted) = registry.by_name(&stage.place_name) {
        let place = minted.place;
        skyline::println!("[stagereg] {name} is already minted at place {place}; reusing it");
        return place as i32;
    }
    match registry.register(&stage) {
        Ok(minted) => {
            skyline::println!(
                "[stagereg] minted {} -> place {}, stage ids {:?}, assets from {}",
                minted.place_name,
                minted.place,
                minted.forms,
                minted.asset_place,
            );
            minted.place as i32
        }
        Err(RegistryError::NoBackend) => {
            skyline::println!(
                "[stagereg] refused {name}: no stage backend on this card. Build the engine with \
                 --features stage_mint, and check the [stageown] lines for which table is not ours."
            );
            -5
        }
        Err(RegistryError::Duplicate(existing)) => {
            skyline::println!("[stagereg] refused {name}: {existing} is already registered");
            -6
        }
        Err(RegistryError::Allocation(error)) => {
            skyline::println!("[stagereg] refused {name}: {error:?}");
            -7
        }
    }
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_set_stage_behaviour(
    place_name: *const core::ffi::c_char,
    donor_place: *const core::ffi::c_char,
) -> i32 {
    if place_name.is_null() || donor_place.is_null() {
        return -1;
    }
    let (Ok(name), Ok(donor)) = (
        core::ffi::CStr::from_ptr(place_name).to_str(),
        core::ffi::CStr::from_ptr(donor_place).to_str(),
    ) else {
        return -2;
    };
    if name.is_empty() || donor.is_empty() {
        return -2;
    }
    let Ok(mut registry) = registry().lock() else {
        return -4;
    };
    if registry.set_behaviour(name, donor) {
        skyline::println!("[stagereg] {name} will be built as the stage class of {donor}");
        0
    } else {
        skyline::println!(
            "[stagereg] cannot set behaviour for {name}: it has not been allocated on this card"
        );
        -5
    }
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_stage_id_for(
    place_name: *const core::ffi::c_char,
    form: u32,
) -> i32 {
    if place_name.is_null() {
        return -1;
    }
    let Ok(name) = core::ffi::CStr::from_ptr(place_name).to_str() else {
        return -2;
    };
    let Some(&(_, wanted)) = FORM_BITS.get(form as usize) else {
        return -3;
    };
    let Ok(registry) = registry().lock() else {
        return -4;
    };
    match registry
        .by_name(name)
        .and_then(|stage| stage.stage_id_for(wanted))
    {
        Some(stage_id) => stage_id as i32,
        None => -5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPANDED: Capacities = Capacities {
        places: 192,
        stage_ids: 512,
    };

    fn stage(name: &str, forms: &[Form]) -> CloneStage {
        let mut stage = CloneStage::new(name);
        stage.forms = forms.to_vec();
        stage
    }

    fn three(name: &str) -> CloneStage {
        stage(name, &[Form::Normal, Form::Omega, Form::Battlefield])
    }

    #[test]
    fn vanilla_capacities_mint_nothing() {
        let mut registry = Registry::new(Capacities::vanilla());
        assert!(!registry.capacities().can_mint());
        assert_eq!(
            registry.register(&three("pumpkin_hill")),
            Err(RegistryError::NoBackend)
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn one_stage_gets_the_first_free_numbers() {
        let mut registry = Registry::new(EXPANDED);
        let minted = registry.register(&three("pumpkin_hill")).unwrap().clone();
        assert_eq!(minted.place, VANILLA_PLACES);
        assert_eq!(
            minted.forms,
            vec![
                (VANILLA_STAGE_IDS, Form::Normal),
                (VANILLA_STAGE_IDS + 1, Form::Omega),
                (VANILLA_STAGE_IDS + 2, Form::Battlefield),
            ]
        );
        assert!(!minted.installed);
    }

    #[test]
    fn numbers_never_overlap_between_stages() {
        let mut registry = Registry::new(EXPANDED);
        registry.register(&three("pumpkin_hill")).unwrap();
        registry.register(&three("emerald_coast")).unwrap();
        registry
            .register(&stage("sky_deck", &[Form::Normal]))
            .unwrap();

        let mut places: Vec<u32> = registry.stages().iter().map(|s| s.place).collect();
        let mut ids: Vec<u32> = registry
            .stages()
            .iter()
            .flat_map(|s| s.forms.iter().map(|(id, _)| *id))
            .collect();
        let (place_count, id_count) = (places.len(), ids.len());
        places.sort_unstable();
        places.dedup();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            places.len(),
            place_count,
            "a place index was handed out twice"
        );
        assert_eq!(ids.len(), id_count, "a stage id was handed out twice");
        assert_eq!(id_count, 7);
    }

    #[test]
    fn the_same_place_name_cannot_be_registered_twice() {
        let mut registry = Registry::new(EXPANDED);
        registry.register(&three("pumpkin_hill")).unwrap();
        assert_eq!(
            registry.register(&three("pumpkin_hill")),
            Err(RegistryError::Duplicate("pumpkin_hill".into()))
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn a_refused_duplicate_consumes_no_numbers() {
        let mut registry = Registry::new(EXPANDED);
        registry.register(&three("pumpkin_hill")).unwrap();
        let _ = registry.register(&three("pumpkin_hill"));
        let next = registry.register(&three("emerald_coast")).unwrap();
        assert_eq!(next.place, VANILLA_PLACES + 1);
        assert_eq!(next.forms[0].0, VANILLA_STAGE_IDS + 3);
    }

    #[test]
    fn lookups_find_a_stage_by_every_key_the_game_uses() {
        let mut registry = Registry::new(EXPANDED);
        registry.register(&three("pumpkin_hill")).unwrap();
        assert!(registry.by_name("pumpkin_hill").is_some());
        assert!(registry.by_place_hash(hash40("pumpkin_hill")).is_some());
        assert!(registry.by_place(VANILLA_PLACES).is_some());
        assert!(registry.by_name("emerald_coast").is_none());
        assert!(registry.by_place_hash(hash40("emerald_coast")).is_none());
    }

    #[test]
    fn required_lengths_cover_every_minted_number() {
        let mut registry = Registry::new(EXPANDED);
        assert_eq!(registry.required_stage_id_length(), None);
        assert_eq!(registry.required_place_length(), None);
        registry.register(&three("pumpkin_hill")).unwrap();
        registry.register(&three("emerald_coast")).unwrap();
        assert_eq!(
            registry.required_stage_id_length(),
            Some(VANILLA_STAGE_IDS + 6)
        );
        assert_eq!(registry.required_place_length(), Some(VANILLA_PLACES + 2));
    }

    #[test]
    fn running_out_of_stage_ids_refuses_rather_than_wrapping() {
        let mut registry = Registry::new(Capacities {
            places: 192,
            stage_ids: VANILLA_STAGE_IDS + 2,
        });
        assert!(matches!(
            registry.register(&three("pumpkin_hill")),
            Err(RegistryError::Allocation(
                AllocationError::OutOfStageIds { .. }
            ))
        ));
        assert!(registry.is_empty());
    }

    #[test]
    fn running_out_of_places_refuses_too() {
        let mut registry = Registry::new(Capacities {
            places: VANILLA_PLACES + 1,
            stage_ids: 512,
        });
        registry.register(&three("pumpkin_hill")).unwrap();
        assert!(matches!(
            registry.register(&three("emerald_coast")),
            Err(RegistryError::Allocation(
                AllocationError::OutOfPlaces { .. }
            ))
        ));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn install_marking_moves_a_stage_out_of_pending() {
        let mut registry = Registry::new(EXPANDED);
        registry.register(&three("pumpkin_hill")).unwrap();
        registry.register(&three("emerald_coast")).unwrap();
        assert_eq!(registry.pending().count(), 2);
        assert!(registry.mark_installed("pumpkin_hill"));
        assert_eq!(registry.pending().count(), 1);
        assert_eq!(
            registry.pending().next().unwrap().place_name,
            "emerald_coast"
        );
        assert!(!registry.mark_installed("never_registered"));
    }

    #[test]
    fn a_form_the_stage_did_not_ask_for_has_no_id() {
        let mut registry = Registry::new(EXPANDED);
        let minted = registry
            .register(&stage("sky_deck", &[Form::Normal]))
            .unwrap();
        assert_eq!(minted.stage_id_for(Form::Normal), Some(VANILLA_STAGE_IDS));
        assert_eq!(minted.stage_id_for(Form::Omega), None);
    }

    #[test]
    fn the_recorded_plan_is_the_one_that_gets_written() {
        let mut registry = Registry::new(EXPANDED);
        let minted = registry.register(&three("pumpkin_hill")).unwrap();
        assert_eq!(minted.plan.place, minted.place);
        assert_eq!(minted.plan.stage_ids.len(), minted.forms.len());
        for (index, (stage_id, form)) in minted.forms.iter().enumerate() {
            assert_eq!(minted.plan.stage_ids[index].stage_id, *stage_id);
            assert_eq!(minted.plan.stage_ids[index].form, *form);
            let row = &minted.plan.stage_ids[index].row;
            assert_eq!(
                u32::from_le_bytes(row[4..8].try_into().unwrap()),
                minted.place
            );
        }
    }
}

#[cfg(test)]
fn declared_capacities() -> Capacities {
    let declared = |name: &str| {
        crate::stage_bounds::STAGE_TABLES
            .iter()
            .find(|table| table.name == name)
            .map(|table| table.expanded_length as u32)
            .unwrap_or(0)
    };
    Capacities {
        places: if cfg!(feature = "stage_mint_places") {
            declared("stage_place")
        } else {
            VANILLA_PLACES
        },
        stage_ids: if cfg!(feature = "stage_mint") {
            declared("stage_id")
        } else {
            VANILLA_STAGE_IDS
        },
    }
}

#[cfg(test)]
pub fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::new(declared_capacities())))
}

#[cfg(all(test, feature = "stage_mint"))]
pub unsafe fn install_stage_id_backend() -> Option<(crate::stage_csk_table::VecFields, *mut u8)> {
    None
}

#[cfg(all(test, feature = "stage_mint"))]
pub unsafe fn install_pending() {}
