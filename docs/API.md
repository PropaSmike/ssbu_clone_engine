# Clone Engine API

The Rust API your plugin calls to give a fighter, article, item, or stage its
own identity.

If Clone Engine is not installed, every call returns `Error::EngineUnavailable`
instead of crashing, so a plugin can check once and disable itself.

The templates are working versions of everything below:
[fighter](../custom_fighters/template/),
[item](../custom_items/template/),
[stage](../custom_stages/template/).

## Add the crate

```toml
[dependencies]
clone_engine_api = { git = "https://github.com/PropaSmike/ssbu_clone_engine", tag = "0.1.0-beta.1" }
```

## Fighters

```rust
use clone_engine_api::{CloneRegistration, KIND_AUTO};

let mut fighter = CloneRegistration::new(
    KIND_AUTO,
    3,                    // base kind: Samus
    "ui_chara_wawa",
    "fighter_kind_wawa",
    "wawa",               // your files live under fighter/wawa
    "samus",              // fallback for files you do not ship
);
fighter.color_count = 8;

let kind = clone_engine_api::allocate(&fighter)?;
```

Never hardcode the number `allocate` returns. It depends on which mods are
installed. Your permanent identity is the name `fighter_kind_wawa`; keep the
number in a variable and read it back when you need it.

### Descriptor fields

| Field | Meaning |
|---|---|
| `custom_kind` | `KIND_AUTO` to let the engine choose. |
| `base_kind` | The vanilla fighter you inherit behavior from. |
| `ui_chara` | CSK identity, such as `ui_chara_wawa`. |
| `fighter_kind_name` | Your permanent identity name. |
| `resource_name` | Your file root, such as `wawa`. |
| `base_resource_name` | Vanilla root used for files you do not ship. |
| `color_start`, `color_count` | Costumes your pack actually contains. |
| `effect_namespace`, `article_namespace` | Leave zero and the engine allocates them. |
| `articles` | Base weapon kinds that need file names of their own. |
| `copy_status_first`, `copy_status_count` | Optional Kirby copy status family. |
| `flags` | `FLAG_OWNS_PARAM_RESOURCES`, `FLAG_KIRBY_COPY_FULL_MODEL`. |

The engine copies every string before returning, so nothing has to outlive the
call.

### Registration and engine state

| Function | Meaning |
|---|---|
| `allocate(&CloneRegistration)` | Register and get a kind. Use this. |
| `register(&CloneRegistration)` | Register a kind you chose yourself. |
| `kind_for_identity(name)` | The kind already given to an identity name. |
| `max_custom_kind()` | Highest kind this engine accepts. |
| `capacity_committed()` | Whether the expanded table exists yet. Normally false during `main`. |
| `api_version()` | Version of the installed engine. |
| `compiled_capabilities()` | What the installed engine was built with. |
| `runtime_capabilities()` | What passed its checks this boot. |
| `smashline_bridge_version()`, `smashline_compatible()` | Which Smashline build it found. Informational. |
| `native_backend_status()` | Detailed flags, for diagnosing a failure. |
| `log(&str)` | Write to the engine's synchronous debug channel. `elog!` formats. |

Registration closes once the game starts building the roster. After that these
calls fail with `ERROR_REGISTRATION_CLOSED` and change nothing.

## Knowing which fighter you are

Native code often presents a clone as its base. Read the kind through these
helpers instead of taking it off the object.

| Function | Use |
|---|---|
| `is_kind(boma, kind)` | Fighter check. Use this one. |
| `is_owned_by_kind(boma, kind)` | Weapon and article check. Use this one. |
| `true_kind(boma)` | The kind behind a fighter BOMA. |
| `owner_true_kind(boma)` | The kind of a fighter, or of a weapon's owner. |
| `base_kind(kind)` | The vanilla base a clone was registered on. |
| `entry_kind(entry_id)` | The kind of a match entry. |
| `article_owner_kind(boma_as_u64)` | `owner_true_kind` without importing smash types. |
| `pocket_holder_kind(boma_as_u64)` | Original owner of a pocketed weapon. |

```rust
if clone_engine_api::is_kind(fighter.boma(), my_kind()) { }
if clone_engine_api::is_owned_by_kind(weapon.boma(), my_kind()) { }
```

## Scripts

ACMD, statuses, OPFF, lifecycle callbacks and weapon ACMD all belong to
Smashline, registered under your own agent name:

