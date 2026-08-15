pub(crate) const OFF_STATUS: usize = 0x64bbd0;
pub(crate) const OFF_GAME: usize = 0x64c310;
pub(crate) const OFF_EFFECT: usize = 0x64c930;
pub(crate) const OFF_EXPRESSION: usize = 0x64cf50;
pub(crate) const OFF_SOUND: usize = 0x64d570;

pub(crate) const OFF_GAME_SHARE: usize = 0x64db90;
pub(crate) const OFF_EFFECT_SHARE: usize = 0x64e2a0;
pub(crate) const OFF_EXPRESSION_SHARE: usize = 0x64e8b0;
pub(crate) const OFF_SOUND_SHARE: usize = 0x64eec0;

pub(crate) const OFF_AI_PROFILE_KIND_BOUND: usize = 0x2c9934;
pub(crate) const OFF_AI_AGENT_KIND_BOUND: usize = 0x2c9b68;

pub(crate) const OFF_AI_ATTACK_LIST_KIND_BOUND: usize = 0x33ba50;
pub(crate) const OFF_AI_ATTACK_DATA_KIND_BOUND: usize = 0x33c510;
pub(crate) const OFF_AI_PARAM_FLOAT_KIND_BOUND: usize = 0x362e00;
pub(crate) const OFF_AI_PARAM_INT_KIND_BOUND: usize = 0x36c1cc;

pub(crate) const OFF_KIRBY_COPY_STATIC_SETUP_TABLE: usize = 0xba14ec;
pub(crate) const OFF_KIRBY_COPY_ARTICLE_HEADER_ASSIGN: usize = 0xba14fc;
pub(crate) const OFF_KIRBY_COPY_CALLBACK_KIND_1: usize = 0xba3e0c;
pub(crate) const OFF_KIRBY_COPY_CALLBACK_KIND_2: usize = 0xba400c;
pub(crate) const OFF_KIRBY_COPY_CALLBACK_KIND_3: usize = 0xba405c;
pub(crate) const OFF_KIRBY_COPY_RESOURCE_TRANSFER: usize = 0x6de830;
pub(crate) const OFF_KIRBY_COPY_RESOURCE_KIND: usize = 0xba4424;
pub(crate) const OFF_KIRBY_COPY_MODEL_NAME_TABLE: usize = 0xba4488;
pub(crate) const OFF_KIRBY_COPY_MODEL_NAME: usize = 0xba448c;
pub(crate) const OFF_KIRBY_COPY_HANDLE_PROBE: usize = 0xba44b0;
pub(crate) const OFF_KIRBY_COPY_MODEL_CHANGER_ENTRY: usize = 0xba4090;
pub(crate) const OFF_KIRBY_COPY_VISUAL_KIND_PROMOTE: usize = 0xba4190;
pub(crate) const OFF_KIRBY_COPY_MODEL_REMOVE_RESULT: usize = 0xba41f4;
pub(crate) const OFF_KIRBY_COPY_FULL_MODEL_GATE: usize = 0xba4198;
pub(crate) const OFF_KIRBY_COPY_FULL_MODEL_REMOVE_GATE: usize = 0xba4210;
pub(crate) const OFF_KIRBY_COPY_SPECIAL_INSTALL: usize = 0xba4254;
pub(crate) const OFF_KIRBY_COPY_MODEL_BASE_KIND: usize = 0xba4514;
pub(crate) const OFF_KIRBY_COPY_BASE_MODEL_PAIR: usize = 0xba4524;
pub(crate) const OFF_KIRBY_COPY_KIND_LIST_TABLE: usize = 0xba4d7c;
pub(crate) const OFF_KIRBY_COPY_SECOND_NAME_MERGE: usize = 0xba4e9c;
pub(crate) const OFF_KIRBY_COPY_BODYMOTION_HANDLE: usize = 0xba4ec4;
pub(crate) const OFF_KIRBY_COPY_ROW_SEARCH: usize = 0xba51d4;
pub(crate) const OFF_KIRBY_COPY_HAT_RECORD_REGION: usize = 0xba53d0;
pub(crate) const OFF_KIRBY_COPY_REMOVAL_HASHLIST_A: usize = 0xba492c;
pub(crate) const OFF_KIRBY_COPY_REMOVAL_HASHLIST_B: usize = 0xba4e20;
pub(crate) const KIRBY_COPY_KIND_HASHLIST_TABLE: usize = 0x4fcd388;
pub(crate) const OFF_KIRBY_COPY_CALLBACK_KIND_4: usize = 0xba5430;
pub(crate) const OFF_KIRBY_COPY_RECORD_CREATOR: usize = 0x17f0bd0;
pub(crate) const OFF_KIRBY_COPY_RECORD_NAME: usize = 0x17f0d30;
pub(crate) const OFF_KIRBY_COPY_RECORD_BODY_NAME: usize = 0x17f0f54;
pub(crate) const OFF_KIRBY_COPY_RECORD_SOUND_NAME: usize = 0x17f1158;
pub(crate) const OFF_KIRBY_COPY_DIR_REGISTRAR: usize = 0x17effe0;
pub(crate) const OFF_KIRBY_COPY_DIR_REGISTRAR_PARENT: usize = 0x17efb80;
pub(crate) const OFF_KIRBY_COPY_DIR_NAME_MERGE: usize = 0x17f0058;

