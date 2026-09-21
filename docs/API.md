# Clone Engine API

Reference for packs that ship code. File-only packs use `fighter.toml`,
`item.toml` and `stage.toml`; see the [wiki](wiki/GETTING_STARTED.md). The
pre-`fighter.toml` calls are in [DEPRECATED.md](DEPRECATED.md).

Templates: [fighter](../custom_fighters/template_v2/),
[item](../custom_items/template_v2/).

```toml
[dependencies]
clone_engine_api = { git = "https://github.com/PropaSmike/ssbu_clone_engine", tag = "0.2.1-beta.1" }
```

Use the tag of the engine release you installed. Without the engine every
call returns `Error::EngineUnavailable`.

## Fighters

### Registration by code

```rust
use clone_engine_api::v2::{fighter, hook, slot, Line, Manifest};
use smash::hash40;
use smashline::{Agent, Main, Priority};

// WAWA: the handle the plugin uses everywhere. "wawa": the identity, the same
// string as Manifest::new's name.
fighter!(WAWA, "wawa");

const COSTUMES: u8 = 8; // how many costumes the pack ships (c00..), used everywhere below

#[skyline::main(name = "wawa")]
pub fn main() {
    let manifest = Manifest::new(
        "wawa",  // name: folder under fighter/, Smashline agent name, identity
        "samus", // base: the vanilla fighter it is built on
    )
    .costumes(COSTUMES as u32)      // the costume count
    .own_css()                      // the plugin publishes the select entry itself (publish_css below)
    .article("beam", "samus/cshot") // article "beam" (fighter/wawa/model/beam), copied from Samus's "cshot"
    .kirby(2);                      // reserve two status numbers for Kirby's copy
    let Ok(kind) = WAWA.register(manifest) else {
        return; // the fault is in the log
    };

    publish_css();

    let slots: Vec<i32> = (0..COSTUMES as i32).collect(); // costumes the values apply to; vec![-1] is all of them
    param_config::update_float(
        kind,                                              // the clone's kind, from register
        slots.clone(),                                     // costumes
        (hash40("param_special_hi"), hash40("y_spd_air")), // (param table, field)
        1.5,                                               // new value
    );
    param_config::update_int(kind, slots.clone(), (hash40("jump_squat_frame"), 0), 4); // top-level field: second hash is 0
    WAWA.param("weight").mul(1.02); // fighter_param attribute: through the engine, see Parameters

    Agent::new("wawa")                                              // scripts under the clone's own name
        .game_acmd("game_attack11", game_attack11, Priority::Default) // script name, your function, priority
        .on_line(Main, wawa_frame)                                  // once per frame
        .install();
    WAWA.article("beam")
        .status(Line::Main, *WEAPON_SAMUS_CSHOT_STATUS_KIND_FLY, beam_main) // status line, a status kind of the source weapon, your function
        .install();
    Agent::new("kirby")
        .status(Main, WAWA.kirby_status(0), kirby_special_n) // kirby_status(0): the first reserved number
        .install();
    WAWA.arm_kirby(); // after the Kirby statuses are installed
    clone_engine_api::install_hooks!(on_link_event); // every #[hook] in the crate
}

// slot: which vtable entry. class: the class object the game passes first;
// object: the fighter; event: the entry's own argument.
#[hook(slot = slot::fighter::ON_LINK_EVENT)]
unsafe fn on_link_event(class: u64, object: *mut smash::app::BattleObject, event: u64) {
    call_original!(class, object, event) // the base's entry
}
```

`register` goes first; everything else needs the kind. It returns the kind
or `Error::Engine(ERROR_MANIFEST)`; the log names the fault.