```rust
smashline::Agent::new("wawa")
    .game_acmd("game_attack11", attack11, smashline::Priority::Low)
    .status(smashline::Pre, *FIGHTER_STATUS_KIND_SPECIAL_N, special_n_pre)
    .on_line(smashline::Main, fighter_frame)
    .install();
```

Registering on the base name instead would change the vanilla fighter and every
other mod built on it.

Any Smashline build works. Clone Engine supplies your agent's name itself, so
there is nothing to check and no fork to install.

## Parameters

ParamConfig applies the values. Clone Engine tells it your allocated kind, which
is the part ParamConfig cannot work out for a custom fighter. Every helper
returns `false` if ParamConfig is missing or refuses the request.

| Function | Scope |
|---|---|
| `param_override(kind, param, op, value)` | Float, every costume. |
| `param_override_slot(kind, slot, param, op, value)` | Float, one costume. |
| `param_override_full(kind, slot, param, subparam, op, value)` | Float, nested. |
| `param_int_override(kind, param, value)` | Integer, every costume. |
| `param_int_override_slot(kind, slot, param, value)` | Integer, one costume. |
| `param_int_override_full(kind, slot, param, subparam, value)` | Integer, nested. |

`ParamOp::Set` and `ParamOp::Mul` pick the writer. Integer multiply is not
bridged, because the supported ParamConfig build does not export it. Pass
`ANY_SLOT` for every costume.

Some fields like `scale`, `jump_y` and `walk_speed_max` are read by the game
in two ways. `param_override` with `ANY_SLOT` covers both. `param_override_slot`
covers only one, so use `ANY_SLOT` for those fields.

Dodge and roll frames, air dodge slide speeds, capture offsets and the other
fields of `fighter_param_motion.prc` take `param_motion` as the param and the
field as the subparam, which is how the game itself keys them:

```rust
param_override_full(kind, ANY_SLOT, "param_motion", "escape_air_slide_speed", ParamOp::Mul, 2.0);
param_int_override_full(kind, ANY_SLOT, "param_motion", "flip", 0);
```

Registering a motion field without `param_motion` is accepted and does
nothing; the engine log says which key to use.

The shared files `common.prc`, `item.prc`, `etc.prc`, `power_up.prc`,
`effect.prc` and `sound.prc` under `fighter/common/param/` can also be
changed for one clone. Every fighter carries its own copy of the six, and
the game reaches all of them with `common` as the param, so:

```rust
param_override_full(kind, ANY_SLOT, "common", "shield_max", ParamOp::Set, 20.0);
param_override_full(kind, ANY_SLOT, "common", "dash_stick_x", ParamOp::Set, 0.5);
param_int_override_full(kind, ANY_SLOT, "common", "dead_up_star_move_frame", 60);
```

Real fighters keep the shipped values. Top-level values only; the colour
lists and the `power_up_*` multiplier lists are not addressable this way,
and a few dozen match-level values (handicap boost, team attack rates, the
finish camera, area wind, `power_up_point_min`) are read from a shared
instance and stay global.

Values that the game pairs with each other need changing together. The
clearest case is the shield: a broken shield comes back with `shield_reset`
HP (37.5), an absolute value the game does not clamp to `shield_max`, and
the bubble is drawn from HP over max. Lower `shield_max` alone and the
second shield is bigger and tougher than the first; lower `shield_reset`
with it.

`fighter_param_thrown.prc` holds where a thrown fighter's body sits during
the holder's throw animations, one XYZ per (holder, victim) pair. Its values
are not addressed by name: a clone gets rules instead, keyed with
`param_thrown`, that the engine applies to every read for that clone. Use
them when your model is a different size or shape from the base's.

```rust
// The clone throws someone: scale every hold offset it produces.
param_override_full(kind, ANY_SLOT, "param_thrown", "offset", ParamOp::Mul, 1.2);
// ... or one component of one throw: forward throw, Y, fixed.
param_override_full(kind, ANY_SLOT, "param_thrown", "offset_f_y", ParamOp::Set, 8.0);
// Someone throws the clone: raise it while it is held.
param_override_full(kind, ANY_SLOT, "param_thrown", "held_offset_y", ParamOp::Mul, 1.2);
```

