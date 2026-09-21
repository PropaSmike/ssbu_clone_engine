# From a one-slot moveset

A moveset that lives on Mario's costumes 8 and 9 and a Clone Engine fighter
built on Mario are the same Smashline code. The difference: the clone is a
fighter of its own, so every "only on my costumes" check goes away, and the
things that used to be a `.prcxml` or a hardcoded number become a `Manifest`
in `main` (or a `fighter.toml` for a pack without code). Old habit on the
left, its replacement on the right.

## Registering

| One-slot | Clone Engine |
|---|---|
| nothing in `main` | `fighter!(WAWA, "wawa")` and `WAWA.register(Manifest::new("wawa", "mario")..)` first thing in `main` |
| `add_chara_db_entry_info` with `fighter_kind: hash40("fighter_kind_mario")`, `ui_chara_id: hash40("ui_chara_mario")` | the same call with `hash40("fighter_kind_wawa")` (also in `fighter_kind_corps`) and `hash40("ui_chara_wawa")`, plus `.own_css()` on the manifest |
| `ui_chara_db.prcxml` with `color_num` and a `msg_name.xmsbt` | `.name_label`, `.costumes`, `.series`, `.disp_order` on the manifest, if you have no CSK call; the engine adds the entry |

```rust
fighter!(WAWA, "wawa");

let manifest = Manifest::new(
    "wawa",   // name: files under fighter/wawa, Smashline agent name, identity
    "mario",  // base
)
.costumes(8)
.own_css();   // the pack's CSK call stays
let Ok(kind) = WAWA.register(manifest) else { return };
```

A pack with no plugin writes the same as `fighter.toml` beside `config.json`
(`name`, `base`, `costumes`, `css = false`).

## Scripts

| One-slot | Clone Engine |
|---|---|
| `Agent::new("mario").set_costume(vec![8, 9])` | `Agent::new("wawa")` |
| `if !is_my_slot(boma) { return original }` inside a script | nothing; the name is the check |
| `Agent::new("mario_fireball")` for an article | `Agent::new("wawa_fireball")` after `.article("fireball", "mario/fireball")` |
| `smashline::clone_weapon("mario", "fireball", ...)` | `.article("fireball", "mario/fireball")` on the manifest (`[[article]]` in `fighter.toml`) |
| `whitelist_kirby_copy_article` | `.kirby_article("fireball", "mario/fireball")` and `.kirby(N)` (`kirby = true` on the article, `[kirby] statuses = N`) |

```rust
Agent::new("wawa")
    .game_acmd("game_attack11", game_attack11, Priority::Default)
    .status(Main, *FIGHTER_STATUS_KIND_SPECIAL_N, special_n_main)
    .on_line(Main, wawa_frame)
    .install();
```

## Knowing who you are

| One-slot | Clone Engine |
|---|---|
| `utility::get_kind(boma) == *FIGHTER_KIND_MARIO && costume >= 8` | `WAWA.is(fighter)` |
| `owner_kind == *FIGHTER_KIND_MARIO` on a weapon | `WAWA.owns(weapon)` |
| `*FIGHTER_KIND_MARIO` as a number | `WAWA.kind()`, which has no value until the engine assigns one; never store it |

`utility::get_kind` on a clone answers the base; compare with `WAWA.is`.

Without `own_css()` both the engine and the mod write a select entry and CSK
keeps the last one. Every manifest method is in the
[API reference](../API.md#registration-by-code).

## Hooks on shared code

| One-slot | Clone Engine |
|---|---|
| `#[skyline::hook(offset = get_agent_virtual_function(*FIGHTER_KIND_MARIO, 46, false, false))]` with a costume check inside | `#[hook(slot = slot::fighter::ON_LINK_EVENT)]`; same slot, but written into the clone's own list, so no check |
| `Patch::in_text(get_agent_virtual_function(kind, 48, false, true)).data(my_fn)` | the same attribute; the clone has its own copy of Mario's list, so Mario keeps his |
| `#[skyline::hook(offset = 0x...)]` on code that is not a vtable entry | `#[hook(offset = 0x..., me = weapon)]`; the check is in the attribute |
| the same hook when nobody else will ever hook that address | `#[skyline::hook]` with `WAWA.is(...)` inside still works, but only one mod can have it |

```rust
#[hook(slot = slot::fighter::ON_LINK_EVENT)]
unsafe fn on_link_event(class: u64, object: *mut smash::app::BattleObject, event: u64) {
    call_original!(class, object, event)
}

#[hook(slot = slot::weapon::ON_ATTACK, article = "fireball")]
unsafe fn fireball_hit(class: u64, weapon: *mut smash::app::Weapon, log: u32) {
    call_original!(class, weapon, log)
}
```

The first parameter is the class object; the fighter or weapon comes second,
as in the one-slot signature. `get_agent_virtual_function` is in
`clone_engine_api::v2` with the tutorials' signature, and
`WAWA.set_vtable_entry(index, f)` writes an entry by hand.

## Parameters

| One-slot | Clone Engine |
|---|---|
| `param_config::update_float(*FIGHTER_KIND_MARIO, vec![8, 9], ..)` | `param_config::update_float(WAWA.kind().get().unwrap(), vec![-1], ..)` |
| `param_config::disable_kirby_copy(*FIGHTER_KIND_MARIO, vec![8, 9])` | the same with the clone's kind |
| a `vl.prc` under `fighter/mario/param/` for the costumes | `fighter/wawa/param/vl.prc`, the clone's own |
| `fighter_param` attributes, `param_motion`, `common` through a prcxml | `WAWA.param("weight").mul(1.05)`, or `[params]` in `fighter.toml` |

## Kirby

| One-slot | Clone Engine |
|---|---|
| `Agent::new("kirby").status(Main, MY_STATUS, ...)` with a number you chose | the same line with `WAWA.kirby_status(0)`; the engine chose the number |
| `fighter/kirby/model/copy_mario_cap/c08` | `fighter/kirby/model/copy_wawa_fitkirby/c00`, declared under `fighter/wawa/kirbycopy/c00` in `config.json`, plus `.kirby_motion(Motion::new(..))` for the copy animations |

## What does not change

The `smash_script` macros, `WorkModule`, `StatusModule`, `MotionModule`, the
`L2C*` types, `#[skyline::main]` and building with `cargo skyline build`.