| `Manifest` method | Meaning |
|---|---|
| `new(name, base)` | `name`: folder under `fighter/`, Smashline name, identity. `base`: vanilla fighter name. |
| `costumes(n)`, `color_start(n)` | Costume range. Default 8 from 0. |
| `name_label(s)`, `series(s)`, `disp_order(n)`, `save_no(n)`, `exhibit_year(n)` | Character select entry. `name_label` is the suffix of your `nam_chr*_00_<x>` labels; defaults to `name`. |
| `narration(label)` | Announcer call, `vc_narration_characall_<name>`. |
| `staffroll()` | Serve `standard/staffroll/texture/standard_staffroll_<name>.nutexb` from the pack. |
| `own_css()` | Do not publish the select entry; the pack's CSK code does, with `hash40("fighter_kind_<name>")` and `hash40("ui_chara_<name>")`. |
| `jingle(name)` | Accepted, not served yet. |
| `ui_chara(s)`, `fighter_kind_name(s)`, `resource_name(s)`, `base_resource_name(s)` | Override the names derived from `name` and `base`. |
| `owns_param_resources()`, `effect_namespace(n)`, `article_namespace(n)` | Leave out. |
| `article(name, "fighter/weapon")` | Copy that vanilla weapon into `fighter/<name>/model/<article>`, scripts under `<name>_<article>`. |
| `base_article(name, "weapon")` | Rename one of the base's own articles into that folder. |
| `kirby_article(name, "fighter/weapon")` | An article for Kirby's copy, files under `fighter/kirby/model/<name>/`. |
| `kirby(statuses)`, `kirby_first(n)` | Reserve that many copy status numbers; `kirby_first` pins the first. |
| `kirby_full_model()`, `kirby_model(dir)`, `kirby_mesh(name, visible)` | Full-body copy; one extra model folder under `fighter/kirby/model/`; a mesh's starting visibility (up to 16). |
| `kirby_motion(Motion)` | A copy animation: `Motion::new(name, "file.nuanmb")` with `.template(motion)`, `.scripts(stem)` or `.game/.sound/.effect/.expression(script)`, `.flags(&[..])`, `.blend_frames(n)`, `.cancel_frame(n)`, `.xlu(start, end)`, `.no_stop_intp(b)`, `.animation_unk(n)`, `.no_extra()`. File: `fighter/kirby/motion/<name>body/c00/`, listed under `fighter/<name>/kirbycopy/cNN/bodymotion`. Name must start with `<name>`. |
| `param(name, v)`, `param_int(name, v)`, `param_mul(name, v)` | One stat. `"a.b"` reaches a sub value. |
| `params(&[..])`, `params_int(&[..])`, `params_mul(&[..])` | Several at once. |
| `param_at(costume, name, v)`, `param_int_at`, `param_mul_at` | One costume only. |
| `raw(toml)` | Append any table the builder lacks. |
| `to_toml()` | The text the engine receives. |

`v2::register_fighter_manifest(label, text)` registers that text directly.

### Character select entry

CSK's own call, as in a one-slot mod. Two fields name the clone instead of
the base, and the manifest says `own_css()` (`css = false` in
`fighter.toml`):