Holder keys are `offset`, `offset_f`, `offset_b`, `offset_hi`, `offset_lw`
(forward, back, up and down throws; `offset` covers all of them plus the
special holds such as a DK clone's cargo throws), each with `_x`, `_y`, `_z`
variants. Victim keys are `held_offset` and its `_x`, `_y`, `_z` variants,
applied whoever the holder is. A whole-vector key takes `Mul` only; a
component takes `Set` or `Mul`. Rules apply in registration order, `ANY_SLOT`
only, and never reach ParamConfig.

`battle_object.prc` and `spirits.prc` cannot be changed per clone.

Interaction rules, each with a `_slot` variant for one costume:

| Function | Meaning |
|---|---|
| `param_article_use_type(weapon_kind, use_type)` | Article use type. |
| `param_disable_kirby_copy`, `param_disable_kirby_copy_slot` | Stop Kirby copying you. |
| `param_kirby_inhale_behavior`, `param_kirby_inhale_behavior_slot` | Inhale behavior. |
| `param_villager_pocket_behavior`, `param_villager_pocket_behavior_slot` | Pocket behavior. |
| `param_disable_villager_pocket`, `param_disable_villager_pocket_slot` | Shorthand for an unpocketable article. |
| `param_rosetta_pull_behavior`, `param_rosetta_pull_behavior_slot` | Luma pull behavior. |

Behaviors are `PARAM_BEHAVIOR_ORIGINAL`, `IGNORE`, `DELETE` and `MISFIRE`.
Weapon kind zero means every weapon you own, where the underlying rule supports
it.

Use these for values in shared files such as `common/fighter_param.prc`.
A `vl.prc` of your own goes in your own folders instead.

## Articles

`clone_article` copies a vanilla weapon into a new weapon kind that belongs to
your fighter, with its own files, and Smashline registers its ACMD
under its own agent name.

A weapon kind and an article-table index are different numbers. The kind is
permanent; the index moves when other mods add articles.

```rust
let barrel = clone_engine_api::clone_article_handle(
    "koopajr",
    WEAPON_KIND_KOOPAJR_CANNONBALL,
    "wawa",
    "wawa_barrel",
    base_kind,
)?;

if let Some(index) = barrel.index() {
    ArticleModule::generate_article(boma, index, false, 0);
}
```

| Function | Meaning |
|---|---|
| `clone_article_handle(...)` | Mint an article and get a handle. Use this. |
| `clone_article(source_owner, source_weapon, destination_owner, name)` | Mint one and get the raw weapon kind. |
| `clone_article_for(...)` | Same, when the table owner and the file owner differ. |
| `article_index(fighter_kind, weapon_kind)` | Resolve the current table index. |
| `article_status(weapon_kind, line, status, function)` | Add status code to a minted article. |
| `clone_copy_article_handle(...)` | Article for a move Kirby copied from you. |
| `clone_copy_article(target_kind, source_owner, source_weapon, resource_owner, name)` | The same without a handle. |
| `copy_article_index(target_kind, weapon_kind)` | Current index in the Kirby-copy table. |

Resolve `index()` immediately before you use it, and never fall back to zero on
failure. Zero is a real vanilla article, so the fallback spawns the wrong thing.

Each article needs a file directory name of its own.

## Kirby copies

Set `copy_status_first` and `copy_status_count` in the descriptor, register the
Kirby status scripts, then publish the family:

```rust
clone_engine_api::arm_kirby_copy_status_family(kind, first, count);
```

Calling `arm_kirby_copy_status_family` before those scripts exist is the one
ordering that matters here. The engine creates the copy record, routes the
model, motion and article files, and sends only the copy of your fighter to your
status family.

`FLAG_KIRBY_COPY_FULL_MODEL` gives Kirby your whole body instead of a hat, and
needs a complete model and motion set for every Kirby color. If a copied move
has an article of its own, mint it with `clone_copy_article_handle`.

Do not create empty `fighter/<clone>/kirbycopy/cNN` groups, and do not fill them
with the base fighter's Kirby files. Both produce a resource-cache failure.

### Extra copy models

A copy gets one model, `copy_<your resource name>_fitkirby`. Vanilla allows up
to three more.
Declare yours the same way:

```rust
clone_engine_api::clone_copy_model(kind, "wawa_kirby_model")?;
```

Each is a directory of its own under `fighter/kirby/model/`, shipped and
declared like the first. A mesh or bone in a model you did not declare is never
loaded, so everything your copy animation drives has to live in one of these.

### Copy motions of your own

Kirby's copy animates from the base fighter's copy animations.
`clone_copy_motion` adds animations of your own beside them:

```rust
let motion = clone_engine_api::CopyMotion::new(
    "my_fighter_special_n",
    "myfighterd00specialn.nuanmb",
)
.template("mario_special_n")
.game_script("game_myfighterspecialn");

clone_engine_api::clone_copy_motion(kind, &motion)?;
```

`template` names a copy motion your base already has and brings over its
scripts, flags, blend frames and cancel frame. That includes the ACMD, so a
motion you gave no scripts to keeps the base fighter's hitboxes. Override them
with `game_script` and `scripts`, and register those on `Agent::new("kirby")`,
whose table the motion is looked up in. They do not have to exist in the game
already. The rest of the builder is `flags`, `blend_frames`, `xlu`,
`cancel_frame`, `no_stop_intp`, `animation_unk` and `without_extra`.

Ship the animation as
`fighter/kirby/motion/<your resource name>body/c00/<file>.nuanmb`, list it in
`new-dir-files` under `fighter/<your resource name>/kirbycopy/cNN/bodymotion`
for every color, and play it from your copy status by the name you registered.

Your motions are added to the base fighter's, never in place of them. Names must
be unique across every installed pack, and 256 can be registered in total.

### Mesh visibility defaults

Hide or show a hat mesh from the start, before any copy animation plays:

```rust
clone_engine_api::clone_copy_mesh_default(kind, "wing", false)?;
```

Use the mesh name your animations use. Up to 16 per fighter. Your animations
still control the mesh once they play.

## Shared hooks

When two movesets need the same game function, the broker installs one hook and
runs the registered callbacks in order.

| Function | Meaning |
|---|---|
| `shared_hook_checked(&SharedHookRegistrationV1)` | Register a callback. Use this. |
| `shared_hook(offset, callback)` | Register against an allowlisted offset, without a fingerprint. |
| `shared_hook_status()` | Broker readiness and failure flags. |
| `shared_hook_original(offset, &[u64; 6])` | Run the original exactly once. |

Callbacks take integer and pointer arguments in `x0..x5` and return one value in
`x0`. Floats, struct returns and variadics are not covered. Return
`HOOK_DECLINED` to fall through to the next callback, `HOOK_HANDLED` if you
produced the result. A subscribing NRO must stay loaded for the whole process.

## Items

A custom item is not a fighter article. It gets its own kind, files, parameters,
animcmd, statuses and Training menu cell.

A pack that only ships content needs no plugin at all: put an `item.toml` beside
its `config.json` and the engine registers it at boot.

```toml
base_kind     = 63          # the vanilla item you build on
resource_name = "wawa"      # your files live under item/wawa
base_item     = "killsword" # optional, names the base in the log
agent_name    = "wawa"      # optional, defaults to resource_name
ui_id         = "ui_item_wawa" # optional, defaults to ui_item_<resource_name>
training_order = 0          # optional position in the Training list
```

Kinds are handed out in directory-name order from `FIRST_CUSTOM_ITEM_KIND`,
stepping over any kind a plugin already took. Nothing depends on the number:
your files are found by `resource_name`, so a pack keeps its assets whichever
kind it lands on.

Use the calls below when the item needs code. A pack may ship both an
`item.toml` and a plugin; whichever registers first wins and the other reuses
it.

A plugin does not pick a number either. Pass `KIND_AUTO` to `allocate_item` and
it returns the kind the engine handed you. Keep it in a `CloneItemKind`, which also
recovers the number later from the resource name, so a second plugin can find
your item without you exporting anything.

```rust
use clone_engine_api::{CloneItemKind, ItemCloneRegistration, ItemStatusLine, KIND_AUTO};

static MY_ITEM: CloneItemKind = CloneItemKind::new("my_item");

let kind = clone_engine_api::allocate_item(&ItemCloneRegistration::new(
    KIND_AUTO,
    0x32,        // vanilla base item
    "my_item",   // item/my_item
    "my_item",   // Lua agent name
))?;
MY_ITEM.store(kind);

clone_engine_api::item_status_named(
    kind,
    ItemStatusLine::Update,
    "WAIT",
    my_wait_update as *const (),
)?;
```

Use `MY_ITEM.raw()` wherever an `ItemModule` call wants a kind, and
`MY_ITEM.is(kind)` to test one you were handed. The module calls take the game's
own `smash::app::ItemKind` wrapper around that number:

```rust
ItemModule::have_item(boma, smash::app::ItemKind(MY_ITEM.raw()), 0, 0, false, false);
```

`register_item` with a number of your own still works and is what an older pack
does, but two packs that pick the same number collide and only one of them
loads.

Register statuses by name, and do it as soon as your plugin starts. The engine
keeps the name and resolves it while installing the script on a live agent,
which is the only moment the status numbers can be read. If you look the number
up yourself from an NRO-load callback it will fail, because the item module
mounts long before its constant table is written.

Unknown names are refused at registration. That check is why a typo shows up as
an error, and not as a callback that never runs.

Item callback lines are `Setting`, `JointSrt`, `Init`, `Update`, `Coroutine` and
`Exit`. They are not fighter status lines, and the statuses you can hook are the
ones your base item already has.

| Function | Meaning |
|---|---|
| `allocate_item(&ItemCloneRegistration)` | Register with `KIND_AUTO` and get the assigned kind back. Use this. |
| `item_kind_for_identity(resource_name)` | The kind an already registered resource name landed on. |
| `CloneItemKind::new(resource_name)` | A handle that caches the kind and resolves it on demand. |
| `register_item(&ItemCloneRegistration)` | Register a custom item over a vanilla base with a kind you chose. |
| `register_item_ui(&ItemUiRegistration)` | Give it a Training menu cell. |
| `item_status_named(kind, line, name, function)` | Add a status callback. Use this. |
| `item_status(kind, line, status, function)` | The same with a number you resolved. `0` is refused. |
| `item_status_kind(name)` | Resolve a status name. Fails until the item module's table is written. |
| `item_common_has(hash)` | Whether a common-item float is supported. |
| `item_common_set(kind, hash, value)` | Override one common-item float, for your item only. |
| `item_common_set_i32(kind, hash, value)` | The same for a bool (0 or 1), an int, or a kind by number. |
| `item_common_set_label(kind, hash, label)` | A kind field by its prc label, such as `item_have_kind_grip`. |
| `item_common_set_hash(kind, hash, name_hash)` | A bone or motion name field, such as `thrown_node`. |
| `item_generate_add(kind, generator, per, min, max, variation)` | Let it drop on its own, or out of a container. |
| `item_base_kind(kind)` | Its vanilla base. |
| `is_item_kind(kind)` | Whether the engine registered it. |
| `item_resource_name(kind)` | Its file root. |
| `item_kind_from_object(object)`, `item_kind_from_boma(boma)` | Recover identity from a live item. Unsafe pointer API. |
| `item_kind_held(boma, index)`, `item_kind_pickable(boma)` | What a fighter is holding or could pick up, custom kinds included. Unsafe pointer API. |
| `item_backend_status()` | Readiness flags for each item layer. |

`ItemModule::get_have_item_kind` reports the BASE kind for a custom item, and
has to: the game calls it every frame and indexes its own tables with the
result, so a custom kind coming back from it walks off the end of all of them.
Use `item_kind_held` when you want identity. Going the other way needs nothing
special, because the engine rewrites the request: `ItemModule::have_item`,
`born_item` and `attach_item` all accept a custom kind directly.

Any number of custom items may hook any number of different vanilla base items
in one session. The engine runs the vanilla item's own code first, then adds
your callbacks for your item only. Vanilla copies of the same base are left
alone.

`item_backend_status()` returns `ITEM_BACKEND_STATUS_*` bits. Check
`ITEM_BACKEND_STATUS_READY` plus the specific bit for the layer you use, such as
`ITEM_BACKEND_STATUS_STATUS_ROUTER_READY` or
`ITEM_BACKEND_STATUS_TRAINING_UI_READY`.

### Owner fighter parameters

Some vanilla items read parameters out of a fighter's `vl.prc` instead of their
own `param.prc`. Steve's blocks are the clearest case: their lifetime and the
damage that breaks them live in Steve's fighter parameters. An item cloned from
one of those shares the value, so editing it would change the vanilla fighter
too.

These two calls give your item its own copy. Pass your item kind, the fighter
kind that owns the parameters, a byte offset into that fighter's parameter
payload, and the value. The offset has to be a multiple of 4 and below `0x4000`.

```rust
const PICKEL: i32 = 0x58;           // Steve, the fighter that owns the params
const LIFE: u32 = 0x518;            // param_pickelobject life, in frames
const AUTO_DAMAGE: u32 = 0x520;     // param_pickelobject auto_damage

clone_engine_api::item_owner_param_set_i32(MY_ITEM.raw(), PICKEL, LIFE, 600)?;
clone_engine_api::item_owner_param_set_f32(MY_ITEM.raw(), PICKEL, AUTO_DAMAGE, 0.0)?;
```

Register them once at startup. The engine applies the overrides only while your
item reads its parameters and puts the fighter's own values back afterwards, so
a vanilla copy of the same item, and the fighter itself, are unaffected.

Set every parameter that decides the same outcome, not just the obvious one. A
Steve block ends on whichever comes first, `life` frames or `auto_damage`
wearing it down, and an item spawned through `have_item` is always the weakest
material, so a longer `life` alone changes nothing.

| Function | Meaning |
|---|---|
| `item_owner_param_set_f32(kind, owner_fighter_kind, offset, value)` | Override one float in the owner fighter's parameters, for your item only. |
| `item_owner_param_set_i32(kind, owner_fighter_kind, offset, value)` | The same for an integer. |

One line per override appears in the log:

```
[itemclone] item_owner_param_set public=0x36b owner=88 +0x518 = 0x258 (1 override(s) for this item)
```

### The rest of the common row

`item/common/param/param.prc` holds 235 fields per item. `item_common_set`
covers its 161 floats; the other 74 are bools, ints, kind labels and hash40
names, and take the same treatment through three calls:

```rust
// a thrown copy vanishes on a shield instead of hopping off it
clone_engine_api::item_common_set_label(MY_ITEM.raw(), hash40("shield_kind"), hash40("item_shield_kind_lost"))?;
// lying on the floor, it launches when hit
clone_engine_api::item_common_set_label(MY_ITEM.raw(), hash40("hit_kind"), hash40("item_hit_kind_fly"))?;
// Kirby cannot swallow it
clone_engine_api::item_common_set_i32(MY_ITEM.raw(), hash40("eatable"), 0)?;
// the bone it is thrown from
clone_engine_api::item_common_set_hash(MY_ITEM.raw(), hash40("thrown_node"), hash40("have"))?;
```

The field names are the prc's (`have_kind`, `size_kind`, `hit_kind`,
`shield_kind`, `reflect_kind`, `flip_type`, `bound_flag`, `scale_type`,
`camera_kind`, `eatable`, `paintable`, `ai_pri`, `thrown_rot_kind`,
`thrown_node`, `clung_node`, `captured_motion` ...), and a kind field takes
either the label the prc shows for it (`item_shield_kind_lost`) or the raw
number. `item_common_has` answers for all of them. `group` is the one field
the game keeps outside its per-kind tables, so it cannot be set this way.