pub(crate) const KIRBY_COPY_NAME_TABLE: usize = 0x509ece0;
pub(crate) const KIRBY_COPY_NAME_COUNT: i32 = 94;
pub(crate) const OFF_KIRBY_COPY_RECORD_LOOKUP_KIND: usize = 0x341b164;
pub(crate) const OFF_KIRBY_COPY_RESOURCE_SLOT_0: usize = 0xba1884;
pub(crate) const OFF_KIRBY_COPY_RESOURCE_SLOT_1: usize = 0xba18c8;
pub(crate) const OFF_KIRBY_COPY_RESOURCE_SLOT_2: usize = 0xba1908;

pub(crate) const OFF_KIRBY_COPY_MEMBER_BUILDER: usize = 0x17f06e0;
pub(crate) const OFF_KIRBY_COPY_MEMBER_BUILDER_2: usize = 0x17f0890;
pub(crate) const OFF_KIRBY_COPY_TRANSFER_LOOKUP: usize = 0x6de954;
pub(crate) const OFF_RESOURCE_MANAGER_GLOBAL: usize = 0x5323680;
pub(crate) const OFF_STD_MUTEX_LOCK: usize = 0x39c1410;
pub(crate) const OFF_STD_MUTEX_UNLOCK: usize = 0x39c1420;
pub(crate) const OFF_STD_RECURSIVE_MUTEX_LOCK: usize = 0x39c1490;
pub(crate) const OFF_STD_RECURSIVE_MUTEX_UNLOCK: usize = 0x39c14a0;
pub(crate) const KIRBY_RECORD_SLOT_COUNT: usize = 20;
pub(crate) const KIRBY_RECORD_TABLE_OFFSET: usize = 0x98;
pub(crate) const KIRBY_RECORD_SLOT_STRIDE: usize = 0x1748;
pub(crate) const KIRBY_RECORD_COLOR_STRIDE: usize = 0x2e8;
pub(crate) const KIRBY_RECORD_MEMBER1_OFFSET: usize = 0x20;
pub(crate) const KIRBY_RECORD_MODEL_TYPE: u32 = 0x3f;

pub(crate) const KIRBY_FULL_MODEL_BRANCH_KIND: i32 = 0x14;

pub(crate) const OFF_KIRBY_COPY_HAT_SYNC: usize = 0xb9a160;

