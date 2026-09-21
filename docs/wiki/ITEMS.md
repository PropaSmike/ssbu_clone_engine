# Items

A custom item gets its own model, animations, `param.prc`, scripts,
statuses, shared-file settings and Training menu cell. Effects and sounds
come from the base item's banks. Start from
[custom_items/template_v2](../../custom_items/template_v2/).

## Declaring the item

In the plugin, before anything else:

```rust
use clone_engine_api::v2::{item, ItemManifest};
use clone_engine_api::ItemStatusLine;

item!(WAWA, "wawa");

#[skyline::main(name = "wawa")]
pub fn main() {
    let manifest = ItemManifest::new(
        "wawa", // resource name: files under item/wawa, script name
        63,     // base kind: the vanilla item it is built on (63 = Killing Edge)
    )
    .base_item("killsword")           // the base's name, for the log
    .spawn_per(30)                    // natural drop weight; most vanilla items are 20 to 50
    .spawn_from(&["box", "barrel"])   // containers that can hold it
    .common("throw_speed_mul", 0.75); // a field of item/common/param/param.prc, for this item only
    let Ok(kind) = WAWA.register(manifest) else { return };

    WAWA.status(ItemStatusLine::Init, "THROW", throw_init); // status line, a status name of the base item, function
}
```

Or as an `item.toml` beside the pack's `config.json`, in which case the
plugin (if any) skips `register`:

```toml
base_kind     = 63                  # the vanilla item you build on (63 = Killing Edge)
resource_name = "wawa"              # your files live under item/wawa
base_item     = "killsword"         # optional: names the base in the log
spawn_per  = 30                     # optional: natural drop weight
spawn_max  = 2                      # optional, with spawn_min; both default to 1
spawn_from = "box, barrel, capsule" # optional: containers that can hold it

[common]
throw_speed_mul = 0.75
life = 300                          # a whole number is an integer field
```

Optional keys: `agent_name` (script name, defaults to `resource_name`),
`ui_id` (defaults to `ui_item_<resource_name>`), `training_order`. Drops
follow the item switch of your base item: when the player turns the base off,
your item is off too. Several items in one pack are `[[item]]` blocks with
the same keys (engines after `0.2.1-beta.1`).

Statuses are registered at startup, by name; they are the base item's and the
lines are `Setting`, `JointSrt`, `Init`, `Update`, `Coroutine`, `Exit`.

To spawn or give the item from code:

```rust
ItemModule::have_item(
    boma,                                             // the fighter that gets the item
    smash::app::ItemKind(WAWA.kind().get().unwrap()), // the clone's kind, wrapped in the game's type
    0, 0, false, false,                               // the game's own arguments, as for a vanilla item
);
```

Packs written before `item.toml` register the item in code with
`allocate_item`; that still works and is in
[Deprecated: the long form](../DEPRECATED.md#items).

## An item whose settings live on a fighter

Some items (Steve's blocks, base kind `0x1ae`) read their settings from a
fighter's `vl.prc`. `owner_param` gives your item its own values, by fighter
and field name, without touching the fighter:

```rust
ItemManifest::new("wawa_block", 0x1ae)
    .owner_param("pickel", "life", 600.0)       // owner fighter (Steve), a field of its vl.prc, value
    .owner_param("pickel", "auto_damage", 0.0)
```

```toml
[owner_params]
pickel.life = 600
pickel.auto_damage = 0
```

Set every field that decides the same outcome: a block ends on `life` or
`auto_damage`, whichever comes first.

[custom_items/template_v2](../../custom_items/template_v2/) does exactly
this in code with `owner_param("pickel", "life").set(600.0)`.

## File layout

```text
item/wawa/model/body/c00/...
item/wawa/motion/body/c00/motion_list.bin
item/wawa/param/param.prc
item/wawa/script/animcmd/body/game.lc
item/wawa/script/animcmd/body/effect.lc
item/wawa/script/animcmd/body/sound.lc
```

In `config.json`, key every `new-dir-files` group by the folder that directly
holds the files (`item/wawa/model/body/c00`), and keep the model folders out
of `new-dir-infos-base`. Otherwise the files never load and the log says
nothing.

## Boot log

```text
[itempack] 2 item.toml pack(s) under sd:/ultimate/mods
[itempack] Wawa Item: public=0x36b base=63 (killsword) resource=wawa ui=ui_item_wawa ui_result=0
```

No line at all means no `item.toml` was found. A `REFUSED` line names the
reason. The `public=` number is the kind this boot; it changes when other packs
are installed, and nothing you ship should depend on it.