As with the floats, a value is in force while your item's own code runs and
the vanilla item keeps its own; a field the game reads by kind before an
item exists, such as the CPU's `ai_pri` or the drop weight's `trait_original`,
still shows the base's value at that moment.

The log names each registration and, once per table, the first swap:

```
[itemclone] item_common_set public=0x36b shield_kind (0xbd90c08b8) -> word +0x78 = 1 (label 0x1599e11ba6) (3 override(s) for this item)
[itemcommon] OVERRIDE public=0x36b base=0x40 word +0x78: 3 -> 1
```

### Spawning on its own

A registered item only appears when something spawns it: your fighter's code,
the Training menu, a `have_item`. The game's own drops come from generation
tables in `item/common/param/generate_param_item.prc`, and a custom kind is not
in them. One call adds it:

```rust
use clone_engine_api::ITEM_VARIATION_AUTO;

// weight 30 in the natural drops; most vanilla items sit at 20..50, capsules
// at 100, Assist Trophies at 150
clone_engine_api::item_generate_add(MY_ITEM.raw(), hash40("item_genid_random"), 30, 1, 1, ITEM_VARIATION_AUTO)?;
// and out of boxes, with the same weight against that box's own list
clone_engine_api::item_generate_add(MY_ITEM.raw(), hash40("item_kind_box"), 30, 1, 1, ITEM_VARIATION_AUTO)?;
```

