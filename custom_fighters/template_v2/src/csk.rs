//! The character select entry, published with the CSK Collection's own API.
//! Two fields name the clone instead of the base: `fighter_kind` and
//! `fighter_kind_corps` are `fighter_kind_<name>`, `ui_chara_id` is
//! `ui_chara_<name>`. Everything else is what a one-slot mod writes.

use std::collections::HashMap;

use crate::smash;
use smash::hash40;
use the_csk_collection_api::*;

pub fn publish() {
    let ui_chara = hash40("ui_chara_template_fighter"); // the clone's UI id: ui_chara_<name>
    let base_ui_chara = hash40(&format!("ui_chara_{}", crate::BASE)); // the base's UI id

    let mut indices = HashMap::new();
    let mut hashes = HashMap::new();
    for color in 0..crate::COSTUMES {
        indices.insert(hash40(&format!("c{color:02}_index")), UnsignedByteType::Overwrite(color)); // costume index
        indices.insert(hash40(&format!("n{color:02}_index")), UnsignedByteType::Overwrite(color)); // name index
        indices.insert(hash40(&format!("c{color:02}_group")), UnsignedByteType::Overwrite(0)); // costume group
        hashes.insert(
            hash40(&format!("characall_label_c{color:02}")), // announcer call for this costume
            Hash40Type::Overwrite(hash40(&format!("vc_narration_characall_{}", crate::BASE))),
        );
        hashes.insert(hash40(&format!("characall_label_article_c{color:02}")), Hash40Type::Overwrite(0));
    }
    indices.insert(hash40("color_start_index"), UnsignedByteType::Overwrite(0)); // first costume
    hashes.insert(hash40("original_ui_chara_hash"), Hash40Type::Overwrite(base_ui_chara));

    allow_ui_chara_hash_online(ui_chara); // let the entry be picked online
    add_chara_db_entry_info(CharacterDatabaseEntry {
        ui_chara_id: ui_chara,                            // the clone's UI id
        clone_from_ui_chara_id: Some(base_ui_chara),      // copy every other field from the base's entry
        name_id: StringType::Overwrite(CStrCSK::new("template_fighter")), // suffix of the nam_chr*_00_<x> labels in msg_name.xmsbt
        fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_template_fighter")), // the clone's identity
        fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_template_fighter")), // same
        ui_series_id: Hash40Type::Overwrite(hash40("ui_series_mario")), // series icon
        disp_order: SignedByteType::Optional(Some(2)),    // position on the select screen
        color_num: UnsignedByteType::Overwrite(crate::COSTUMES), // number of costumes
        extra_index_maps: UnsignedByteMap::Overwrite(indices),
        extra_hash_maps: Hash40Map::Overwrite(hashes),
        ..Default::default()
    });

    for color in 0..crate::COSTUMES {
        add_chara_layout_db_entry_info(CharacterLayoutDatabaseEntry {
            ui_layout_id: hash40(&format!("ui_chara_template_fighter_{color:02}")), // layout id: ui_chara_<name>_<costume>
            clone_from_ui_layout_id: Some(hash40(&format!("ui_chara_{}_{:02}", crate::BASE, color % 8))), // the base has 8 layouts
            ui_chara_id: Hash40Type::Overwrite(ui_chara), // the clone's UI id
            chara_color: UnsignedByteType::Overwrite(color), // the costume
            ..Default::default()
        });
    }

    clone_engine_api::elog!("[template_v2] CSK entry published for {} costumes", crate::COSTUMES);
}
