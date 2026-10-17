use core::sync::atomic::AtomicUsize;

use smash::app::lua_bind::{PostureModule, StatusModule, WorkModule};
use smash::app::{sv_battle_object, utility, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;

use crate::{resolve, true_kind, FIRST_CUSTOM_KIND};

type PartnerOfFn = extern "C" fn(i32, u32) -> u32;
type IsPartnerFn = extern "C" fn(i32, u32) -> bool;
type SwapFn = extern "C" fn(i32) -> bool;

static PARTNER_OF_FN: AtomicUsize = AtomicUsize::new(0);
static IS_PARTNER_FN: AtomicUsize = AtomicUsize::new(0);
static SWAP_FN: AtomicUsize = AtomicUsize::new(0);

const BOMA_BATTLE_OBJECT_ID: usize = 8;

unsafe fn ids(boma: *mut BattleObjectModuleAccessor) -> (i32, u32) {
    (
        WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID),
        core::ptr::read_volatile((boma as usize + BOMA_BATTLE_OBJECT_ID) as *const u32),
    )
}

pub unsafe fn is_partner(boma: *mut BattleObjectModuleAccessor) -> bool {
    if boma.is_null() || true_kind(boma) < FIRST_CUSTOM_KIND {
        return false;
    }
    if let Some(address) = resolve(&IS_PARTNER_FN, b"clone_engine_is_partner_v1\0") {
        let function: IsPartnerFn = core::mem::transmute(address);
        let (entry, id) = ids(boma);
        return function(entry, id);
    }
    utility::get_kind(&mut *boma) == *FIGHTER_KIND_NANA
}

pub unsafe fn has_partner(boma: *mut BattleObjectModuleAccessor) -> bool {
    on_stage(boma).is_some()
}

unsafe fn on_stage(boma: *mut BattleObjectModuleAccessor) -> Option<*mut BattleObjectModuleAccessor> {
    let partner = of(boma)?;
    let status = StatusModule::status_kind(partner);
    (status != *FIGHTER_STATUS_KIND_STANDBY && status != *FIGHTER_STATUS_KIND_DEAD).then_some(partner)
}

pub unsafe fn of(boma: *mut BattleObjectModuleAccessor) -> Option<*mut BattleObjectModuleAccessor> {
    if boma.is_null() || true_kind(boma) < FIRST_CUSTOM_KIND {
        return None;
    }
    let id = if let Some(address) = resolve(&PARTNER_OF_FN, b"clone_engine_partner_of_v1\0") {
        let function: PartnerOfFn = core::mem::transmute(address);
        let (entry, own) = ids(boma);
        function(entry, own)
    } else {
        let kind = utility::get_kind(&mut *boma);
        if kind != *FIGHTER_KIND_POPO && kind != *FIGHTER_KIND_NANA {
            return None;
        }
        WorkModule::get_int(boma, *FIGHTER_POPO_INSTANCE_WORK_ID_INT_PARTNER_OBJECT_ID) as u32
    };
    if id == *BATTLE_OBJECT_ID_INVALID as u32 || !sv_battle_object::is_active(id) {
        return None;
    }
    let partner = sv_battle_object::module_accessor(id);
    (!partner.is_null() && partner != boma).then_some(partner)
}

pub unsafe fn distance(boma: *mut BattleObjectModuleAccessor) -> Option<f32> {
    let partner = on_stage(boma)?;
    let here = *PostureModule::pos(boma);
    let there = *PostureModule::pos(partner);
    let (x, y) = (there.x - here.x, there.y - here.y);
    Some((x * x + y * y).sqrt())
}

pub unsafe fn swap(boma: *mut BattleObjectModuleAccessor) -> bool {
    if boma.is_null() {
        return false;
    }
    let Some(address) = resolve(&SWAP_FN, b"clone_engine_partner_swap_v1\0") else {
        return false;
    };
    let function: SwapFn = core::mem::transmute(address);
    function(WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID))
}