`generator` is the hash40 of the table's `gen_id` label: `item_genid_random`
for the natural drops, or `item_kind_<name>` of the vanilla container that
should drop it (`box`, `barrel`, `capsule`, `carrierbox`, `kusudama`,
`sandbag`, `grass`). `per` is the weight, `min` and `max` how many appear at
once, `variation` normally `ITEM_VARIATION_AUTO`.

A plugin-less `item.toml` pack gets the same with three keys:

```toml
spawn_per  = 30                    # natural drops; leave it out for none
spawn_max  = 2                     # optional, with spawn_min; both default to 1
spawn_from = "box, barrel, capsule" # optional container drops, same weight
```

The entry follows the item switch of your **base** item. A custom kind has no
row in the item switch menu, so when the player turns your base off, your item
stops too, and when the base is banned on a stage, so is yours. There is no
way to give it a row of its own; the menu's 201 rows are fixed.

| Function | Meaning |
|---|---|
| `item_generate_add(kind, generator, per, min, max, variation)` | Put your item into one of the game's generation tables. |

Log lines, one per registration, one per table at every match start, and one
for each draw of your item:

```
[itemgen] generate_add public=0x36a (base 0x3f) in item_genid_random: per 30 count 1..1 variation -1; follows the base's item switch
[itemgen] match setup #1: item_genid_random (item table, record 0x...): 87 vanilla entries, 1 clone entry appended, 0 held back by the item switch
[itemgen] lot from item_genid_random: clone 0x36a drawn, handing the game base 0x3f variation -1 (ticket queued)
```

