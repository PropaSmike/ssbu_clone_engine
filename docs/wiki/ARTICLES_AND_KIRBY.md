# Articles and Kirby

## Articles

Declare one in the manifest:

```rust
Manifest::new("my_fighter", "samus")
    .article(
        "beam",        // folder fighter/my_fighter/model/beam, scripts under "my_fighter_beam"
        "samus/cshot", // the vanilla article it is copied from: fighter/weapon
    )
```

or in `fighter.toml`:

```toml
[[article]]
name = "beam"          # files under fighter/my_fighter/model/beam
from = "samus/cshot"   # the vanilla article it is copied from
```

Its scripts go under the name `<fighter>_<article>`:

```rust
smashline::Agent::new("my_fighter_beam")                              // the article's own name
    .game_acmd("game_fly", game_fly, smashline::Priority::Default)     // script name, function, priority
    .install();

MY_FIGHTER.article("beam")
    .status(Line::Main, *WEAPON_SAMUS_CSHOT_STATUS_KIND_FLY, beam_main) // status line, a status kind of the source weapon, function
    .install();
```

Each article needs a folder of its own. Read
`MY_FIGHTER.article("beam").index()` right before you use it, never earlier,
and never fall back to 0. `MY_FIGHTER.article("beam").spawn(boma)` does both.

## Articles with collision

An article with an `.lvd` file keeps the base's file name inside your folder:

```text
fighter/<your fighter>/model/<your article>/c00/<base article name>.lvd
```

Renamed, it loads with no collision.

A costume folder without its own `.lvd` uses the one from the nearest costume
that has one. Several articles made from the same base article each use their
own file.

## Kirby copies

Reserve the status numbers the copied move needs:

```rust
Manifest::new("my_fighter", "mario")
    .kirby(1)          // how many status numbers the copied move needs
```

```toml
[kirby]
statuses = 1
```

Read them back with `kirby_status(n)`, register the scripts on Kirby, then
arm:

```rust
smashline::Agent::new("kirby")                                  // Kirby's copy scripts go on Kirby
    .status(Main, MY_FIGHTER.kirby_status(0), kirby_special_n)  // status line, the first reserved number, function
    .install();
MY_FIGHTER.arm_kirby();                                         // after the statuses are installed
```

`arm_kirby` must come after the statuses are installed.

### Hooks that run on Kirby

Your fighter's `slot` and `common_slot` hooks never run for the copy: the
object playing it is Kirby's, a vanilla kind. `copy_slot` registers the entry
Kirby runs while he holds your ability.

```rust
#[hook(copy_slot = slot::common::SYS_LINE_STATUS_END_CONTROL)]
unsafe fn kirby_status_end(agent: u64) -> L2CValue {  // agent is KIRBY's agent, not yours
    call_original!(agent)                             // Kirby's own entry, also when no copy is held
}
```

Slots 10 to 14 only; 10 and 12 return an `L2CValue`, 11, 13 and 14 a `u64`.
Two Kirbys holding two different copies each get their own. Slot 10 runs once as Kirby's agent starts,
before he holds any copy, so a copy hook there never fires; slot 12 runs at the
end of every status. See [Common vtables](../API.md#common-vtables).

The copy model, hat or full body, goes in
`fighter/kirby/model/copy_<your name>_fitkirby/cNN/`, and the pack's
`config.json` declares the copy files under `fighter/<your name>/kirbycopy/cNN`.
Do not create empty `kirbycopy/cNN` groups and do not fill them with the base
fighter's Kirby files; both make the game's file cache fail.

`.kirby_full_model()` (`full_model = true`) gives Kirby your whole body
instead of a hat. It needs a complete model and animation set for every Kirby
colour.

### More than one model

`.kirby_model("my_fighter_kirby_extra")` (`model = "..."` in `[kirby]`)
declares one extra model folder under `fighter/kirby/model/`. Anything in a
model you did not declare is never loaded.

### Your own copy animations

Only the animations you change; the rest come from the base fighter:

```rust
use clone_engine_api::v2::Motion;

Manifest::new("my_fighter", "mario")
    .kirby_motion(
        Motion::new(
            "my_fighter_special_n",         // motion name; must start with your name
            "myfighterd00specialn.nuanmb",  // animation file
        )
        .template("mario_special_n")        // borrow flags and scripts from this copy motion
        .scripts("myfighterspecialn"),      // game_, sound_, effect_, expression_ scripts, on Agent::new("kirby")
    )
```

```toml
[[kirby.motion]]
name = "my_fighter_special_n"
animation = "myfighterd00specialn.nuanmb"
template = "mario_special_n"
scripts = "myfighterspecialn"
```

The animation file goes in `fighter/kirby/motion/<your name>body/c00/` and is
listed in `new-dir-files` under your `kirbycopy/cNN/bodymotion` group. A
motion with a template and no scripts keeps the template's hitboxes.

### Hiding a hat piece from the start

```rust
Manifest::new("my_fighter", "mario")
    .kirby_mesh("wing", false)   // mesh name, visible at the start
```

```toml
[[kirby.mesh]]
name = "wing"
visible = false
```

Your animations still control the mesh once they play.