```rust
use the_csk_collection_api::*;

fn publish_css() {
    let ui_chara = hash40("ui_chara_wawa"); // the clone's UI id: ui_chara_<name>
    let mut indices = HashMap::new();
    let mut hashes = HashMap::new();
    for color in 0..COSTUMES {              // one set per costume the pack ships
        indices.insert(hash40(&format!("c{color:02}_index")), UnsignedByteType::Overwrite(color)); // costume index
        indices.insert(hash40(&format!("n{color:02}_index")), UnsignedByteType::Overwrite(color)); // name index
        indices.insert(hash40(&format!("c{color:02}_group")), UnsignedByteType::Overwrite(0));     // costume group
        hashes.insert(
            hash40(&format!("characall_label_c{color:02}")),          // announcer call for this costume
            Hash40Type::Overwrite(hash40("vc_narration_characall_wawa")),
        );
    }
    hashes.insert(hash40("original_ui_chara_hash"), Hash40Type::Overwrite(hash40("ui_chara_samus"))); // the base's UI id

    allow_ui_chara_hash_online(ui_chara); // let the entry be picked online
    add_chara_db_entry_info(CharacterDatabaseEntry {
        ui_chara_id: ui_chara,                                                 // the clone's UI id
        clone_from_ui_chara_id: Some(hash40("ui_chara_samus")),                // copy every other field from the base's entry
        name_id: StringType::Overwrite(CStrCSK::new("wawa")),                  // suffix of the nam_chr*_00_<x> labels in msg_name.xmsbt
        fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_wawa")),      // the clone's identity: fighter_kind_<name>
        fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_wawa")), // same
        ui_series_id: Hash40Type::Overwrite(hash40("ui_series_metroid")),      // series icon
        disp_order: SignedByteType::Optional(Some(40)),                        // position on the select screen
        color_num: UnsignedByteType::Overwrite(COSTUMES),                      // number of costumes
        shop_item_tag: Hash40Type::Overwrite(hash40("-1")),                    // the base's DLC fields hide the entry; clear them
        alt_chara_id: Hash40Type::Overwrite(hash40("-1")),
        save_no: SignedByteType::Overwrite(0),
        is_dlc: BoolType::Overwrite(false),
        is_patch: BoolType::Overwrite(false),
        extra_index_maps: UnsignedByteMap::Overwrite(indices),
        extra_hash_maps: Hash40Map::Overwrite(hashes),
        ..Default::default()
    });
    for color in 0..COSTUMES {              // one layout per costume; the base has only 8, so wrap
        add_chara_layout_db_entry_info(CharacterLayoutDatabaseEntry {
            ui_layout_id: hash40(&format!("ui_chara_wawa_{color:02}")),                       // layout id: ui_chara_<name>_<costume>
            clone_from_ui_layout_id: Some(hash40(&format!("ui_chara_samus_{:02}", color % 8))), // copy the base's layout for that costume
            ui_chara_id: Hash40Type::Overwrite(ui_chara),                                     // the clone's UI id
            chara_color: UnsignedByteType::Overwrite(color),                                  // the costume
            ..Default::default()
        });
    }
}
```

`fighter_kind` and `fighter_kind_corps` are `hash40("fighter_kind_<name>")`,
`ui_chara_id` is `hash40("ui_chara_<name>")`. The rest is CSK's.

Without `own_css()` the engine publishes an entry itself from `name_label`,
`series`, `disp_order`, `save_no`, `exhibit_year` and `narration`. Do not do
both; CSK keeps the last one written.

### `fighter.toml`

The same declaration as a file beside `config.json`, read at boot. The plugin
then skips `register`; `WAWA.ready()` is already true. If both exist the
file wins.

```toml
name = "wawa"             # folder under fighter/, Smashline agent name, identity
base = "samus"            # the vanilla fighter it is built on
costumes = 8              # costumes c00..c07
css = false               # the plugin publishes the CSK entry

[[article]]
name = "beam"             # article folder fighter/wawa/model/beam, scripts under "wawa_beam"
from = "samus/cshot"      # the vanilla weapon it is copied from

[kirby]
statuses = 2              # status numbers reserved for Kirby's copy

[params.mul]
weight = 1.02             # multiply the base's value
```

With no plugin, drop `css = false` and add `series`, `disp_order`,
`display_name`, `narration`; the engine publishes the entry.

| Key | `Manifest` method |
|---|---|
| `name`, `base`, `costumes`, `color_start`, `series`, `disp_order`, `save_no`, `exhibit_year`, `narration`, `staffroll`, `jingle`, `ui_chara`, `fighter_kind_name`, `resource_name`, `base_resource_name` | Same name. |
| `display_name` | `name_label`. |
| `css = false` | `own_css()`. |
| `[[article]]` with `name`, `from` or `base_article`, `kirby = true` | `article`, `base_article`, `kirby_article`. |
| `[kirby]` with `statuses`, `first`, `full_model`, `model`; `[[kirby.mesh]]` with `name`, `visible` | `kirby`, `kirby_first`, `kirby_full_model`, `kirby_model`, `kirby_mesh`. |
| `[[kirby.motion]]` with `name`, `animation`, `template`, `scripts` or `game`/`sound`/`effect`/`expression`, `flags` or `loop`, `blend_frames`, `cancel_frame`, `xlu`, `no_stop_intp`, `animation_unk`, `no_extra` | `kirby_motion`. |
| `[params]`, `[params.mul]`, `[params.cNN]`, `[params.cNN.mul]` | `param*`. A decimal is a float, a whole number an integer. |

