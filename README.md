# SSBU Clone Engine

Clone Engine lets a SSBU mod use its own fighter, article, item, or
stage identity instead of replacing a vanilla slot. A clone inherits 
behavior from a chosen base, while the engine keeps its files,
parameters, scripts, UI, articles, Kirby data, and runtime identity separate.

13.0.4 only

## Dependencies

- Smashline (clone engine fork) : <https://github.com/PropaSmike/smashline/tree/clone-engine-beta>
- ParamConfig (Not required but heavily recommended)
- The CSK Collection (For UI slots addition)
- NRO Hook
- ARCropolis & Skyline (Obviously)

Clone Engine refuses to activate its custom fighters
if it detects the Smashline build isn't the fork.

## Install

```text
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_clone_engine.nro
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libsmashline_plugin.nro
```

Install each content pack under "ultimate/mods/<pack name>".
Fighter and item plugins go in their own directories as usual as "plugin.nro".
Stage packs normally need no NRO: Clone Engine reads their "stage.toml" files
at boot. A stage uses "plugin.nro" only when it needs custom code.

## Fighters

Use ["custom_fighters/template"](custom_fighters/template/).

```rust
let fighter = clone_engine_api::CloneRegistration::new(
    clone_engine_api::KIND_AUTO,
    0,
    "ui_chara_my_fighter",
    "fighter_kind_my_fighter",
    "my_fighter",
    "mario",
);
let kind = clone_engine_api::allocate(&fighter)?;

smashline::Agent::new("my_fighter")
    .game_acmd("game_attack11", game_attack11, smashline::Priority::Low)
    .on_line(smashline::Main, fighter_frame)
    .install();
```

The int kind is assigned automatically. Never hardcode it
in assets or configuration; use names such as
"fighter_kind_my_fighter" and keep the value returned by "allocate".

## Templates

- [Fighter template](custom_fighters/template/): identity, Smashline,
  ParamConfig, CSK, assets, articles, Kirby, and shared hooks.
- [Simple item template](custom_items/template/): independent item
  identity, model, motion, local/common parameters, ACMD, status, and Training
  UI cell.
- [Stage template](custom_stages/template/): independent normal,
  Omega, and Battlefield forms, stage-select entry, parameters, collision,
  effects, sound, and camera resources.

## Features

- custom fighter identity and resource routing;
- Smashline agent-name bridge;
- CSS entries, colors, UI, effects, sounds, camera, and CPU AI routing;
- ParamConfig float, integer, slot, structured, and interaction bridges;
- independent fighter articles and Kirby copy support;
- simple custom items with private resources, parameters, animcmd, and paged
  Training item cells;
- clone-owned item status scripts, for any number of native base kinds;
- custom stage identity, forms, stage-select capacity/paging, collision/config bridges,
  and CSK stage rows;
- checked shared-hook arbitration.

Assist Trophy, Pokemon, and boss item families are available as research features behind
"research_item_families". They are not compiled into release.

## Build

```sh
cargo skyline build --release
```

Features prefixed with "research_", "diag_", or
"selftest_" are excluded from release builds.

## API implementation

```toml
[dependencies]
clone_engine_api = { git = "https://github.com/PropaSmike/ssbu_clone_engine", tag = "0.1.0-beta.1" }
```

## Documentation

- [Getting started](docs/wiki/GETTING_STARTED.md)
- [Fighters and Smashline](docs/wiki/FIGHTERS.md)
- [Parameters](docs/wiki/PARAMETERS.md)
- [Articles and Kirby](docs/wiki/ARTICLES_AND_KIRBY.md)
- [Items and Training UI](docs/wiki/ITEMS.md)
- [Stages](docs/wiki/STAGES.md)
- [Rust API reference](docs/API.md)

## Credits

Clone Engine exists thanks to the work and research of the Smash Ultimate modding community.
People aren't always the kindest to those who put in the hardest work, so please go show them
the love and respect they deserve.

**Runtime dependencies**
- [Skyline](https://github.com/skyline-dev/skyline) — shadowninja108, jam1garner, 3096, Raytwo, Genwald, blu-dev, jugeeya, Sammi-Husky
- [ARCropolis](https://github.com/Raytwo/arcropolis) — Raytwo, blu-dev, Coolsonickirby, jam1garner, jozz024, WuBoytH, itsmeft24, Genwald
- [Smashline](https://github.com/HDR-Development/smashline) — WuBoytH, blu-dev, FatherOfEgg, moklmaru, plyrthn, Moydow
- [NRO Hook](https://github.com/ultimate-research/nro-hook-plugin) — jam1garner, jugeeya, blu-dev
- [The CSK Collection](https://github.com/Coolsonickirby/the_csk_collection_api) — Coolsonickirby, zrksyd
- [ParamConfig](https://github.com/CSharpM7/lib_paramconfig) — CSharpM7, Coolsonickirby, theincredibleplayer

**Build dependencies**
- [skyline-rs](https://github.com/ultimate-research/skyline-rs) — jam1garner, Raytwo, jugeeya, blu-dev, WuBoytH, tech-ticks, Genwald, TheGreenPlanet
- [skyline-smash](https://github.com/ultimate-research/skyline-smash) — WuBoytH, blu-dev, jobrien97, jam1garner, jugeeya, Ayerbe-Dev, theincredibleplayer, FaultyPine
- [smash-script](https://github.com/WuBoytH/smash-script) — blu-dev, Claude-1308, Ayerbe-Dev, FaultyPine, WuBoytH
- [ninput](https://github.com/blu-dev/ninput) — blu-dev

The Smashline fork this engine requires is a fork of HDR-Development/Smashline
with one optional custom-kind bridge added. Upstream behavior is unchanged when
Clone Engine is absent.

## License

Clone Engine is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License version 3 as published by the Free
Software Foundation. It is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See [LICENSE](LICENSE) for the full text.