pub(crate) const OFF_KIRBY_COPY_SETUP: usize = 0xba0e80;
pub(crate) const OFF_KIRBY_COPY_SETUP_SHIM: usize = 0x2149830;
pub(crate) const OFF_SV_BATTLE_OBJECT_KIND: usize = 0x2283540;
pub(crate) const OFF_KIRBY_GET_COPY_KIND: usize = 0xb9d870;
pub(crate) const OFF_KIRBY_GET_COPY_SLOT_NO: usize = 0xba29a0;
pub(crate) const OFF_KIRBY_COPY_ABILITY_RESET: usize = 0xb96770;
pub(crate) const OFF_KIRBY_PER_FIGHTER_FRAME: usize = 0xb97b30;
pub(crate) const OFF_COPY_SETUP_GATE1: usize = 0xba14dc;
pub(crate) const OFF_COPY_SETUP_SLOT: usize = 0xba1864;
pub(crate) const OFF_COPY_SETUP_DISPATCH: usize = 0xba19f4;
pub(crate) const OFF_COPY_SETUP_GRANT: usize = 0xba1e64;
pub(crate) const OFF_COPY_CHARA_IMPL_1: usize = 0x6e26ec;
pub(crate) const OFF_COPY_CHARA_IMPL_2: usize = 0x6e291c;
pub(crate) const OFF_COPY_CHARA_IMPL_3: usize = 0x6e2a4c;
pub(crate) const OFF_COPY_MOTION_BIND_TAIL_KIND: usize = 0x6e2770;
pub(crate) const OFF_COPY_CHARA_THUNK_1: usize = 0x20aa084;
pub(crate) const OFF_COPY_CHARA_THUNK_2: usize = 0x20aa1c4;
pub(crate) const OFF_COPY_CHARA_THUNK_3: usize = 0x20aa25c;
pub(crate) const OFF_COPY_CHARA_THUNK_4: usize = 0x20aa31c;
pub(crate) const OFF_COPY_CHARA_THUNK_5: usize = 0x20aaedc;
pub(crate) const OFF_COPY_CHARA_THUNK_6: usize = 0x20ac26c;
pub(crate) const OFF_COPY_CHARA_THUNK_7: usize = 0x20ac700;
pub(crate) const OFF_COPY_CHARA_THUNK_8: usize = 0x20acbe0;
pub(crate) const OFF_WEAPON_PRELOAD: usize = 0x17eeae0;
pub(crate) const OFF_WEAPON_LOOP_CONT_A: usize = 0x607e44;
pub(crate) const OFF_WEAPON_LOOP_CONT_B: usize = 0x607e74;
pub(crate) const OFF_WEAPON_LOOP_SLOT_ARGS: usize = 0x607f28;
pub(crate) const OFF_WEAPON_LOOP_RESOLVED: usize = 0x607f98;
pub(crate) const OFF_RESOURCE_SLOT: usize = 0x17f1aa0;

pub(crate) const OFF_FIGHTER_CLASS_RESOLVER: usize = 0x68d530;

pub(crate) const FIGHTER_CLASS_TABLE: usize = 0x529bfd0;

pub(crate) const OFF_STATIC_FIGHTER_DATA: usize = 0x64b730;

pub(crate) const OFF_FIGHTER_AUX_DATA_INIT: usize = 0x34af10;

#[cfg(feature = "clone_runtime")]
pub(crate) const OFF_MODEL_PATH_RESOLVE: usize = 0x17e9a00;
#[cfg(feature = "clone_runtime")]
pub(crate) const OFF_FIGHTER_RESOURCE_PATH_RESOLVE: usize = 0x17e88d0;

pub(crate) const RESOURCE_INDEX_NOT_FOUND: i32 = 0xff_ffff;
pub(crate) const OFF_FIGHTER_BOUNDARY_PARAMS: usize = 0x6797b0;

#[cfg(any(feature = "diag_article", feature = "css_slot"))]
pub(crate) const OFF_GENERATE_ARTICLE_IMPL: usize = 0x2092ab0;

#[cfg(feature = "css_slot")]
pub(crate) const OFF_GENERATE_ARTICLE_ENABLE_IMPL: usize = 0x2092ad0;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_SHOOT_ARTICLE_IMPL: usize = 0x2092b20;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_SHOOT_EXIST_ARTICLE_IMPL: usize = 0x2092b40;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_REMOVE_ARTICLE_IMPL: usize = 0x2092d90;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_REMOVE_EXIST_ARTICLE_IMPL: usize = 0x2092da0;

#[cfg(feature = "css_slot")]
pub(crate) const OFF_ARTICLE_CREATOR_DISPATCH: usize = 0x3d40c0;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_ARTICLE_CUSTOM_CREATOR: usize = 0x3a5d80;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_ARTICLE_BASE_CREATOR: usize = 0x3a6ce0;

#[cfg(feature = "css_slot")]
pub(crate) const OFF_SV_BATTLE_OBJECT_MODULE_ACCESSOR: usize = 0x2283700;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_SV_BATTLE_OBJECT_IS_ACTIVE: usize = 0x22839d0;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_SV_BATTLE_OBJECT_KIND_FOR_ARTICLE: usize = 0x2283540;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_SV_BATTLE_OBJECT_CATEGORY: usize = 0x22835d0;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_STATUS_KIND_IMPL: usize = 0x2087720;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_MOTION_KIND_IMPL: usize = 0x205cc20;
#[cfg(feature = "css_slot")]
pub(crate) const OFF_MOTION_FRAME_IMPL: usize = 0x205cc70;

#[cfg(any(feature = "diag_article", feature = "css_slot"))]
pub(crate) const ARTICLE_MODULE_OFF: usize = 0x98;