Several fighters in one file: `[[fighter]]` blocks, with `[[fighter.article]]`,
`[fighter.kirby]` and so on.

### `Fighter`

| Method | Meaning |
|---|---|
| `kind()` | The kind this boot. Never store it. |
| `ready()` | Registered. |
| `is(x)` | `x` is this fighter. `x`: `&L2CFighterCommon`, `&L2CAgentBase`, `&L2CWeaponCommon`, `*mut BattleObject`, `*mut Fighter`, `*mut Weapon`, `*mut BattleObjectModuleAccessor`, `u64`. |
| `owns(x)` | `x` is this fighter or one of its weapons or articles. |
| `entries()`, `in_match()` | Match slots it occupies. |
| `article("name")` | `.status(line, status, f)`, `.install()`, `.weapon_kind()`, `.index()`, `.spawn(boma)`, `.set_vtable_entry(n, f)`. Read `index()` right before use; never fall back to 0. |
| `copy_article("name")` | Same, for a `kirby_article`. |
| `param("name")` | `.sub("field")`, `.slot(n)`, then `.set(v)`, `.mul(v)` or `.int(v)`. |
| `kirby_family()`, `kirby_status(n)`, `arm_kirby()` | The reserved statuses. Arm after the `Agent::new("kirby")` statuses are installed. |
| `set_vtable_entry(n, f)` | Write one vtable entry; returns the previous. |
| `register(manifest)` | Above. |

Compare fighters with `is` and `owns`; `utility::get_kind` answers the base.

### Scripts

Smashline, under your own name:

```rust
smashline::Agent::new("wawa")                                            // the clone's name, never the base's
    .game_acmd("game_attack11", attack11, smashline::Priority::Default)  // script name, your function, priority
    .status(smashline::Pre, *FIGHTER_STATUS_KIND_SPECIAL_N, special_n_pre) // status line, status kind, your function
    .on_line(smashline::Main, fighter_frame)                             // once per frame
    .install();
```

### Mount event

`v2::on_mods_mounted(callback)`: runs once ARCropolis has mounted the mods.

## `#[hook]`

```rust
// slot: the fighter vtable entry to replace, by name.
// class: the class object the game passes first; fighter: the fighter; log: the entry's own argument.
#[hook(slot = slot::fighter::ON_SEARCH)]
unsafe fn on_search(class: u64, fighter: *mut smash::app::Fighter, log: u64) -> u64 {
    call_original!(class, fighter, log) // the base's entry, with the same arguments
}

// article: which of the fighter's articles; slot: a weapon vtable entry.
#[hook(slot = slot::weapon::ON_ATTACK, article = "fireball")]
unsafe fn fireball_hit(class: u64, weapon: *mut smash::app::Weapon, log: u32) {
    call_original!(class, weapon, log)
}

// item: the item! static; slot: an item vtable entry. item: the item object (no class object first).
#[hook(slot = slot::item::UPDATE_9, item = BLOCK)]
unsafe fn block_update(item: *mut smash::app::BattleObject) {
    call_original!(item);
    smash::app::lua_bind::ModelModule::set_scale((*item).module_accessor, 1.5); // after the original: the frame's last word
}

// offset: address of a game function in main; me: the parameter that must be this fighter's,
// otherwise the call goes straight to the original.
#[hook(offset = 0x33bd9c0, me = weapon)]
unsafe fn weapon_hit(vtable: u64, weapon: *mut smash::app::Weapon, log: u32) {
    call_original!(vtable, weapon, log)
}
```

