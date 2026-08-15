use core::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

use crate::smash;
use smash::hash40;
use the_csk_collection_api::*;

static REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn install() {
    if REGISTERED.swap(true, Ordering::AcqRel) {
        return;
    }

    let mut indices = HashMap::new();
    let mut hashes = HashMap::new();
    for color in crate::COLOR_START..crate::COLOR_START + crate::COLOR_COUNT {
        indices.insert(
            hash40(&format!("c{color:02}_index")),
            UnsignedByteType::Overwrite(color as u8),
        );
        indices.insert(
            hash40(&format!("n{color:02}_index")),
            UnsignedByteType::Overwrite(color as u8),
        );
        indices.insert(
            hash40(&format!("c{color:02}_group")),
            UnsignedByteType::Overwrite(0),
        );
        hashes.insert(
            hash40(&format!("characall_label_c{color:02}")),
            Hash40Type::Overwrite(hash40("vc_narration_characall_mario")),
        );
        hashes.insert(
            hash40(&format!("characall_label_article_c{color:02}")),
            Hash40Type::Overwrite(0),
        );
    }
    indices.insert(
        hash40("color_start_index"),
        UnsignedByteType::Overwrite(crate::COLOR_START as u8),
    );
    hashes.insert(
        hash40("original_ui_chara_hash"),
        Hash40Type::Overwrite(hash40("ui_chara_mario")),
    );

    let ui_chara = hash40(crate::UI_CHARA_NAME);
    allow_ui_chara_hash_online(ui_chara);
    add_chara_db_entry_info(CharacterDatabaseEntry {
        ui_chara_id: ui_chara,
        clone_from_ui_chara_id: Some(hash40("ui_chara_mario")),
        name_id: StringType::Overwrite(CStrCSK::new("template_fighter")),
        fighter_kind: Hash40Type::Overwrite(hash40(crate::FIGHTER_KIND_NAME)),
        fighter_kind_corps: Hash40Type::Overwrite(hash40(crate::FIGHTER_KIND_NAME)),
        ui_series_id: Hash40Type::Overwrite(hash40("ui_series_mario")),
        fighter_type: Hash40Type::Overwrite(hash40("fighter_type_normal")),
        alt_chara_id: Hash40Type::Overwrite(hash40("-1")),
        shop_item_tag: Hash40Type::Overwrite(hash40("-1")),
        exhibit_year: ShortType::Overwrite(1981),
        disp_order: SignedByteType::Optional(Some(2)),
        save_no: SignedByteType::Overwrite(0),
        chara_count: SignedByteType::Overwrite(1),
        can_select: BoolType::Overwrite(true),
        is_usable_soundtest: BoolType::Overwrite(true),
        is_called_pokemon: BoolType::Overwrite(false),
        is_mii: BoolType::Overwrite(false),
        is_boss: BoolType::Overwrite(false),
        is_hidden_boss: BoolType::Overwrite(false),
        is_dlc: BoolType::Overwrite(false),
        is_patch: BoolType::Overwrite(false),
        is_plural_message: BoolType::Overwrite(false),
        is_plural_narration: BoolType::Overwrite(false),
        is_article: BoolType::Overwrite(false),
        has_multiple_face: BoolType::Overwrite(false),
        result_pf0: BoolType::Overwrite(true),
        result_pf1: BoolType::Overwrite(true),
        result_pf2: BoolType::Overwrite(true),
        color_num: UnsignedByteType::Overwrite(crate::COLOR_COUNT as u8),
        extra_index_maps: UnsignedByteMap::Overwrite(indices),
        extra_hash_maps: Hash40Map::Overwrite(hashes),
        ..Default::default()
    });

    add_chara_layout_db_entry_info(CharacterLayoutDatabaseEntry {
        ui_layout_id: hash40("ui_chara_template_fighter_00"),
        clone_from_ui_layout_id: Some(hash40("ui_chara_mario_00")),
        ui_chara_id: Hash40Type::Overwrite(ui_chara),
        chara_color: UnsignedByteType::Overwrite(0),
        ..Default::default()
    });

    clone_engine_api::elog!(
        "[template] CSK row published: {} -> {}",
        crate::UI_CHARA_NAME,
        crate::FIGHTER_KIND_NAME
    );
}
