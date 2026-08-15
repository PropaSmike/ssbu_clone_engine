use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const API_VERSION_V1: u32 = 1;
pub const API_VERSION_V2: u32 = 2;
pub const FIRST_CUSTOM_KIND: i32 = 118;
pub const MAX_PROVEN_CUSTOM_KIND: i32 = 119;

pub const RESULT_OK: i32 = 0;
pub const ERROR_NULL: i32 = -1;
pub const ERROR_VERSION: i32 = -2;
pub const ERROR_STRUCT_SIZE: i32 = -3;
pub const ERROR_CUSTOM_KIND: i32 = -4;
pub const ERROR_BASE_KIND: i32 = -5;
pub const ERROR_NAME: i32 = -6;
pub const ERROR_COLOR_RANGE: i32 = -7;
pub const ERROR_DUPLICATE: i32 = -8;
pub const ERROR_ARTICLE: i32 = -9;
pub const ERROR_NAMESPACE: i32 = -10;

pub const FLAG_OWNS_PARAM_RESOURCES: u32 = 1;
pub const FLAG_KIRBY_COPY_FULL_MODEL: u32 = 1 << 1;
pub const ERROR_UNSUPPORTED: i32 = -11;
pub const ERROR_BACKEND_UNAVAILABLE: i32 = -12;
pub const ERROR_REGISTRATION_CLOSED: i32 = -13;
pub const ERROR_ARTICLE_OWNER: i32 = -14;
pub const ERROR_ARTICLE_SOURCE: i32 = -15;
pub const ERROR_ARTICLE_CAPACITY: i32 = -16;
pub const ERROR_ARTICLE_RESOURCE_CONFLICT: i32 = -17;
pub const ERROR_HOOK_CAPACITY: i32 = -18;
pub const ERROR_HOOK_UNAVAILABLE: i32 = -19;
pub const ERROR_HOOK_PREFLIGHT: i32 = -20;
pub const ERROR_HOOK_ABI: i32 = -21;
pub const ERROR_HOOK_CONFLICT: i32 = -22;
pub const ERROR_HOOK_INSTALL: i32 = -23;
pub const ERROR_ITEM_FAMILY_EMPTY: i32 = -24;
pub const ERROR_ITEM_FAMILY_LAYOUT: i32 = -25;
pub const ERROR_ITEM_FAMILY_CAPACITY: i32 = -26;
pub const ERROR_ITEM_CATEGORY: i32 = -27;
pub const ERROR_ITEM_AGENT_UNAVAILABLE: i32 = -28;
pub const ERROR_ITEM_RESOURCE_UNAVAILABLE: i32 = -29;
pub const ERROR_ITEM_SPAWN_UNAVAILABLE: i32 = -30;
pub const ERROR_ITEM_UI_METADATA: i32 = -31;
pub const ERROR_ITEM_UI_CAPACITY: i32 = -32;
pub const ERROR_ITEM_UI_UNAVAILABLE: i32 = -33;
pub const ERROR_SMASHLINE_REQUIRED: i32 = -34;

pub const SMASHLINE_BRIDGE_VERSION_REQUIRED: u32 = 2;

pub const KIND_AUTO: i32 = -1;

pub const BACKEND_STATUS_COMPILED: u32 = 1 << 0;
pub const BACKEND_STATUS_STATIC_PREFLIGHT_OK: u32 = 1 << 1;
pub const BACKEND_STATUS_STATIC_TABLES_READY: u32 = 1 << 2;
pub const BACKEND_STATUS_RESOURCE_LAYOUT_PROVEN: u32 = 1 << 3;
pub const BACKEND_STATUS_RESOURCE_LIFECYCLE_PROVEN: u32 = 1 << 4;
pub const BACKEND_STATUS_HOOKS_INSTALLED: u32 = 1 << 5;
pub const BACKEND_STATUS_REGISTRATION_CLOSED: u32 = 1 << 6;
pub const BACKEND_STATUS_READY: u32 = 1 << 31;

pub const FIRST_CUSTOM_ITEM_KIND: i32 = 0x36A;

pub const ITEM_BACKEND_STATUS_COMPILED: u32 = 1 << 0;
pub const ITEM_BACKEND_STATUS_MAIN_PREFLIGHT_OK: u32 = 1 << 1;
pub const ITEM_BACKEND_STATUS_IDENTITY_HOOKS_READY: u32 = 1 << 2;
pub const ITEM_BACKEND_STATUS_ITEM_NRO_READY: u32 = 1 << 3;
pub const ITEM_BACKEND_STATUS_STATUS_ROUTER_READY: u32 = 1 << 4;
pub const ITEM_BACKEND_STATUS_REGISTRATION_CLOSED: u32 = 1 << 5;
pub const ITEM_BACKEND_STATUS_RESOURCE_ROUTER_READY: u32 = 1 << 6;
pub const ITEM_BACKEND_STATUS_PARAM_ROUTER_READY: u32 = 1 << 7;
pub const ITEM_BACKEND_STATUS_CATEGORY_ROUTER_READY: u32 = 1 << 8;
pub const ITEM_BACKEND_STATUS_FAMILY_ROUTER_READY: u32 = 1 << 9;
pub const ITEM_BACKEND_STATUS_MULTI_BASE_STATUS_ROUTER_READY: u32 = 1 << 10;
pub const ITEM_BACKEND_STATUS_ASSIST_GENERATION_READY: u32 = 1 << 11;
pub const ITEM_BACKEND_STATUS_POKEMON_GENERATION_READY: u32 = 1 << 12;
pub const ITEM_BACKEND_STATUS_BOSS_LIFECYCLE_READY: u32 = 1 << 13;
pub const ITEM_BACKEND_STATUS_EFFECT_SOUND_ROUTER_READY: u32 = 1 << 14;
pub const ITEM_BACKEND_STATUS_TRAINING_UI_READY: u32 = 1 << 15;
pub const ITEM_BACKEND_STATUS_RULES_UI_READY: u32 = 1 << 16;
pub const ITEM_BACKEND_STATUS_READY: u32 = 1 << 31;

pub const CAP_FIGHTER_IDENTITY: u64 = 1 << 0;
pub const CAP_SMASHLINE_BRIDGE: u64 = 1 << 1;
pub const CAP_FIGHTER_ARTICLES: u64 = 1 << 2;
pub const CAP_KIRBY_COPY: u64 = 1 << 3;
pub const CAP_PARAMCONFIG_BRIDGE: u64 = 1 << 4;
pub const CAP_SHARED_HOOKS: u64 = 1 << 5;
pub const CAP_ITEM_IDENTITY: u64 = 1 << 6;
pub const CAP_ITEM_RESOURCES: u64 = 1 << 7;
pub const CAP_ITEM_PARAMS: u64 = 1 << 8;
pub const CAP_ITEM_ANIMCMD: u64 = 1 << 9;
pub const CAP_ITEM_STATUS: u64 = 1 << 10;
pub const CAP_ITEM_TRAINING_UI: u64 = 1 << 11;
pub const CAP_STAGE_MINT: u64 = 1 << 12;
pub const CAP_STAGE_CONFIG: u64 = 1 << 13;
pub const CAP_STAGE_SELECT_EXTENDED: u64 = 1 << 14;
pub const CAP_STAGE_CSK: u64 = 1 << 15;
pub const CAP_RESEARCH_ITEM_FAMILIES: u64 = 1 << 32;
pub const CAP_RESEARCH_ITEM_MULTIBASE_STATUS_RESERVED: u64 = 1 << 33;

pub const STAGE_FORM_NORMAL: u32 = 1 << 0;
pub const STAGE_FORM_OMEGA: u32 = 1 << 1;
pub const STAGE_FORM_BATTLEFIELD: u32 = 1 << 2;

