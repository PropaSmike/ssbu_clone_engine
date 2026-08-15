# Stages

A stage registration mints a place of its own plus normal, Omega and
Battlefield StageIDs, and adds it to the stage-select screen. Clone Engine
routes its models, motions, collisions, parameters, effects, sounds, cameras and
CSK row, and extends stage-select capacity and paging to fit it.

Most stage packs need no plugin. The engine reads `stage.toml` from the pack at
boot and does all of it. Start from
[`custom_stages/template`](../../custom_stages/template/).

## stage.toml

```toml
place = "template_stage"
display_name = "Template Stage"
id_name = "Template_Stage"
forms = ["normal", "omega", "battlefield"]
ships_battle_tree = false
series = "mario"
disp_order = 121
donor = "battlefield"
```

`place` is the permanent lowercase asset name. `id_name` supplies the message
and UI identifier, and its lowercase form must equal `place`.

Three separate donor jobs, which are easy to confuse:

| Key | Job |
|---|---|
| `donor` | The vanilla stage class and gimmick code your stage runs. |
| `resource_place` | Borrow another stage's whole asset tree at runtime. |
| `content_donor` | Fill in files your own tree is missing. |
| `content_donor_tree` | Whether that donor's `normal` or `battle` tree is used. |
| `carry_donor_scenery` | Include donor scenery files when converting. |

Omit what you do not need. A fully custom stage usually sets only a behavior
`donor`. Set `content_donor_tree` explicitly whenever you use `content_donor`:
left to chance, a partly filled directory can mix Normal and Battlefield members
of the same model.

## Forms

Leave `ships_battle_tree` false and the normal tree supplies every form. Set it
true only once `stage/<place>/battle` exists. A missing tree crashes the load;
it does not fall back.

Each form can have its own collision, parameters, models, camera, effects and
sounds, so test each one you enable.

## Moving platforms

A moving platform needs its LVD collision plus a matching dynamic-collision
entry. Clone Engine applies that entry only while your stage is running, so
other stages using the same behavior donor are unaffected. Use the exact
collision and bone names from your own stage files.
