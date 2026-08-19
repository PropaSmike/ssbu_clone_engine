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
| `smashline_bridge_version()`, `smashline_compatible()` | The Smashline fork it found. |
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

ACMD, statuses, OPFF, lifecycle callbacks and weapon ACMD all belong to the
Smashline fork, registered under your own agent name:

```rust
smashline::Agent::new("wawa")
    .game_acmd("game_attack11", attack11, smashline::Priority::Low)
    .status(smashline::Pre, *FIGHTER_STATUS_KIND_SPECIAL_N, special_n_pre)
    .on_line(smashline::Main, fighter_frame)
    .install();
```

Registering on the base name instead would change the vanilla fighter and every
other mod built on it.

Check `smashline_compatible()` before allocating. The engine refuses custom
fighters when the fork is missing or the wrong version.

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
your fighter, with its own files, and the Smashline fork registers its ACMD
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
| `stage_id_for(place, form)` | The StageID of one form this session. |
| `register_stage(&StageRegistration)` | Publish its name, series, order and stage-select entry. |

Requires `CAP_STAGE_MINT`, `CAP_STAGE_CONFIG`, `CAP_STAGE_SELECT_EXTENDED` and
`CAP_STAGE_CSK`.

## Errors

Wrappers return `Error::Engine(code)`. The ones you are most likely to see:

| Code | Meaning |
|---|---|
| `ERROR_BACKEND_UNAVAILABLE` | That subsystem is not available in this build. |
| `ERROR_REGISTRATION_CLOSED` | You registered after the roster was built. |
| `ERROR_DUPLICATE` | The identity or kind is already taken. |
| `ERROR_NAMESPACE` | Effect or article namespace conflict. |
| `ERROR_ARTICLE_RESOURCE_CONFLICT` | Two minted articles were given one directory. |
| `ERROR_SMASHLINE_REQUIRED` | The Smashline fork is missing or incompatible. |
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