pub const ITEM_UI_FLAG_TRAINING: u32 = 1 << 0;
pub const ITEM_UI_FLAG_RULES: u32 = 1 << 1;
pub const ITEM_UI_FLAG_POKEBALL: u32 = 1 << 2;
pub const ITEM_UI_FLAG_MASTERBALL: u32 = 1 << 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ItemCategory {
    Unknown = 0,
    Item = 1,
    Assist = 2,
    Pokemon = 3,
    Boss = 4,
}

impl ItemCategory {
    fn from_raw(value: u32) -> Self {
        match value {
            1 => Self::Item,
            2 => Self::Assist,
            3 => Self::Pokemon,
            4 => Self::Boss,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ItemSpawnSource {
    Unknown = 0,
    Direct = 1,
    Assist = 2,
    PokeBall = 3,
    MasterBall = 4,
    Boss = 5,
    FamilyChild = 6,
}

impl ItemSpawnSource {
    fn from_raw(value: u32) -> Self {
        match value {
            1 => Self::Direct,
            2 => Self::Assist,
            3 => Self::PokeBall,
            4 => Self::MasterBall,
            5 => Self::Boss,
            6 => Self::FamilyChild,
            _ => Self::Unknown,
        }
    }
}

pub const SHARED_HOOK_ABI_GPR_X0: u32 = 1;
pub const SHARED_HOOK_STATUS_INITIALIZED: u32 = 1 << 0;
pub const SHARED_HOOK_STATUS_SELF_TEST_OK: u32 = 1 << 1;
pub const SHARED_HOOK_STATUS_INSTALL_FAILED: u32 = 1 << 2;
pub const SHARED_HOOK_STATUS_REGISTRATION_CLOSED: u32 = 1 << 3;
pub const SHARED_HOOK_STATUS_READY: u32 = 1 << 31;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SharedHookRegistrationV1 {
    pub api_version: u32,
    pub struct_size: u32,
    pub offset: u64,
    pub expected_opcodes: [u32; 4],
    pub abi: u32,
    pub argument_count: u32,
    pub callback: usize,
    pub flags: u32,
    pub reserved_u32: u32,
    pub reserved: [u64; 1],
}

impl SharedHookRegistrationV1 {
    pub fn new(
        offset: u64,
        expected_opcodes: [u32; 4],
        argument_count: u32,
        callback: HookFn,
    ) -> Self {
        Self {
            api_version: API_VERSION_V1,
            struct_size: std::mem::size_of::<Self>() as u32,
            offset,
            expected_opcodes,
            abi: SHARED_HOOK_ABI_GPR_X0,
            argument_count,
            callback: callback as usize,
            flags: 0,
            reserved_u32: 0,
            reserved: [0],
        }
    }
}

const _: [(); 64] = [(); std::mem::size_of::<SharedHookRegistrationV1>()];

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CloneArticleRegistrationV1 {
    pub base_weapon_kind: i32,
    pub reserved: u32,
    pub file_name: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CloneItemRegistrationV1 {
    pub api_version: u32,
    pub struct_size: u32,
    pub item_kind: i32,
    pub base_item_kind: i32,
    pub resource_name: *const c_char,
    pub agent_name: *const c_char,
    pub flags: u32,
    pub reserved_u32: u32,
    pub reserved: [u64; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CloneItemFamilyMemberV2 {
    pub item_kind: i32,
    pub flags: u32,
    pub resource_name: *const c_char,
    pub agent_name: *const c_char,
    pub reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CloneItemFamilyRegistrationV2 {
    pub api_version: u32,
    pub struct_size: u32,
    pub base_owner_kind: i32,
    pub member_count: u32,
    pub member_struct_size: u32,
    pub flags: u32,
    pub members: *const CloneItemFamilyMemberV2,
    pub reserved: [u64; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CloneItemUiRegistrationV1 {
    pub api_version: u32,
    pub struct_size: u32,
    pub item_kind: i32,
    pub flags: u32,
    pub ui_id: *const c_char,
    pub training_order: i32,
    pub rules_order: i32,
    pub reserved: [u64; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CloneRegistrationV1 {
    pub api_version: u32,
    pub struct_size: u32,
    pub custom_kind: i32,
    pub base_kind: i32,
    pub ui_chara: *const c_char,
    pub fighter_kind_name: *const c_char,
    pub resource_name: *const c_char,
    pub base_resource_name: *const c_char,
    pub color_start: u32,
    pub color_count: u32,
    pub copy_status_first: i32,
    pub copy_status_count: i32,
    pub article_namespace: u32,
    pub effect_namespace: u32,
    pub articles: *const CloneArticleRegistrationV1,
    pub article_count: u32,
    pub flags: u32,
    pub reserved: [u64; 4],
}

const _: [(); 16] = [(); std::mem::size_of::<CloneArticleRegistrationV1>()];
const _: [(); 72] = [(); std::mem::size_of::<CloneItemRegistrationV1>()];
const _: [(); 40] = [(); std::mem::size_of::<CloneItemFamilyMemberV2>()];
const _: [(); 64] = [(); std::mem::size_of::<CloneItemFamilyRegistrationV2>()];
const _: [(); 64] = [(); std::mem::size_of::<CloneItemUiRegistrationV1>()];
const _: [(); 120] = [(); std::mem::size_of::<CloneRegistrationV1>()];

#[derive(Clone, Copy, Debug)]
pub struct Article<'a> {
    pub base_weapon_kind: i32,
    pub file_name: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemCloneRegistration<'a> {
    pub item_kind: i32,
    pub base_item_kind: i32,
    pub resource_name: &'a str,
    pub agent_name: &'a str,
}

impl<'a> ItemCloneRegistration<'a> {
    pub const fn new(
        item_kind: i32,
        base_item_kind: i32,
        resource_name: &'a str,
        agent_name: &'a str,
    ) -> Self {
        Self {
            item_kind,
            base_item_kind,
            resource_name,
            agent_name,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemUiRegistration<'a> {
    pub item_kind: i32,
    pub ui_id: &'a str,
    pub flags: u32,
    pub training_order: i32,
    pub rules_order: i32,
}

impl<'a> ItemUiRegistration<'a> {
    pub const fn training(item_kind: i32, ui_id: &'a str) -> Self {
        Self {
            item_kind,
            ui_id,
            flags: ITEM_UI_FLAG_TRAINING,
            training_order: 0,
            rules_order: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StageForm {
    Normal = 0,
    Omega = 1,
    Battlefield = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageCapacity {
    pub places: u32,
    pub stage_ids: u32,
    pub can_mint: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct StageAllocation<'a> {
    pub place_name: &'a str,
    pub resource_place: Option<&'a str>,
    pub ships_battle_tree: bool,
    pub forms: u32,
}

impl<'a> StageAllocation<'a> {
    pub const fn new(place_name: &'a str) -> Self {
        Self {
            place_name,
            resource_place: None,
            ships_battle_tree: false,
            forms: STAGE_FORM_NORMAL | STAGE_FORM_OMEGA | STAGE_FORM_BATTLEFIELD,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StageRegistration<'a> {
    pub place_name: &'a str,
    pub name_id: &'a str,
    pub ships_battle_tree: bool,
    pub ui_series_id: u64,
    pub display_order: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemFamilyMember<'a> {
    pub item_kind: i32,
    pub resource_name: &'a str,
    pub agent_name: &'a str,
}

impl<'a> ItemFamilyMember<'a> {
    pub const fn new(item_kind: i32, resource_name: &'a str, agent_name: &'a str) -> Self {
        Self {
            item_kind,
            resource_name,
            agent_name,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemFamilyRegistration<'a> {
    pub base_owner_kind: i32,
    pub members: &'a [ItemFamilyMember<'a>],
}

impl<'a> ItemFamilyRegistration<'a> {
    pub const fn new(base_owner_kind: i32, members: &'a [ItemFamilyMember<'a>]) -> Self {
        Self {
            base_owner_kind,
            members,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CloneRegistration<'a> {
    pub custom_kind: i32,
    pub base_kind: i32,
    pub ui_chara: &'a str,
    pub fighter_kind_name: &'a str,
    pub resource_name: &'a str,
    pub base_resource_name: &'a str,
    pub color_start: u32,
    pub color_count: u32,
    pub copy_status_first: i32,
    pub copy_status_count: i32,
    pub article_namespace: u32,
    pub effect_namespace: u32,
    pub articles: &'a [Article<'a>],
    pub flags: u32,
}

impl<'a> CloneRegistration<'a> {
    pub const fn new(
        custom_kind: i32,
        base_kind: i32,
        ui_chara: &'a str,
        fighter_kind_name: &'a str,
        resource_name: &'a str,
        base_resource_name: &'a str,
    ) -> Self {
        Self {
            custom_kind,
            base_kind,
            ui_chara,
            fighter_kind_name,
            resource_name,
            base_resource_name,
            color_start: 0,
            color_count: 1,
            copy_status_first: -1,
            copy_status_count: 0,
            article_namespace: 0,
            effect_namespace: 0,
            articles: &[],
            flags: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    EngineUnavailable,
    InvalidName,
    Engine(i32),
}

type ApiVersionFn = unsafe extern "C" fn() -> u32;
type CapabilitiesFn = unsafe extern "C" fn() -> u64;
type MaxCustomKindFn = unsafe extern "C" fn() -> i32;
type SmashlineBridgeVersionFn = unsafe extern "C" fn() -> u32;
type NativeBackendStatusFn = unsafe extern "C" fn() -> u32;
type RegisterV1Fn = unsafe extern "C" fn(*const CloneRegistrationV1) -> i32;
type KindQueryFn = unsafe extern "C" fn(i32) -> i32;
type EntryKindFn = unsafe extern "C" fn(i32) -> i32;
type IdentityKindFn = unsafe extern "C" fn(*const c_char) -> i32;
type CapacityCommittedFn = unsafe extern "C" fn() -> u32;
type ArmKirbyFamilyFn = unsafe extern "C" fn(i32, i32, i32) -> i32;
type CloneArticleFn = unsafe extern "C" fn(*const c_char, i32, *const c_char, *const c_char) -> i32;
type CloneArticleForFn =
    unsafe extern "C" fn(*const c_char, i32, *const c_char, *const c_char, *const c_char) -> i32;
type ArticleIndexFn = unsafe extern "C" fn(i32, i32) -> i32;
type CloneCopyArticleFn =
    unsafe extern "C" fn(i32, *const c_char, i32, *const c_char, *const c_char) -> i32;
type CopyArticleIndexFn = unsafe extern "C" fn(i32, i32) -> i32;
type LogFn = unsafe extern "C" fn(*const u8, usize);
type ParamOverrideFn = unsafe extern "C" fn(i32, i32, u64, u64, u32, f64) -> i32;
type ParamIntOverrideFn = unsafe extern "C" fn(i32, i32, u64, u64, i32) -> i32;
type ArticleOwnerKindFn = unsafe extern "C" fn(u64) -> i32;
type SharedHookFn = unsafe extern "C" fn(u64, usize) -> i32;
type SharedHookV2Fn = unsafe extern "C" fn(*const SharedHookRegistrationV1) -> i32;
type SharedHookStatusFn = unsafe extern "C" fn() -> u32;
type SharedHookOriginalFn = unsafe extern "C" fn(u64, *const [u64; 6]) -> u64;
type RegisterItemV1Fn = unsafe extern "C" fn(*const CloneItemRegistrationV1) -> i32;
type RegisterItemFamilyV2Fn = unsafe extern "C" fn(*const CloneItemFamilyRegistrationV2) -> i32;
type RegisterItemUiV1Fn = unsafe extern "C" fn(*const CloneItemUiRegistrationV1) -> i32;
type ItemBaseKindFn = unsafe extern "C" fn(i32) -> i32;
type IsItemKindFn = unsafe extern "C" fn(i32) -> bool;
type ItemResourceNameFn = unsafe extern "C" fn(i32) -> *const c_char;
type ItemObjectKindFn = unsafe extern "C" fn(*const c_void) -> i32;
type ItemKindQueryFn = unsafe extern "C" fn(i32) -> i32;
type ItemCategoryFn = unsafe extern "C" fn(i32) -> u32;
type ItemSpawnSourceFn = unsafe extern "C" fn(*const c_void) -> u32;
type ItemStatusFn = unsafe extern "C" fn(i32, i32, i32, usize) -> i32;
type ItemStatusNamedFn = unsafe extern "C" fn(i32, i32, *const c_char, usize) -> i32;
type ItemStatusKindFn = unsafe extern "C" fn(*const c_char) -> i32;
type ItemCommonSetFn = unsafe extern "C" fn(i32, u64, f32) -> i32;
type ItemCommonHasFn = unsafe extern "C" fn(u64) -> i32;
type ItemBackendStatusFn = unsafe extern "C" fn() -> u32;
type StageCapacityFn = unsafe extern "C" fn(*mut u32, *mut u32) -> i32;
type StageAllocateFn = unsafe extern "C" fn(*const c_char, *const c_char, bool, u32) -> i32;
type StageBehaviourFn = unsafe extern "C" fn(*const c_char, *const c_char) -> i32;
type StageIdFn = unsafe extern "C" fn(*const c_char, u32) -> i32;
type StageRegisterFn = unsafe extern "C" fn(*const c_char, *const c_char, bool, u64, i32) -> i32;

static API_VERSION_FN: AtomicUsize = AtomicUsize::new(0);
static COMPILED_CAPABILITIES_FN: AtomicUsize = AtomicUsize::new(0);
static RUNTIME_CAPABILITIES_FN: AtomicUsize = AtomicUsize::new(0);
static MAX_CUSTOM_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static SMASHLINE_BRIDGE_VERSION_FN: AtomicUsize = AtomicUsize::new(0);
static NATIVE_BACKEND_STATUS_FN: AtomicUsize = AtomicUsize::new(0);
static REGISTER_V1_FN: AtomicUsize = AtomicUsize::new(0);
static BASE_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static ENTRY_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static IDENTITY_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static CAPACITY_COMMITTED_FN: AtomicUsize = AtomicUsize::new(0);
static ARM_KIRBY_FAMILY_FN: AtomicUsize = AtomicUsize::new(0);
static CLONE_ARTICLE_FN: AtomicUsize = AtomicUsize::new(0);
static CLONE_ARTICLE_FOR_FN: AtomicUsize = AtomicUsize::new(0);
static ARTICLE_INDEX_FN: AtomicUsize = AtomicUsize::new(0);
static CLONE_COPY_ARTICLE_FN: AtomicUsize = AtomicUsize::new(0);
static COPY_ARTICLE_INDEX_FN: AtomicUsize = AtomicUsize::new(0);
static LOG_FN: AtomicUsize = AtomicUsize::new(0);
static PARAM_OVERRIDE_FN: AtomicUsize = AtomicUsize::new(0);
static PARAM_INT_OVERRIDE_FN: AtomicUsize = AtomicUsize::new(0);
static ARTICLE_OWNER_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static POCKET_HOLDER_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static SHARED_HOOK_FN: AtomicUsize = AtomicUsize::new(0);
static SHARED_HOOK_V2_FN: AtomicUsize = AtomicUsize::new(0);
static SHARED_HOOK_STATUS_FN: AtomicUsize = AtomicUsize::new(0);
static SHARED_HOOK_ORIGINAL_FN: AtomicUsize = AtomicUsize::new(0);
static REGISTER_ITEM_V1_FN: AtomicUsize = AtomicUsize::new(0);
static REGISTER_ITEM_FAMILY_V2_FN: AtomicUsize = AtomicUsize::new(0);
static REGISTER_ITEM_UI_V1_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_BASE_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static IS_ITEM_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_RESOURCE_NAME_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_KIND_FROM_OBJECT_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_KIND_FROM_BOMA_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_CATEGORY_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_FAMILY_OWNER_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_FAMILY_MEMBER_INDEX_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_SPAWN_SOURCE_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_PARENT_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_STATUS_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_STATUS_NAMED_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_STATUS_KIND_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_COMMON_SET_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_COMMON_HAS_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_BACKEND_STATUS_FN: AtomicUsize = AtomicUsize::new(0);
static STAGE_CAPACITY_FN: AtomicUsize = AtomicUsize::new(0);
static STAGE_ALLOCATE_FN: AtomicUsize = AtomicUsize::new(0);
static STAGE_BEHAVIOUR_FN: AtomicUsize = AtomicUsize::new(0);
static STAGE_ID_FN: AtomicUsize = AtomicUsize::new(0);
static STAGE_REGISTER_FN: AtomicUsize = AtomicUsize::new(0);

fn resolve(slot: &AtomicUsize, symbol: &'static [u8]) -> Option<usize> {
    let cached = slot.load(Ordering::Acquire);
    if cached != 0 {
        return Some(cached);
    }

    let mut address = 0usize;
    let result = unsafe { skyline::nn::ro::LookupSymbol(&mut address, symbol.as_ptr()) };
    if result != 0 || address == 0 {
        return None;
    }
    slot.store(address, Ordering::Release);
    Some(address)
}

pub fn api_version() -> Option<u32> {
    let address = resolve(&API_VERSION_FN, b"clone_engine_api_version\0")?;
    let function: ApiVersionFn = unsafe { std::mem::transmute(address) };
    Some(unsafe { function() })
}

pub fn compiled_capabilities() -> u64 {
    let Some(address) = resolve(
        &COMPILED_CAPABILITIES_FN,
        b"clone_engine_compiled_capabilities_v1\0",
    ) else {
        return 0;
    };
    let function: CapabilitiesFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub fn runtime_capabilities() -> u64 {
    let Some(address) = resolve(
        &RUNTIME_CAPABILITIES_FN,
        b"clone_engine_runtime_capabilities_v1\0",
    ) else {
        return 0;
    };
    let function: CapabilitiesFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub fn max_custom_kind() -> i32 {
    let Some(address) = resolve(&MAX_CUSTOM_KIND_FN, b"clone_engine_max_custom_kind\0") else {
        return MAX_PROVEN_CUSTOM_KIND;
    };
    let function: MaxCustomKindFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub fn smashline_bridge_version() -> u32 {
    let Some(address) = resolve(
        &SMASHLINE_BRIDGE_VERSION_FN,
        b"clone_engine_smashline_bridge_version\0",
    ) else {
        return 0;
    };
    let function: SmashlineBridgeVersionFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub fn smashline_compatible() -> bool {
    smashline_bridge_version() >= SMASHLINE_BRIDGE_VERSION_REQUIRED
}

pub fn native_backend_status() -> u32 {
    let Some(address) = resolve(
        &NATIVE_BACKEND_STATUS_FN,
        b"clone_engine_native_backend_status\0",
    ) else {
        return 0;
    };
    let function: NativeBackendStatusFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub fn item_backend_status() -> u32 {
    let Some(address) = resolve(
        &ITEM_BACKEND_STATUS_FN,
        b"clone_engine_item_backend_status\0",
    ) else {
        return 0;
    };
    let function: ItemBackendStatusFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub fn register_item(item: &ItemCloneRegistration<'_>) -> Result<(), Error> {
    let resource_name = CString::new(item.resource_name).map_err(|_| Error::InvalidName)?;
    let agent_name = CString::new(item.agent_name).map_err(|_| Error::InvalidName)?;
    let registration = CloneItemRegistrationV1 {
        api_version: API_VERSION_V1,
        struct_size: std::mem::size_of::<CloneItemRegistrationV1>() as u32,
        item_kind: item.item_kind,
        base_item_kind: item.base_item_kind,
        resource_name: resource_name.as_ptr(),
        agent_name: agent_name.as_ptr(),
        flags: 0,
        reserved_u32: 0,
        reserved: [0; 4],
    };
    let address = resolve(&REGISTER_ITEM_V1_FN, b"clone_engine_register_item_v1\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: RegisterItemV1Fn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(&registration) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn register_item_ui(item: &ItemUiRegistration<'_>) -> Result<(), Error> {
    let ui_id = CString::new(item.ui_id).map_err(|_| Error::InvalidName)?;
    let registration = CloneItemUiRegistrationV1 {
        api_version: API_VERSION_V1,
        struct_size: std::mem::size_of::<CloneItemUiRegistrationV1>() as u32,
        item_kind: item.item_kind,
        flags: item.flags,
        ui_id: ui_id.as_ptr(),
        training_order: item.training_order,
        rules_order: item.rules_order,
        reserved: [0; 4],
    };
    let address = resolve(
        &REGISTER_ITEM_UI_V1_FN,
        b"clone_engine_register_item_ui_v1\0",
    )
    .ok_or(Error::EngineUnavailable)?;
    let function: RegisterItemUiV1Fn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(&registration) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn register_item_family(family: &ItemFamilyRegistration<'_>) -> Result<(), Error> {
    if family.members.is_empty() {
        return Err(Error::Engine(ERROR_ITEM_FAMILY_EMPTY));
    }
    let member_count =
        u32::try_from(family.members.len()).map_err(|_| Error::Engine(ERROR_ITEM_FAMILY_LAYOUT))?;

    let names = family
        .members
        .iter()
        .map(|member| {
            Ok((
                CString::new(member.resource_name).map_err(|_| Error::InvalidName)?,
                CString::new(member.agent_name).map_err(|_| Error::InvalidName)?,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let members = family
        .members
        .iter()
        .zip(names.iter())
        .map(
            |(member, (resource_name, agent_name))| CloneItemFamilyMemberV2 {
                item_kind: member.item_kind,
                flags: 0,
                resource_name: resource_name.as_ptr(),
                agent_name: agent_name.as_ptr(),
                reserved: [0; 2],
            },
        )
        .collect::<Vec<_>>();
    let registration = CloneItemFamilyRegistrationV2 {
        api_version: API_VERSION_V2,
        struct_size: std::mem::size_of::<CloneItemFamilyRegistrationV2>() as u32,
        base_owner_kind: family.base_owner_kind,
        member_count,
        member_struct_size: std::mem::size_of::<CloneItemFamilyMemberV2>() as u32,
        flags: 0,
        members: members.as_ptr(),
        reserved: [0; 4],
    };

    let address = resolve(
        &REGISTER_ITEM_FAMILY_V2_FN,
        b"clone_engine_register_item_family_v2\0",
    )
    .ok_or(Error::EngineUnavailable)?;
    let function: RegisterItemFamilyV2Fn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(&registration) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn item_base_kind(item_kind: i32) -> i32 {
    let Some(address) = resolve(&ITEM_BASE_KIND_FN, b"clone_engine_item_base_kind\0") else {
        return item_kind;
    };
    let function: ItemBaseKindFn = unsafe { std::mem::transmute(address) };
    unsafe { function(item_kind) }
}

pub fn is_item_kind(item_kind: i32) -> bool {
    let Some(address) = resolve(&IS_ITEM_KIND_FN, b"clone_engine_is_item_kind\0") else {
        return false;
    };
    let function: IsItemKindFn = unsafe { std::mem::transmute(address) };
    unsafe { function(item_kind) }
}

pub fn item_resource_name(item_kind: i32) -> Option<String> {
    let address = resolve(&ITEM_RESOURCE_NAME_FN, b"clone_engine_item_resource_name\0")?;
    let function: ItemResourceNameFn = unsafe { std::mem::transmute(address) };
    let name = unsafe { function(item_kind) };
    if name.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned(),
    )
}

pub fn item_category(item_kind: i32) -> ItemCategory {
    let Some(address) = resolve(&ITEM_CATEGORY_FN, b"clone_engine_item_category\0") else {
        return ItemCategory::Unknown;
    };
    let function: ItemCategoryFn = unsafe { std::mem::transmute(address) };
    ItemCategory::from_raw(unsafe { function(item_kind) })
}

pub fn item_family_owner(item_kind: i32) -> Option<i32> {
    let address = resolve(&ITEM_FAMILY_OWNER_FN, b"clone_engine_item_family_owner\0")?;
    let function: ItemKindQueryFn = unsafe { std::mem::transmute(address) };
    let owner = unsafe { function(item_kind) };
    (owner >= 0).then_some(owner)
}

pub fn item_family_member_index(item_kind: i32) -> Option<u32> {
    let address = resolve(
        &ITEM_FAMILY_MEMBER_INDEX_FN,
        b"clone_engine_item_family_member_index\0",
    )?;
    let function: ItemKindQueryFn = unsafe { std::mem::transmute(address) };
    let index = unsafe { function(item_kind) };
    u32::try_from(index).ok()
}

pub unsafe fn item_kind_from_object(object: *const c_void) -> i32 {
    let Some(address) = resolve(
        &ITEM_KIND_FROM_OBJECT_FN,
        b"clone_engine_item_kind_from_object\0",
    ) else {
        return -1;
    };
    let function: ItemObjectKindFn = std::mem::transmute(address);
    function(object)
}

pub unsafe fn item_kind_from_boma(module_accessor: *const c_void) -> i32 {
    let Some(address) = resolve(
        &ITEM_KIND_FROM_BOMA_FN,
        b"clone_engine_item_kind_from_boma\0",
    ) else {
        return -1;
    };
    let function: ItemObjectKindFn = std::mem::transmute(address);
    function(module_accessor)
}

pub unsafe fn item_spawn_source(object: *const c_void) -> ItemSpawnSource {
    let Some(address) = resolve(&ITEM_SPAWN_SOURCE_FN, b"clone_engine_item_spawn_source\0") else {
        return ItemSpawnSource::Unknown;
    };
    let function: ItemSpawnSourceFn = std::mem::transmute(address);
    ItemSpawnSource::from_raw(function(object))
}

pub unsafe fn item_parent_kind(object: *const c_void) -> Option<i32> {
    let address = resolve(&ITEM_PARENT_KIND_FN, b"clone_engine_item_parent_kind\0")?;
    let function: ItemObjectKindFn = std::mem::transmute(address);
    let parent = function(object);
    (parent >= 0).then_some(parent)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ItemStatusLine {
    Setting = 0,
    JointSrt = 1,
    Init = 2,
    Update = 3,
    Coroutine = 4,
    Exit = 5,
}

pub fn item_status_named(
    item_kind: i32,
    line: ItemStatusLine,
    status_name: &str,
    function: *const (),
) -> Result<(), Error> {
    let name = CString::new(status_name).map_err(|_| Error::InvalidName)?;
    let address = resolve(
        &ITEM_STATUS_NAMED_FN,
        b"clone_engine_item_status_named_v1\0",
    )
    .ok_or(Error::EngineUnavailable)?;
    let function_ptr: ItemStatusNamedFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function_ptr(item_kind, line as i32, name.as_ptr(), function as usize) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn item_status(
    item_kind: i32,
    line: ItemStatusLine,
    status_kind: i32,
    function: *const (),
) -> Result<(), Error> {
    let address = resolve(&ITEM_STATUS_FN, b"clone_engine_item_status_v1\0")
        .ok_or(Error::EngineUnavailable)?;
    let function_ptr: ItemStatusFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function_ptr(item_kind, line as i32, status_kind, function as usize) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn item_status_kind(name: &str) -> Result<i32, Error> {
    let name = CString::new(name).map_err(|_| Error::InvalidName)?;
    let address = resolve(&ITEM_STATUS_KIND_FN, b"clone_engine_item_status_kind\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: ItemStatusKindFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(name.as_ptr()) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result)
}

pub fn item_common_set(item_kind: i32, field: u64, value: f32) -> Result<(), Error> {
    let address = resolve(&ITEM_COMMON_SET_FN, b"clone_engine_item_common_set\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: ItemCommonSetFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(item_kind, field, value) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn item_common_has(field: u64) -> bool {
    let Some(address) = resolve(&ITEM_COMMON_HAS_FN, b"clone_engine_item_common_has\0") else {
        return false;
    };
    let function: ItemCommonHasFn = unsafe { std::mem::transmute(address) };
    unsafe { function(field) != 0 }
}

pub fn stage_capacity() -> Result<StageCapacity, Error> {
    let address = resolve(&STAGE_CAPACITY_FN, b"clone_engine_stage_capacity\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: StageCapacityFn = unsafe { std::mem::transmute(address) };
    let mut places = 0u32;
    let mut stage_ids = 0u32;
    let result = unsafe { function(&mut places, &mut stage_ids) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(StageCapacity {
        places,
        stage_ids,
        can_mint: result != 0,
    })
}

pub fn allocate_stage(stage: &StageAllocation<'_>) -> Result<i32, Error> {
    let place = CString::new(stage.place_name).map_err(|_| Error::InvalidName)?;
    let resource = stage
        .resource_place
        .map(CString::new)
        .transpose()
        .map_err(|_| Error::InvalidName)?;
    let address = resolve(&STAGE_ALLOCATE_FN, b"clone_engine_allocate_stage\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: StageAllocateFn = unsafe { std::mem::transmute(address) };
    let result = unsafe {
        function(
            place.as_ptr(),
            resource
                .as_ref()
                .map_or(std::ptr::null(), |value| value.as_ptr()),
            stage.ships_battle_tree,
            stage.forms,
        )
    };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result)
}

pub fn set_stage_behaviour(place_name: &str, donor_place: &str) -> Result<(), Error> {
    let place = CString::new(place_name).map_err(|_| Error::InvalidName)?;
    let donor = CString::new(donor_place).map_err(|_| Error::InvalidName)?;
    let address = resolve(&STAGE_BEHAVIOUR_FN, b"clone_engine_set_stage_behaviour\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: StageBehaviourFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(place.as_ptr(), donor.as_ptr()) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn stage_id_for(place_name: &str, form: StageForm) -> Result<i32, Error> {
    let place = CString::new(place_name).map_err(|_| Error::InvalidName)?;
    let address =
        resolve(&STAGE_ID_FN, b"clone_engine_stage_id_for\0").ok_or(Error::EngineUnavailable)?;
    let function: StageIdFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(place.as_ptr(), form as u32) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result)
}

pub fn register_stage(stage: &StageRegistration<'_>) -> Result<(), Error> {
    let place = CString::new(stage.place_name).map_err(|_| Error::InvalidName)?;
    let name_id = CString::new(stage.name_id).map_err(|_| Error::InvalidName)?;
    let address = resolve(&STAGE_REGISTER_FN, b"clone_engine_register_stage\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: StageRegisterFn = unsafe { std::mem::transmute(address) };
    let result = unsafe {
        function(
            place.as_ptr(),
            name_id.as_ptr(),
            stage.ships_battle_tree,
            stage.ui_series_id,
            stage.display_order,
        )
    };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn register(clone: &CloneRegistration<'_>) -> Result<(), Error> {
    register_inner(clone).map(|_| ())
}

fn register_inner(clone: &CloneRegistration<'_>) -> Result<i32, Error> {
    let ui_chara = CString::new(clone.ui_chara).map_err(|_| Error::InvalidName)?;
    let fighter_kind_name =
        CString::new(clone.fighter_kind_name).map_err(|_| Error::InvalidName)?;
    let resource_name = CString::new(clone.resource_name).map_err(|_| Error::InvalidName)?;
    let base_resource_name =
        CString::new(clone.base_resource_name).map_err(|_| Error::InvalidName)?;

    let article_names = clone
        .articles
        .iter()
        .map(|article| CString::new(article.file_name).map_err(|_| Error::InvalidName))
        .collect::<Result<Vec<_>, _>>()?;
    let articles = clone
        .articles
        .iter()
        .zip(article_names.iter())
        .map(|(article, name)| CloneArticleRegistrationV1 {
            base_weapon_kind: article.base_weapon_kind,
            reserved: 0,
            file_name: name.as_ptr(),
        })
        .collect::<Vec<_>>();

    let registration = CloneRegistrationV1 {
        api_version: API_VERSION_V1,
        struct_size: std::mem::size_of::<CloneRegistrationV1>() as u32,
        custom_kind: clone.custom_kind,
        base_kind: clone.base_kind,
        ui_chara: ui_chara.as_ptr(),
        fighter_kind_name: fighter_kind_name.as_ptr(),
        resource_name: resource_name.as_ptr(),
        base_resource_name: base_resource_name.as_ptr(),
        color_start: clone.color_start,
        color_count: clone.color_count,
        copy_status_first: clone.copy_status_first,
        copy_status_count: clone.copy_status_count,
        article_namespace: clone.article_namespace,
        effect_namespace: clone.effect_namespace,
        articles: if articles.is_empty() {
            std::ptr::null()
        } else {
            articles.as_ptr()
        },
        article_count: articles.len() as u32,
        flags: clone.flags,
        reserved: [0; 4],
    };

    let address =
        resolve(&REGISTER_V1_FN, b"clone_engine_register_v1\0").ok_or(Error::EngineUnavailable)?;
    let function: RegisterV1Fn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(&registration) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    if result != RESULT_OK {
        return Ok(result);
    }
    if registration.custom_kind != KIND_AUTO {
        return Ok(registration.custom_kind);
    }
    kind_for_identity(clone.fighter_kind_name)
        .or_else(|| kind_for_identity(clone.resource_name))
        .ok_or(Error::Engine(ERROR_BACKEND_UNAVAILABLE))
}

pub fn allocate(clone: &CloneRegistration<'_>) -> Result<i32, Error> {
    let mut request = clone.clone();
    request.custom_kind = KIND_AUTO;
    register_inner(&request)
}

pub fn kind_for_identity(name: &str) -> Option<i32> {
    let name = CString::new(name).ok()?;
    let address = resolve(&IDENTITY_KIND_FN, b"clone_engine_kind_for_identity\0")?;
    let function: IdentityKindFn = unsafe { std::mem::transmute(address) };
    let kind = unsafe { function(name.as_ptr()) };
    (kind >= 0).then_some(kind)
}

pub struct CloneKind {
    cached: std::sync::atomic::AtomicI32,
    identity: &'static str,
}

impl CloneKind {
    pub const fn new(identity: &'static str) -> Self {
        Self {
            cached: std::sync::atomic::AtomicI32::new(-1),
            identity,
        }
    }

    pub fn store(&self, kind: i32) {
        if kind >= 0 {
            self.cached.store(kind, Ordering::Release);
        }
    }

    pub fn get(&self) -> Option<i32> {
        let cached = self.cached.load(Ordering::Acquire);
        if cached >= 0 {
            return Some(cached);
        }
        let resolved = kind_for_identity(self.identity)?;
        self.cached.store(resolved, Ordering::Release);
        Some(resolved)
    }

    pub fn raw(&self) -> i32 {
        self.get().unwrap_or(-1)
    }
}
pub fn capacity_committed() -> bool {
    let Some(address) = resolve(&CAPACITY_COMMITTED_FN, b"clone_engine_capacity_committed\0")
    else {
        return false;
    };
    let function: CapacityCommittedFn = unsafe { std::mem::transmute(address) };
    unsafe { function() != 0 }
}

pub fn base_kind(custom_kind: i32) -> i32 {
    let Some(address) = resolve(&BASE_KIND_FN, b"clone_engine_get_base_kind\0") else {
        return -1;
    };
    let function: KindQueryFn = unsafe { std::mem::transmute(address) };
    unsafe { function(custom_kind) }
}

pub fn entry_kind(entry_id: i32) -> i32 {
    let Some(address) = resolve(&ENTRY_KIND_FN, b"clone_engine_get_entry_kind\0") else {
        return -1;
    };
    let function: EntryKindFn = unsafe { std::mem::transmute(address) };
    unsafe { function(entry_id) }
}

pub fn article_owner_kind(module_accessor: u64) -> i32 {
    let Some(address) = resolve(
        &ARTICLE_OWNER_KIND_FN,
        b"clone_engine_article_owner_kind_v1\0",
    ) else {
        return -1;
    };
    let function: ArticleOwnerKindFn = unsafe { std::mem::transmute(address) };
    unsafe { function(module_accessor) }
}

#[repr(C)]
pub struct HookCall {
    pub args: [u64; 6],
    pub result: u64,
}

const _: [(); 56] = [(); std::mem::size_of::<HookCall>()];

pub type HookFn = unsafe extern "C" fn(*mut HookCall) -> u32;

pub const HOOK_DECLINED: u32 = 0;
pub const HOOK_HANDLED: u32 = 1;

#[deprecated(note = "use shared_hook_checked with an audited opcode fingerprint")]
pub unsafe fn shared_hook(offset: u64, callback: HookFn) -> bool {
    let Some(address) = resolve(&SHARED_HOOK_FN, b"clone_engine_shared_hook_v1\0") else {
        return false;
    };
    let function: SharedHookFn = std::mem::transmute(address);
    function(offset, callback as usize) == RESULT_OK
}

pub unsafe fn shared_hook_checked(registration: &SharedHookRegistrationV1) -> Result<(), Error> {
    let address = resolve(&SHARED_HOOK_V2_FN, b"clone_engine_shared_hook_v2\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: SharedHookV2Fn = std::mem::transmute(address);
    let result = function(registration);
    if result == RESULT_OK {
        Ok(())
    } else {
        Err(Error::Engine(result))
    }
}

pub fn shared_hook_status() -> u32 {
    let Some(address) = resolve(
        &SHARED_HOOK_STATUS_FN,
        b"clone_engine_shared_hook_status_v1\0",
    ) else {
        return 0;
    };
    let function: SharedHookStatusFn = unsafe { std::mem::transmute(address) };
    unsafe { function() }
}

pub unsafe fn shared_hook_original(offset: u64, args: &[u64; 6]) -> u64 {
    let Some(address) = resolve(
        &SHARED_HOOK_ORIGINAL_FN,
        b"clone_engine_shared_hook_original_v1\0",
    ) else {
        return 0;
    };
    let function: SharedHookOriginalFn = std::mem::transmute(address);
    function(offset, args as *const [u64; 6])
}

pub fn pocket_holder_kind(module_accessor: u64) -> i32 {
    let Some(address) = resolve(
        &POCKET_HOLDER_KIND_FN,
        b"clone_engine_pocket_holder_kind_v1\0",
    ) else {
        return -1;
    };
    let function: ArticleOwnerKindFn = unsafe { std::mem::transmute(address) };
    unsafe { function(module_accessor) }
}

pub fn clone_article(
    source_owner: &str,
    source_weapon_kind: i32,
    destination_owner: &str,
    name: &str,
) -> Result<i32, Error> {
    let address = resolve(&CLONE_ARTICLE_FN, b"clone_engine_clone_article_v1\0")
        .ok_or(Error::EngineUnavailable)?;
    let source_owner = CString::new(source_owner).map_err(|_| Error::InvalidName)?;
    let destination_owner = CString::new(destination_owner).map_err(|_| Error::InvalidName)?;
    let name = CString::new(name).map_err(|_| Error::InvalidName)?;

    let function: CloneArticleFn = unsafe { std::mem::transmute(address) };
    let result = unsafe {
        function(
            source_owner.as_ptr(),
            source_weapon_kind,
            destination_owner.as_ptr(),
            name.as_ptr(),
        )
    };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result)
}

pub fn clone_article_for(
    source_owner: &str,
    source_weapon_kind: i32,
    destination_owner: &str,
    resource_owner: &str,
    name: &str,
) -> Result<i32, Error> {
    let address = resolve(
        &CLONE_ARTICLE_FOR_FN,
        b"clone_engine_clone_article_for_v1\0",
    )
    .ok_or(Error::EngineUnavailable)?;
    let source_owner = CString::new(source_owner).map_err(|_| Error::InvalidName)?;
    let destination_owner = CString::new(destination_owner).map_err(|_| Error::InvalidName)?;
    let resource_owner = CString::new(resource_owner).map_err(|_| Error::InvalidName)?;
    let name = CString::new(name).map_err(|_| Error::InvalidName)?;

    let function: CloneArticleForFn = unsafe { std::mem::transmute(address) };
    let result = unsafe {
        function(
            source_owner.as_ptr(),
            source_weapon_kind,
            destination_owner.as_ptr(),
            resource_owner.as_ptr(),
            name.as_ptr(),
        )
    };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result)
}

pub fn clone_copy_article(
    target_kind: i32,
    source_owner: &str,
    source_weapon_kind: i32,
    resource_owner: &str,
    name: &str,
) -> Result<i32, Error> {
    let address = resolve(
        &CLONE_COPY_ARTICLE_FN,
        b"clone_engine_clone_copy_article_v1\0",
    )
    .ok_or(Error::EngineUnavailable)?;
    let source_owner = CString::new(source_owner).map_err(|_| Error::InvalidName)?;
    let resource_owner = CString::new(resource_owner).map_err(|_| Error::InvalidName)?;
    let name = CString::new(name).map_err(|_| Error::InvalidName)?;

    let function: CloneCopyArticleFn = unsafe { std::mem::transmute(address) };
    let result = unsafe {
        function(
            target_kind,
            source_owner.as_ptr(),
            source_weapon_kind,
            resource_owner.as_ptr(),
            name.as_ptr(),
        )
    };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result)
}

pub fn copy_article_index(target_kind: i32, weapon_kind: i32) -> Option<i32> {
    let address = resolve(
        &COPY_ARTICLE_INDEX_FN,
        b"clone_engine_copy_article_index_v1\0",
    )?;
    let function: CopyArticleIndexFn = unsafe { std::mem::transmute(address) };
    let index = unsafe { function(target_kind, weapon_kind) };
    (index >= 0).then_some(index)
}

pub fn article_index(fighter_kind: i32, weapon_kind: i32) -> Option<i32> {
    let address = resolve(&ARTICLE_INDEX_FN, b"clone_engine_article_index_v1\0")?;
    let function: ArticleIndexFn = unsafe { std::mem::transmute(address) };
    let index = unsafe { function(fighter_kind, weapon_kind) };
    (index >= 0).then_some(index)
}

pub fn log(message: &str) {
    let Some(address) = resolve(&LOG_FN, b"clone_engine_log_v1\0") else {
        return;
    };
    let function: LogFn = unsafe { std::mem::transmute(address) };
    unsafe { function(message.as_ptr(), message.len()) };
}

#[macro_export]
macro_rules! elog {
    ($($arg:tt)*) => {
        $crate::log(&format!($($arg)*))
    };
}

fn hash40(name: &str) -> u64 {
    const POLY: u32 = 0xEDB8_8320;
    let mut crc = 0xFFFF_FFFFu32;
    for byte in name.as_bytes() {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
        }
    }
    let crc = !crc;
    ((name.len() as u64) << 32) | u64::from(crc)
}

pub const ANY_SLOT: i32 = -1;

pub const PARAM_BEHAVIOR_ORIGINAL: i32 = 0;
pub const PARAM_BEHAVIOR_IGNORE: i32 = 1;
pub const PARAM_BEHAVIOR_DELETE: i32 = 2;
pub const PARAM_BEHAVIOR_MISFIRE: i32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ParamOp {
    Set = 0,
    Mul = 1,
}

pub fn param_override(kind: i32, param: &str, op: ParamOp, value: f64) -> bool {
    param_override_full(kind, ANY_SLOT, param, "", op, value)
}

pub fn param_override_slot(kind: i32, slot: i32, param: &str, op: ParamOp, value: f64) -> bool {
    param_override_full(kind, slot, param, "", op, value)
}

pub fn param_override_full(
    kind: i32,
    slot: i32,
    param: &str,
    subparam: &str,
    op: ParamOp,
    value: f64,
) -> bool {
    let Some(address) = resolve(&PARAM_OVERRIDE_FN, b"clone_engine_param_override_v1\0") else {
        return false;
    };
    let sub = if subparam.is_empty() {
        0
    } else {
        hash40(subparam)
    };
    let call: ParamOverrideFn = unsafe { std::mem::transmute(address) };
    unsafe { call(kind, slot, hash40(param), sub, op as u32, value) == 0 }
}

pub fn param_int_override(kind: i32, param: &str, value: i32) -> bool {
    param_int_override_full(kind, ANY_SLOT, param, "", value)
}

pub fn param_int_override_slot(kind: i32, slot: i32, param: &str, value: i32) -> bool {
    param_int_override_full(kind, slot, param, "", value)
}

pub fn param_int_override_full(
    kind: i32,
    slot: i32,
    param: &str,
    subparam: &str,
    value: i32,
) -> bool {
    let Some(address) = resolve(
        &PARAM_INT_OVERRIDE_FN,
        b"clone_engine_param_int_override_v1\0",
    ) else {
        return false;
    };
    let sub = if subparam.is_empty() {
        0
    } else {
        hash40(subparam)
    };
    let call: ParamIntOverrideFn = unsafe { std::mem::transmute(address) };
    unsafe { call(kind, slot, hash40(param), sub, value) == 0 }
}

fn param_int_override_raw(
    kind: i32,
    slot: i32,
    param_hash: u64,
    subparam_hash: u64,
    value: i32,
) -> bool {
    let Some(address) = resolve(
        &PARAM_INT_OVERRIDE_FN,
        b"clone_engine_param_int_override_v1\0",
    ) else {
        return false;
    };
    let call: ParamIntOverrideFn = unsafe { std::mem::transmute(address) };
    unsafe { call(kind, slot, param_hash, subparam_hash, value) == 0 }
}

fn param_weapon_subkey(weapon_kind: i32) -> Option<u64> {
    weapon_kind.checked_abs().map(|kind| kind as u64)
}

fn valid_param_behavior(behavior: i32) -> bool {
    (PARAM_BEHAVIOR_ORIGINAL..=PARAM_BEHAVIOR_MISFIRE).contains(&behavior)
}

pub fn param_article_use_type(weapon_kind: i32, use_type: i32) -> bool {
    let Some(weapon_kind) = weapon_kind.checked_abs().filter(|kind| *kind != 0) else {
        return false;
    };
    param_int_override_raw(-weapon_kind, 1, hash40("article_use_type"), 0, use_type)
}

pub fn param_disable_kirby_copy(kind: i32) -> bool {
    param_disable_kirby_copy_slot(kind, ANY_SLOT)
}

pub fn param_disable_kirby_copy_slot(kind: i32, slot: i32) -> bool {
    param_int_override_raw(kind, slot, hash40("kirby_cant_copy"), 0, 0)
}

pub fn param_kirby_inhale_behavior(kind: i32, weapon_kind: i32, behavior: i32) -> bool {
    param_kirby_inhale_behavior_slot(kind, ANY_SLOT, weapon_kind, behavior)
}

pub fn param_kirby_inhale_behavior_slot(
    kind: i32,
    slot: i32,
    weapon_kind: i32,
    behavior: i32,
) -> bool {
    let Some(weapon_kind) = param_weapon_subkey(weapon_kind) else {
        return false;
    };
    if !valid_param_behavior(behavior) {
        return false;
    }
    param_int_override_raw(
        kind,
        slot,
        hash40("kirby_inhale_behavior"),
        weapon_kind,
        behavior,
    )
}

pub fn param_villager_pocket_behavior(kind: i32, weapon_kind: i32, behavior: i32) -> bool {
    param_villager_pocket_behavior_slot(kind, ANY_SLOT, weapon_kind, behavior)
}

pub fn param_villager_pocket_behavior_slot(
    kind: i32,
    slot: i32,
    weapon_kind: i32,
    behavior: i32,
) -> bool {
    let Some(weapon_kind) = param_weapon_subkey(weapon_kind) else {
        return false;
    };
    if !valid_param_behavior(behavior) {
        return false;
    }
    param_int_override_raw(
        kind,
        slot,
        hash40("villager_pocket_behavior"),
        weapon_kind,
        behavior,
    )
}

pub fn param_disable_villager_pocket(kind: i32, weapon_kind: i32) -> bool {
    param_disable_villager_pocket_slot(kind, ANY_SLOT, weapon_kind)
}

pub fn param_disable_villager_pocket_slot(kind: i32, slot: i32, weapon_kind: i32) -> bool {
    param_villager_pocket_behavior_slot(kind, slot, weapon_kind, PARAM_BEHAVIOR_MISFIRE)
}

pub fn param_rosetta_pull_behavior(kind: i32, weapon_kind: i32, behavior: i32) -> bool {
    param_rosetta_pull_behavior_slot(kind, ANY_SLOT, weapon_kind, behavior)
}

pub fn param_rosetta_pull_behavior_slot(
    kind: i32,
    slot: i32,
    weapon_kind: i32,
    behavior: i32,
) -> bool {
    let Some(weapon_kind) = param_weapon_subkey(weapon_kind) else {
        return false;
    };
    if !valid_param_behavior(behavior) {
        return false;
    }
    param_int_override_raw(
        kind,
        slot,
        hash40("rosetta_pull_behavior"),
        weapon_kind,
        behavior,
    )
}

pub fn clone_article_handle(
    source_owner: &str,
    source_weapon_kind: i32,
    destination_owner: &str,
    name: &str,
    destination_fighter_kind: i32,
) -> Result<ArticleHandle, Error> {
    let weapon_kind = clone_article(source_owner, source_weapon_kind, destination_owner, name)?;
    Ok(ArticleHandle::new(destination_fighter_kind, weapon_kind))
}

pub fn clone_copy_article_handle(
    target_kind: i32,
    source_owner: &str,
    source_weapon_kind: i32,
    resource_owner: &str,
    name: &str,
) -> Result<ArticleHandle, Error> {
    let weapon_kind = clone_copy_article(
        target_kind,
        source_owner,
        source_weapon_kind,
        resource_owner,
        name,
    )?;
    Ok(ArticleHandle::kirby_copy(target_kind, weapon_kind))
}

pub struct ArticleHandle {
    fighter_kind: i32,
    weapon_kind: i32,
    kirby_copy: bool,
}

impl ArticleHandle {
    pub const fn new(fighter_kind: i32, weapon_kind: i32) -> Self {
        Self {
            fighter_kind,
            weapon_kind,
            kirby_copy: false,
        }
    }

    pub const fn kirby_copy(target_kind: i32, weapon_kind: i32) -> Self {
        Self {
            fighter_kind: target_kind,
            weapon_kind,
            kirby_copy: true,
        }
    }

    pub fn weapon_kind(&self) -> i32 {
        self.weapon_kind
    }

    pub fn index(&self) -> Option<i32> {
        if self.kirby_copy {
            copy_article_index(self.fighter_kind, self.weapon_kind)
        } else {
            article_index(self.fighter_kind, self.weapon_kind)
        }
    }
}

pub fn arm_kirby_copy_status_family(kind: i32, first_status: i32, count: i32) -> bool {
    let Some(address) = resolve(
        &ARM_KIRBY_FAMILY_FN,
        b"clone_engine_arm_kirby_copy_status_family\0",
    ) else {
        return false;
    };
    let function: ArmKirbyFamilyFn = unsafe { std::mem::transmute(address) };
    unsafe { function(kind, first_status, count) != 0 }
}

#[cfg(feature = "smash")]
pub unsafe fn true_kind<T>(module_accessor: *mut T) -> i32 {
    if module_accessor.is_null() {
        return -1;
    }
    let module_accessor = module_accessor.cast::<smash::app::BattleObjectModuleAccessor>();
    let entry = smash::app::lua_bind::WorkModule::get_int(
        module_accessor,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    let custom = entry_kind(entry);
    if custom >= FIRST_CUSTOM_KIND {
        custom
    } else {
        smash::app::utility::get_kind(&mut *module_accessor)
    }
}

#[cfg(feature = "smash")]
pub unsafe fn is_kind<T>(module_accessor: *mut T, expected_kind: i32) -> bool {
    true_kind(module_accessor) == expected_kind
}

#[cfg(feature = "smash")]
pub unsafe fn owner_true_kind<T>(module_accessor: *mut T) -> i32 {
    if module_accessor.is_null() {
        return -1;
    }
    let module_accessor = module_accessor.cast::<smash::app::BattleObjectModuleAccessor>();

    let category = smash::app::utility::get_category(&mut *module_accessor);
    if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER {
        return true_kind(module_accessor);
    }
    if category != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_WEAPON {
        return -1;
    }

    let engine_answer = article_owner_kind(module_accessor as u64);
    if engine_answer >= FIRST_CUSTOM_KIND {
        return engine_answer;
    }

    let owner_id = smash::app::lua_bind::WorkModule::get_int(
        module_accessor,
        *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER,
    ) as u32;
    if owner_id == *smash::lib::lua_const::BATTLE_OBJECT_ID_INVALID as u32
        || !smash::app::sv_battle_object::is_active(owner_id)
    {
        return -1;
    }

    let owner = smash::app::sv_battle_object::module_accessor(owner_id);
    true_kind(owner)
}

#[cfg(feature = "smash")]
pub unsafe fn is_owned_by_kind<T>(module_accessor: *mut T, expected_kind: i32) -> bool {
    owner_true_kind(module_accessor) == expected_kind
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ArticleStatusLine {
    Pre = 0,
    Main = 1,
    End = 2,
    Init = 3,
    Exec = 4,
    ExecStop = 5,
    Post = 6,
    Exit = 7,
    MapCorrection = 8,
    FixCamera = 9,
    FixPosSlow = 10,
    CheckDamage = 11,
    CheckAttack = 12,
    OnChangeLr = 13,
    LeaveStop = 14,
    NotifyEventGimmick = 15,
    CalcParam = 16,
}

type ArticleStatusFn = unsafe extern "C" fn(i32, i32, i32, *const ()) -> i32;
static ARTICLE_STATUS_FN: AtomicUsize = AtomicUsize::new(0);

pub fn article_status(
    weapon_kind: i32,
    line: ArticleStatusLine,
    status_kind: i32,
    function: *const (),
) -> Result<(), Error> {
    let address = resolve(&ARTICLE_STATUS_FN, b"clone_engine_article_status_v1\0")
        .ok_or(Error::EngineUnavailable)?;
    let function_ptr: ArticleStatusFn = unsafe { core::mem::transmute(address) };
    let result = unsafe { function_ptr(weapon_kind, line as i32, status_kind, function) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}
