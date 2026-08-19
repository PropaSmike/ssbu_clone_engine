# Items and the Training menu

A custom item gets its own kind and its own:

- model, textures, motion and `param.prc`;
- game, effect and sound animcmd;
- statuses, layered over the base item's;
- common float parameters, changed for your item only;
- Training menu cell, with a name, help text and optional icon.

A custom item's effects and sounds come from its base item's banks. Start from
[`custom_items/template`](../../custom_items/template/).

## A content-only pack needs no plugin

Put an `item.toml` beside the pack's `config.json` and the engine registers it
at boot. Two keys are the minimum:

```toml
base_kind     = 63      # the vanilla item you build on (63 = Killing Edge)
resource_name = "wawa"  # your files live under item/wawa
```

The optional keys are `base_item`, which names the base in the log, `agent_name`,
which defaults to `resource_name`, `ui_id`, which defaults to
`ui_item_<resource_name>`, and `training_order`.

## A plugin does not pick a number either

Pass `KIND_AUTO` to `allocate_item`. It registers the item and returns the kind
the engine assigned, stepping over anything another pack already took. Hold it
in a `CloneItemKind`, which caches the number and can recover it later from the
resource name.

```rust
use clone_engine_api::{CloneItemKind, ItemCloneRegistration, KIND_AUTO};

static WAWA: CloneItemKind = CloneItemKind::new("wawa");

let kind = clone_engine_api::allocate_item(&ItemCloneRegistration::new(
    KIND_AUTO,
    63,
    "wawa",
    "wawa",
))?;
WAWA.store(kind);
```

Everything that wants a kind takes `WAWA.raw()`, including the `ItemModule`
calls that spawn or attach the item. Those take the game's own
`smash::app::ItemKind` wrapper around the number:

```rust
ItemModule::have_item(boma, smash::app::ItemKind(WAWA.raw()), 0, 0, false, false);
```

`register_item` with a number you chose still works, and a pack written before
`allocate_item` keeps running. It is the worse option: the number has to be at
least `FIRST_CUSTOM_ITEM_KIND` (0x36A) and unique across every custom item the
user has installed, and neither you nor the engine can enforce that for a
number two packs hardcoded.

## File layout

```text
item/wawa/model/body/c00/...
item/wawa/motion/body/c00/motion_list.bin
item/wawa/param/param.prc
item/wawa/script/animcmd/body/game.lc
item/wawa/script/animcmd/body/effect.lc
item/wawa/script/animcmd/body/sound.lc
```

Items have no aggregate directory. Every `new-dir-files` group must therefore be
keyed by the leaf directory that holds the files, and the model directories must
not appear in `new-dir-infos-base`. If you key a group on a parent directory
instead, the files never load and nothing in the log tells you why.

## Boot log

The engine prints one line per pack:

```
[itempack] 2 item.toml pack(s) under sd:/ultimate/mods
[itempack] Wawa Item: public=0x36b base=63 (killsword) resource=wawa ui=ui_item_wawa ui_result=0
```

No line at all means no `item.toml` was found. A `REFUSED` line names the
reason. A kind another pack already took is stepped over automatically, so the
number in that line changes when the user installs something else. Nothing you
ship should depend on it.
