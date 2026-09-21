# Fighters

## Declaring the fighter

In the plugin, before anything else:

```rust
use clone_engine_api::v2::{fighter, Manifest};

fighter!(MY_FIGHTER, "my_fighter");

let manifest = Manifest::new("my_fighter", "mario")
    .costumes(8)
    .own_css();
let Ok(kind) = MY_FIGHTER.register(manifest) else { return };
```

Or as a `fighter.toml` beside the pack's `config.json`, in which case the
plugin (if any) skips `register`:

```toml
name = "my_fighter"
base = "mario"
costumes = 8
css = false
```

Every key and method is in the [API reference](../API.md#fighters).

## Character select entry

Your CSK call stays as it is. Two fields name the clone:

```rust
add_chara_db_entry_info(CharacterDatabaseEntry {
    ui_chara_id: hash40("ui_chara_my_fighter"),
    clone_from_ui_chara_id: Some(hash40("ui_chara_mario")),
    fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),
    fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),
    ..
});
```

The full call is in the [API reference](../API.md#character-select-entry).
A pack with no plugin drops `css = false` and adds `display_name`, `series`,
`disp_order` to `fighter.toml`; the engine publishes the entry.

## Stats

ParamConfig's calls stay as they are, with `kind` where the base kind was:

```rust
param_config::update_float(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.5);
```

See [Stats and parameters](PARAMETERS.md).

## Scripts

Smashline scripts go under your own fighter name, never the base's:

```rust
smashline::Agent::new("my_fighter")
    .game_acmd("game_attack11", game_attack11, smashline::Priority::Default)
    .status(smashline::Exec, FIGHTER_STATUS_KIND_WAIT, wait_exec)
    .on_line(smashline::Main, fighter_frame)
    .install();
```

`MY_FIGHTER.is(x)` and `MY_FIGHTER.owns(x)` are the checks for a plain
`#[skyline::hook]`; Smashline scripts under `"my_fighter"` need none.

## Packs written before `fighter.toml`

They register with `CloneRegistration` and `allocate`, and check `is_kind`
in every callback. Still works: [Deprecated: the long form](../DEPRECATED.md).

## Final Smash backgrounds

The base fighter's background is used unless you ship your own at
`fighter/<your name>/finalsmash/shared/`, with the base's layout. If it comes
up black, the log says what was looked for and found:

```text
[fsload] kind 20 (falco) finalsmash directory Some(15204), 2 of 2 model probes resolved, 0 missing
```
