//! An item family: a boss cloned together with the items it creates. Galleom
//! is a family of three (the boss, its missile and its shockwave); a clone
//! names all three, in the base's order, and each member loads its own files
//! from item/<resource name>/ (without them it plays with the base's).
//!
//! Assist Trophies and Poke Ball Pokemon are families too: pass the base
//! family's first kind and one member per member of that family.

use clone_engine_api::{
    item_category, item_family_member_index, item_family_owner, item_kind_for_identity,
    register_item_family, ItemFamilyMember, ItemFamilyRegistration, KIND_AUTO,
};
use smash::lib::lua_const::*;

/// The members, in the base family's order: the boss first.
pub const MEMBERS: [&str; 3] = [
    "template_boss",           // the boss itself
    "template_boss_missile",   // what Galleom's missile becomes
    "template_boss_shockwave", // what Galleom's shockwave becomes
];

/// Registers the family. Every kind is KIND_AUTO: the engine hands them out
/// and keeps them the same from one boot to the next.
pub fn register() {
    let members = MEMBERS.map(|name| {
        ItemFamilyMember::new(
            KIND_AUTO, // item kind: the engine picks a free one
            name,      // resource name: files under item/<name>/
            name,      // script agent name: the agent its scripts install on
        )
    });
    let family = ItemFamilyRegistration::new(
        *ITEM_KIND_GALLEOM, // the base family's first member
        &members,           // one entry per member of the base family
    );
    if let Err(error) = register_item_family(&family) {
        skyline::println!("[template_item_v2] family refused: {error:?}");
        return;
    }
    for name in MEMBERS {
        // the kind the engine picked, read back by resource name
        let Some(kind) = item_kind_for_identity(name) else {
            continue;
        };
        skyline::println!(
            "[template_item_v2] {name} = kind {kind:#x}, {:?}, owner {:?}, member {:?}",
            item_category(kind),            // Boss here; Assist or Pokemon for those families
            item_family_owner(kind),        // the boss's kind, for every member
            item_family_member_index(kind), // 0 for the boss, then 1, 2
        );
    }
}
