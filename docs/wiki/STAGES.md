# Stages

Put a `stage.toml` beside the pack's `config.json`. No code is needed. Start
from [custom_stages/template](../../custom_stages/template/).

## stage.toml

```toml
place = "template_stage"          # lowercase folder name under stage/
display_name = "Template Stage"   # name on the stage select screen
id_name = "Template_Stage"        # the message and UI id; lowercased it must equal place
forms = ["normal", "omega", "battlefield"] # which forms exist
ships_battle_tree = false         # true only once stage/<place>/battle exists
series = "mario"                  # series icon
disp_order = 121                  # position on the stage select screen
donor = "battlefield"             # the vanilla stage whose code and gimmicks run
bgm = "smashbtl"                  # My Music playlist
bgm_setting_no = 0                # its column, 0 to 15
```

| Key | What it does |
|---|---|
| `donor` | Your stage runs this vanilla stage's code (moving parts, hazards) |
| `resource_place` | Your stage uses this vanilla stage's whole file tree |

`content_donor`, `content_donor_tree` and `carry_donor_scenery` are Clone
Pack Workbench keys, ignored by the engine. Always set `content_donor_tree`
with `content_donor`.

## Several stages in one pack

```toml
[[stage]]
place = "sector_z"
id_name = "Sector_Z"
donor = "fox_corneria"

[[stage]]
place = "badtime"
id_name = "Badtime"
donor = "pictochat2"
```

Each stage keeps its own `stage/<place>` folder and sound files, and the
pack's one `config.json` lists all of them. Engines up to `0.2.1-beta.1` read
only the single-stage form and load nothing from a file with blocks.

## Music

```toml
bgm = "demon"           # a playlist, by series name or full label ("bgmdemon" is the same)
bgm_setting_no = 0      # which column of the playlist starts, 0 to 15
bgm_selector = false    # true adds the album selector
```

Most playlists only fill column 0; the boot log warns when the column is
empty. Left out, the stage uses its `donor`'s playlist; with no `donor`
either, My Music is empty.

For a track list of your own, publish a playlist with `playlist_entries` in
the CSK Collection's JSON under `sd:/ultimate/mods/<pack>/database/` and point
`bgm` at it. Do not put `stage_database_entries` in that JSON; the engine
writes the stage row itself.

## Forms

With `ships_battle_tree = false`, the normal folder supplies every form. Set
it true only once `stage/<place>/battle` exists. A missing folder crashes the
load; it does not fall back.

Each form can have its own collision, settings, models, camera, effects and
sounds, so test every form you enable.

## Moving platforms

A moving platform needs its collision in the `.lvd` file plus a matching
dynamic collision entry. The engine applies that entry only while your stage
is running, so other stages with the same `donor` are unaffected. Use the
exact collision and bone names from your own stage files.
