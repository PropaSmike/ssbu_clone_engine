# Getting started

Converting a moveset that sits on a vanilla slot:
[From a one-slot moveset](FROM_A_ONE_SLOT_MOVESET.md).

## 1. Install the engine

The engine goes in the global plugin folder:

```text
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_clone_engine.nro
```

A pack's own code goes inside that pack's mod folder as `plugin.nro`, never in
the global folder. A pack made of files only has no `plugin.nro` at all.

## 2. Copy a template

- [Fighter](../../custom_fighters/template_v2/)
- [Item](../../custom_items/template_v2/)
- [Stage](../../custom_stages/template/)

Copy the whole template folder, then change every name in it before you add
your own content: the name in `fighter!(..)` and `Manifest::new(..)` (or in
`fighter.toml` for a pack without code), the folder names under `fighter/`,
`item/` or `stage/`, and the Smashline agent names.

## 3. Names are permanent, numbers are not

The kinds and ids the engine hands out change with the player's other mods.
Never write one into a file name or a config file.

## 4. Read the log

The engine reports what it did in Skyline's log. Look for lines starting with
`[fighterpack]`, `[itempack]`, `[stagepack]` and `[clone_engine]`. A line
saying `refused` or `REFUSED` names the reason a pack did not load.