`held back by the item switch` is your item sitting out a match because its
base is switched off.

### Item families

`register_item_family`, `item_category`, `item_family_owner`,
`item_family_member_index`, `item_spawn_source` and `item_parent_kind` cover
Assist Trophies, Poké Ball Pokémon and bosses. They sit behind the
`research_item_families` feature, which a release build does not include, so the
published NRO returns `ERROR_BACKEND_UNAVAILABLE`. Build the engine yourself with
that feature to experiment with them.

## Stages

Most stage packs need no plugin. The engine reads `stage.toml` at boot and does
all of this for them. The calls are for a stage that needs runtime code, and a
plugin may sit beside a `stage.toml` because duplicate registration reuses the
existing stage.

| Function | Meaning |
|---|---|
| `stage_capacity()` | Remaining places and StageIDs, and whether minting is ready. |
| `allocate_stage(&StageAllocation)` | Mint a place and its normal, Omega and Battlefield forms. |
| `set_stage_behaviour(place, donor)` | Choose the vanilla stage class and gimmicks. |
| `set_stage_music(place, &StageMusic)` | Choose the My Music playlist, its column and the album selector. |
| `stage_id_for(place, form)` | The StageID of one form this session. |
| `register_stage(&StageRegistration)` | Publish its name, series, order and stage-select entry. |