| Attribute | Meaning |
|---|---|
| `slot = slot::fighter::NAME` | Replace that entry in the fighter's own vtable copy. Runs for this fighter only. First parameter is the class object, the fighter second. Names: `slot::fighter::*` (146), `slot::weapon::*` (104). |
| `slot = slot::weapon::NAME, article = "name"` | Same, in the article's own copy. |
| `slot = slot::item::NAME, item = ITEM` | Same, in the item's own copy (181 entries). First parameter is the item object. `INITIALIZE(item, id, kind, flag, record)` at every spawn, `kind` is the base's; `START(item)` after it; `UPDATE_1..UPDATE_9(item)` each frame in order, statuses run inside them. Unnamed entries are `UNKn`. |
| `offset = 0x...` | Hook game code that is not a vtable entry, shared with other packs. Up to six integer or pointer parameters, integer, pointer or no return. |
| `me = <parameter>` | Offset form only: skip the call unless that parameter is this fighter's. `of = OTHER` names another fighter. |
| `expect = [w0, w1, w2, w3]` | Offset form: the words expected at the address. Without it the address must be an untouched function start. |

`install_hooks!(a, b, ...)` installs them, after registration. A hook on the
base's function through plain `#[skyline::hook]` also runs for the clone;
`replace` is not supported.

`v2::get_agent_virtual_function(kind, index, is_weapon, get_ptr)`: the
tutorials' signature, returns 0 instead of aborting; `get_ptr = true` on a
clone kind is 0.

## Parameters

Two syntaxes, by where the game reads the value.

| Values | Syntax |
|---|---|
| `vl.prc`: `param_special_*`, `param_private`, `fly_data`, `air_lasso_data`, everything under the fighter's own file | ParamConfig |
| `fighter_param.prc` fields (the attributes: `walk_speed_max`, `jump_y`, `scale`, `weight`, ...) | engine |
| `fighter_param_motion.prc` fields (`param_motion`) | engine |
| `fighter/common/param/` files: `common.prc`, `item.prc`, `etc.prc`, `power_up.prc`, `effect.prc`, `sound.prc` (`common`) | engine |
| `fighter_param_thrown.prc` (`param_thrown`) | engine |
| lists and arrays inside `vl.prc` (`hit_data`, `hit_target`, `map_coll_data`, `cliff_hang_data`, ...) | your own `vl.prc` file |
| lists inside the common files, `battle_object.prc`, `spirits.prc`, the match-level fields of the common files | cannot be changed per clone |

### ParamConfig

`vl.prc` values, read through `WorkModule::get_param_*`, which ParamConfig
hooks. Its calls, with the clone's kind where the base kind was; `vec![-1]`
is every costume.

```rust
let kind = WAWA.kind().get().unwrap();                 // the clone's kind, in place of *FIGHTER_KIND_SAMUS
let slots: Vec<i32> = (0..COSTUMES as i32).collect();  // costumes; vec![-1] is all of them

// (kind, costumes, (param table, field), value)
param_config::update_float(kind, slots.clone(), (hash40("param_special_hi"), hash40("y_spd_air")), 1.5);
// a top-level field has no table: second hash is 0
param_config::update_int(kind, slots.clone(), (hash40("jump_squat_frame"), 0), 4);
// multiply the base's float value
param_config::update_attribute_mul(kind, slots.clone(), (hash40("param_special_n"), hash40("fireball_speed_mul")), 1.15);
// multiply the base's integer value
param_config::update_int_mul(kind, vec![-1], (hash40("landing_frame"), 0), 0.5);

// (kind, costumes): Kirby cannot copy these costumes
param_config::disable_kirby_copy(kind, vec![-1]);
let beam = WAWA.article("beam").weapon_kind().unwrap(); // the article's weapon kind; 0 means every weapon the fighter owns
// (kind, costumes, weapon kind, behaviour): what Kirby's inhale does to the article
param_config::set_kirby_inhale_behavior(kind, vec![-1], beam, param_config::POCKET_BEHAVIOR_MISFIRE);
// what Villager's pocket does to it
param_config::set_villager_pocket_behavior(kind, vec![-1], beam, param_config::POCKET_BEHAVIOR_DELETE);
// same as POCKET_BEHAVIOR_MISFIRE
param_config::disable_villager_pocket(kind, vec![-1], beam);
// what Rosalina's pull does to it
param_config::set_rosetta_pull_behavior(kind, vec![-1], beam, param_config::POCKET_BEHAVIOR_IGNORE);
// (weapon kind, use type): the article's use type
param_config::set_article_use_type(beam, *ARTICLE_USETYPE_FINAL);
```

