# SSBU Clone Engine

Clone Engine lets a Smash Ultimate mod add a fighter, item or stage of its
own, built on a vanilla one, instead of replacing a slot.

Game version 13.0.4 only.

## What you need

- Skyline and ARCropolis, the usual mod loaders
- Smashline
- NRO Hook
- The CSK Collection, which adds the character and stage select entries
- ParamConfig, optional but recommended, for changing fighter stats

## Install

Put the engine in the global plugin folder of your SD card:

```text
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_clone_engine.nro
```

Smashline (`libsmashline_plugin.nro`) goes in the same folder.

Every mod pack goes in its own folder under `ultimate/mods/`. If a pack has
code, that code is a `plugin.nro` inside the pack's own folder, never in the
global plugin folder.

## A fighter

The plugin registers the fighter, then keeps its CSK, ParamConfig and
Smashline calls as they were; only the names change:

```rust
use clone_engine_api::v2::{fighter, Manifest};

fighter!(MY_FIGHTER, "my_fighter"); // the handle, and the identity

#[skyline::main(name = "my_fighter")]
pub fn main() {
    let manifest = Manifest::new(
        "my_fighter", // name: files under fighter/my_fighter, Smashline agent name
        "mario",      // base: the vanilla fighter it behaves like
    )
    .costumes(8)
    .own_css();       // the CSK call below publishes the select entry
    let Ok(kind) = MY_FIGHTER.register(manifest) else { return };

    add_chara_db_entry_info(CharacterDatabaseEntry {
        ui_chara_id: hash40("ui_chara_my_fighter"),                                 // ui_chara_<name>
        fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),      // fighter_kind_<name>
        fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),
        ..
    });
    param_config::update_float(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.5); // the clone's kind, every costume
    smashline::Agent::new("my_fighter")                                              // scripts under the clone's own name
        .game_acmd("game_attack11", game_attack11, smashline::Priority::Default)
        .on_line(smashline::Main, fighter_frame)
        .install();
}
```

A pack with no code declares the same thing in a `fighter.toml` next to its
`config.json`, and the engine publishes the select entry:

```toml
name = "my_fighter"        # your files live under fighter/my_fighter
base = "mario"             # the vanilla fighter it behaves like
display_name = "My Fighter"
costumes = 8
series = "mario"
```

Start from [custom_fighters/template_v2](custom_fighters/template_v2/).
Packs written before `fighter.toml`, where the plugin registers the fighter
itself, keep working; that form is documented in
[docs/DEPRECATED.md](docs/DEPRECATED.md).

## Numbers are not yours to pick

The engine assigns every kind and id at boot, and they change with the
player's other mods. Use your name (`my_fighter`) everywhere; never write a
number into a file name or a config.

## Clone Pack Workbench

[Clone Pack Workbench](https://github.com/PropaSmike/clone_pack_workbench)
builds the pack itself: it writes `config.json`, `fighter.toml`, `item.toml`
and `stage.toml`, and checks a pack before you install it. Start there for a
pack without code.

## Templates

| Template | What it shows |
|---|---|
| [Fighter](custom_fighters/template_v2/) | Registration by code, the CSK select entry, ParamConfig and engine stats, a Smashline moveset, an article, Kirby copy statuses and a hook |
| [Item](custom_items/template_v2/) | Registration by code, a status, owner parameters, a vtable hook |
| [Stage](custom_stages/template/) | A stage with normal, Omega and Battlefield forms and a stage select entry |

The deprecated long-form templates, where every registration is written in
code, are [custom_fighters/template](custom_fighters/template/),
[custom_items/template](custom_items/template/) and
[custom_items/fighter_owned_template](custom_items/fighter_owned_template/).

## What the engine handles

- fighters: identity, files, character select entry, colours, effects,
  sounds, camera, CPU behaviour, stats, Final Smash backgrounds
- articles (a fighter's projectiles and objects), with Kirby copy support
- items: files, settings, scripts, statuses, Training menu cell, drops
- stages: identity, forms, stage select entry and paging, collision,
  settings, music
- several packs changing the same game function at once

Assist Trophies, Pokemon and bosses exist only as a research feature
(`research_item_families`) and are not in the released engine.

## Documentation

- [Getting started](docs/wiki/GETTING_STARTED.md)
- [Fighters](docs/wiki/FIGHTERS.md)
- [From a one-slot moveset](docs/wiki/FROM_A_ONE_SLOT_MOVESET.md)
- [Stats and parameters](docs/wiki/PARAMETERS.md)
- [Articles and Kirby](docs/wiki/ARTICLES_AND_KIRBY.md)
- [Items](docs/wiki/ITEMS.md)
- [Stages](docs/wiki/STAGES.md)
- [API reference](docs/API.md), for packs with code
- [Deprecated: the long form](docs/DEPRECATED.md), for packs written before `fighter.toml`

## Using the API in your plugin

```toml
[dependencies]
clone_engine_api = { git = "https://github.com/PropaSmike/ssbu_clone_engine", tag = "0.2.1-beta.1" }
```

Use the tag of the engine release you installed.

## Building the engine

```sh
cargo skyline build --release
```

Features named `research_*`, `diag_*` or `selftest_*` are left out of release
builds.

## Credits

Clone Engine exists thanks to the work and research of the Smash Ultimate
modding community. People are not always kind to those who put in the hardest
work, so please show them the respect they deserve.

**Runtime dependencies**
- [Skyline](https://github.com/skyline-dev/skyline): shadowninja108, jam1garner, 3096, Raytwo, Genwald, blu-dev, jugeeya, Sammi-Husky
- [ARCropolis](https://github.com/Raytwo/arcropolis): Raytwo, blu-dev, Coolsonickirby, jam1garner, jozz024, WuBoytH, itsmeft24, Genwald
- [Smashline](https://github.com/HDR-Development/smashline): WuBoytH, blu-dev, FatherOfEgg, moklmaru, plyrthn, Moydow
- [NRO Hook](https://github.com/ultimate-research/nro-hook-plugin): jam1garner, jugeeya, blu-dev
- [The CSK Collection](https://github.com/Coolsonickirby/the_csk_collection_api): Coolsonickirby, zrksyd
- [ParamConfig](https://github.com/CSharpM7/lib_paramconfig): CSharpM7, Coolsonickirby, theincredibleplayer

**Build dependencies**
- [skyline-rs](https://github.com/ultimate-research/skyline-rs): jam1garner, Raytwo, jugeeya, blu-dev, WuBoytH, tech-ticks, Genwald, TheGreenPlanet
- [skyline-smash](https://github.com/ultimate-research/skyline-smash): WuBoytH, blu-dev, jobrien97, jam1garner, jugeeya, Ayerbe-Dev, theincredibleplayer, FaultyPine
- [smash-script](https://github.com/WuBoytH/smash-script): blu-dev, Claude-1308, Ayerbe-Dev, FaultyPine, WuBoytH
- [ninput](https://github.com/blu-dev/ninput): blu-dev

## License

Clone Engine is free software under the GNU General Public License version 3.
It comes with no warranty. See [LICENSE](LICENSE).