Requires `CAP_STAGE_MINT`, `CAP_STAGE_CONFIG`, `CAP_STAGE_SELECT_EXTENDED` and
`CAP_STAGE_CSK`.

`set_stage_music` runs before `register_stage`, like `set_stage_behaviour`,
because the row is written once at registration. A `None` field takes the
behaviour donor's value. See [Stages](wiki/STAGES.md#music).

## Errors

Wrappers return `Error::Engine(code)`. The ones you are most likely to see:

| Code | Meaning |
|---|---|
| `ERROR_BACKEND_UNAVAILABLE` | That subsystem is not available in this build. |
| `ERROR_REGISTRATION_CLOSED` | You registered after the roster was built. |
| `ERROR_DUPLICATE` | The identity or kind is already taken. |
| `ERROR_NAMESPACE` | Effect or article namespace conflict. |
| `ERROR_ARTICLE_RESOURCE_CONFLICT` | Two minted articles were given one directory. |
| `ERROR_SMASHLINE_REQUIRED` | Retired. No longer returned; any Smashline build works. |
| `ERROR_HOOK_PREFLIGHT` | The live game code did not match what the hook expected. |
| `ERROR_HOOK_ABI` | That hook ABI is not supported. |
| `ERROR_ITEM_UI_METADATA` | UI metadata is malformed or does not match the item. |
| `ERROR_ITEM_UI_UNAVAILABLE` | That item UI layer is not available. |

Malformed names, null pointers, bad versions, invalid bases and exhausted
capacity have their own constants in `clone_engine_api`.

## C ABI

Rust plugins should use the wrappers above. Other languages can look up the
`clone_engine_*` exports directly; the Rust client is the authoritative list of
their exact names and signatures. Every registration structure is `#[repr(C)]`,
starts with `api_version` and `struct_size`, and keeps zeroed reserved fields so
it can grow without breaking older callers.