Behaviours: `POCKET_BEHAVIOR_ORIGINAL`, `IGNORE`, `DELETE`, `MISFIRE`.

`clone_engine_api::param_*` send the same keys to ParamConfig, for a pack
without the `param_config` crate: `param_disable_kirby_copy(kind)`,
`param_kirby_inhale_behavior(kind, weapon_kind, behavior)`,
`param_villager_pocket_behavior`, `param_disable_villager_pocket`,
`param_rosetta_pull_behavior`, `param_article_use_type(weapon_kind,
use_type)`, and for one costume `param_disable_kirby_copy_slot(kind, slot)`,
`param_kirby_inhale_behavior_slot(kind, slot, weapon_kind, behavior)`,
`param_villager_pocket_behavior_slot`, `param_disable_villager_pocket_slot`,
`param_rosetta_pull_behavior_slot`.

### Engine

The four groups below are not read through `get_param_*`, or not only: the
game reads `fighter_param.prc` and `fighter_param_motion.prc` rows through
`FighterParamAccessor2` and, for most fields, straight out of the row
(`scale` is never read any other way); the six common files are per-fighter
copies read directly; `fighter_param_thrown.prc` has no field names. A
ParamConfig entry for these applies to some reads and not others. The engine
writes the clone's row, copies and results, and pushes `set` values to
ParamConfig too, so one call covers every read.

```rust
// param(name) then .sub(field) for a nested value, .slot(n) for one costume,
// then .set(value), .mul(factor) or .int(value)
WAWA.param("walk_speed_max").set(1.2);                              // fighter_param.prc field: (name), no sub
WAWA.param("scale").set(1.1);                                       // read once at construction, engine only
WAWA.param("param_motion").sub("escape_air_slide_distance").set(60.0); // fighter_param_motion.prc: ("param_motion", field)
WAWA.param("param_motion").sub("flip").int(0);                      // an integer field
WAWA.param("common").sub("shield_max").set(20.0);                   // any of the six common files: ("common", field)
WAWA.param("common").sub("shield_reset").set(7.5);                  // set with shield_max, the game does not clamp it
WAWA.param("param_thrown").sub("offset_f_y").set(8.0);              // where a fighter this clone throws sits: forward throw, Y
WAWA.param("param_thrown").sub("held_offset").mul(1.2);             // where the clone sits when thrown: whole vector, mul only
```

```toml
[params]
walk_speed_max = 1.2
param_motion.flip = 0                 # a whole number is an integer
common.shield_max = 20.0

[params.mul]
param_thrown.held_offset = 1.2

[params.c03]
param_special_hi.y_spd_air = 2.0      # costume 3 only; a vl.prc value, pushed to ParamConfig
```

| Param | Sub | Notes |
|---|---|---|
| a `fighter_param.prc` field | none | Every costume only. |
| `param_motion` | a `fighter_param_motion.prc` field | |
| `common` | a top-level field of `common.prc`, `item.prc`, `etc.prc`, `power_up.prc`, `effect.prc`, `sound.prc` | Lists inside them and the ~40 match-level fields (handicap, team attack, finish camera, area wind, `power_up_point_min`) stay global. |
| `param_thrown` | holder: `offset`, `offset_f`, `offset_b`, `offset_hi`, `offset_lw`; victim: `held_offset`; each also `_x`, `_y`, `_z` | Whole vector: `mul` only. Component: `set` or `mul`. Every costume only. |

### Lists and arrays

Neither syntax keys into a list. `vl.prc` lists (`hit_data`, `hit_target`,
`map_coll_data`, `cliff_hang_data`, `jostle_map_coll_data`,
`virtual_node_data_hit`, ...) come from your own
`fighter/<name>/param/vl.prc`, declared under every costume in
`config.json`; a costume that misses it falls back to the base's whole file.
Lists inside the common files, `battle_object.prc` and `spirits.prc` cannot
be changed per clone.

## Items

### Registration by code

```rust
use clone_engine_api::v2::{item, ItemManifest};
use clone_engine_api::ItemStatusLine;

// WAWA: the handle. "wawa": the resource name, same as ItemManifest::new's.
item!(WAWA, "wawa");

#[skyline::main(name = "wawa_item")]
pub fn main() {
    let manifest = ItemManifest::new(
        "wawa", // resource name: files under item/wawa, script name
        0x1ae,  // base kind: the vanilla item it is built on (0x1ae = Steve's block)
    )
    .base_item("pickelobject")             // the base's name, for the log
    .spawn_per(30)                         // natural drop weight
    .common("throw_speed_mul", 0.75)       // a field of item/common/param/param.prc, value
    .owner_param("pickel", "life", 600.0); // owner fighter (Steve), a field of its vl.prc, value
    if WAWA.register(manifest).is_err() {
        return; // the fault is in the log
    }
    WAWA.status(ItemStatusLine::Init, "THROW", throw_init); // status line, status name of the base item, your function
}
```

| `ItemManifest` method | Meaning |
|---|---|
| `new(resource_name, base_kind)` | Files under `item/<resource_name>`; the vanilla item kind it is built on. |
| `base_item(name)` | Names the base in the log. |
| `agent_name(name)` | Script name. Default `resource_name`. |
| `ui_id(id)` | Training menu id. Default `ui_item_<resource_name>`. |
| `training_order(n)` | Position in the Training list. |
| `spawn_per(weight)` | Natural drops. Vanilla items sit at 20..50, capsules at 100. Follows the base item's item switch. |
| `spawn_range(min, max)` | How many appear at once. Default 1, 1. |
| `spawn_from(&["box", "barrel"])` | Container drops: `box`, `barrel`, `capsule`, `carrierbox`, `kusudama`, `sandbag`, `grass`. |
| `common(field, v)`, `common_int(field, v)` | A field of `item/common/param/param.prc`, for this item only. |
| `owner_param(fighter, field, v)`, `owner_param_int` | A field of that fighter's `vl.prc`, for this item only. |
| `raw(toml)`, `to_toml()` | As for `Manifest`. |

`v2::register_item_manifest(label, text)` registers that text directly.

### `item.toml`

```toml
base_kind     = 430                 # the vanilla item it is built on (430 = 0x1ae, Steve's block)
resource_name = "wawa"              # files under item/wawa, script name
base_item     = "pickelobject"      # optional: the base's name, for the log
agent_name    = "wawa"              # optional: script name, default resource_name
ui_id         = "ui_item_wawa"      # optional: Training menu id, default ui_item_<resource_name>
training_order = 0                  # optional: position in the Training list
spawn_per  = 30                     # optional: natural drop weight; none without it
spawn_min  = 1                      # optional: how many appear at once
spawn_max  = 2
spawn_from = "box, barrel, capsule" # optional: containers that can hold it

[common]
throw_speed_mul = 0.75              # a float field of item/common/param/param.prc
life = 300                          # a whole number is an integer field

[owner_params]
pickel.life = 600                   # owner fighter, then a field of its vl.prc
pickel.auto_damage = 0
```

Keys map to the methods above by name. A decimal is a float, a whole number
an integer. Several items: `[[item]]` blocks (engines after `0.2.1-beta.1`).

### `Item`

| Method | Meaning |
|---|---|
| `kind()`, `ready()`, `is(x)` | As for `Fighter`. |
| `status(line, "NAME", f)` | Status callback by name, registered at startup. Lines: `Setting`, `JointSrt`, `Init`, `Update`, `Coroutine`, `Exit`. Statuses are the base item's; an unknown name is refused. |
| `owner_param("fighter", "field").set(v)` | As the manifest method. |
| `vtable_entry(n)`, `set_vtable_entry(n, f)` | The item's own vtable copy. |
| `register(manifest)` | Above. |

Spawning from code:

```rust
ItemModule::have_item(
    boma,                                            // the fighter that gets the item
    smash::app::ItemKind(WAWA.kind().get().unwrap()), // the clone's kind, wrapped in the game's type
    0, 0, false, false,                              // the game's own arguments, as for a vanilla item
);
```

`born_item` and `attach_item` take the kind the same way.
`ItemModule::get_have_item_kind` answers the base kind; use `item_kind_held`.

| Function | Meaning |
|---|---|
| `item_kind_for_identity(resource_name)` | The kind a registered name landed on. |
| `item_base_kind(kind)` | Its vanilla base. |
| `is_item_kind(kind)` | Registered by the engine. |
| `item_resource_name(kind)` | Its file root. |
| `item_kind_from_object(object)`, `item_kind_from_boma(boma)` | Kind of a live item. Unsafe. |
| `item_kind_held(boma, index)`, `item_kind_pickable(boma)` | What a fighter holds or could pick up. Unsafe. |
| `item_common_has(hash)` | Whether a common field is supported. |
| `item_common_set_label(kind, hash, label)` | A kind field of the common file by label, such as `hash40("shield_kind")` to `hash40("item_shield_kind_lost")`. |
| `item_common_set_hash(kind, hash, name)` | A bone or motion name field, such as `thrown_node`. |
| `item_backend_status()` | `ITEM_BACKEND_STATUS_*` readiness bits. |

Common fields: `have_kind`, `size_kind`, `hit_kind`, `shield_kind`,
`reflect_kind`, `flip_type`, `bound_flag`, `scale_type`, `camera_kind`,
`eatable`, `paintable`, `ai_pri`, `thrown_rot_kind`, `thrown_node`,
`clung_node`, `captured_motion`, ... `group` cannot be set.

Item families (`register_item_family`, `item_category`, `item_family_owner`,
`item_family_member_index`, `item_spawn_source`, `item_parent_kind`) need an
engine built with `research_item_families`; the release returns
`ERROR_BACKEND_UNAVAILABLE`.

## Stages

`stage.toml` only; see [Stages](wiki/STAGES.md). A stage plugin holds code
and registers nothing. The old calls are in
[DEPRECATED.md](DEPRECATED.md#stages).

## Engine state

| Function | Meaning |
|---|---|
| `api_version()` | Installed engine version. |
| `compiled_capabilities()`, `runtime_capabilities()` | `CAP_*` bits built in, and passing this boot. |
| `max_custom_kind()` | Highest fighter kind accepted. |
| `kind_for_identity("fighter_kind_<name>")` | The kind given to that identity. |
| `smashline_bridge_version()`, `smashline_compatible()` | Smashline build found. |
| `native_backend_status()` | Diagnostic flags. |
| `log(&str)`, `elog!(..)` | Engine log. |

Registration closes when the roster is built: `ERROR_REGISTRATION_CLOSED`.

## Errors

`Error::Engine(code)`:

| Code | Meaning |
|---|---|
| `ERROR_BACKEND_UNAVAILABLE` | Not in this build. |
| `ERROR_REGISTRATION_CLOSED` | Registered after the roster was built. |
| `ERROR_DUPLICATE` | Identity or kind taken; the log says what by whom. |
| `ERROR_NAMESPACE` | No effect or article namespace left. |
| `ERROR_ARTICLE_RESOURCE_CONFLICT` | Two articles in one folder. |
| `ERROR_HOOK_PREFLIGHT` | Game code did not match the hook's expectation. |
| `ERROR_HOOK_ABI` | Hook signature not supported. |
| `ERROR_ITEM_UI_METADATA` | Training menu data malformed. |
| `ERROR_ITEM_UI_UNAVAILABLE` | Item UI layer unavailable. |
| `ERROR_MANIFEST` | Manifest did not parse or registered nothing. |
| `ERROR_SMASHLINE_REQUIRED` | Retired. |

Other constants in `clone_engine_api`.

## C ABI

Exports are `clone_engine_*`; the Rust client is the authoritative list.
Every structure is `#[repr(C)]` with `api_version` and `struct_size` first
and zeroed reserved fields.
