# Stats and parameters

## Which syntax

| Values | Syntax |
|---|---|
| `vl.prc` (`param_special_*`, `param_private`, `fly_data`, ...) | ParamConfig |
| `fighter_param.prc` attributes (`walk_speed_max`, `jump_y`, `scale`, `weight`, ...) | engine |
| `fighter_param_motion.prc` (`param_motion`), the `fighter/common/param/` files (`common`), throw positions (`param_thrown`) | engine |
| lists and arrays in `vl.prc` (`hit_data`, ...) | your own `vl.prc` |
| lists in the common files, `battle_object.prc`, `spirits.prc` | cannot be changed per clone |

The engine ones are read by the game past ParamConfig's hooks (through
`FighterParamAccessor2` or straight out of the row), so a ParamConfig entry
only half applies; the engine call also pushes to ParamConfig.

## ParamConfig

Its calls work unchanged with the clone's kind. `vec![-1]` is every costume.

```rust
let kind = MY_FIGHTER.kind().get().unwrap();

param_config::update_float(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.5);
param_config::update_attribute_mul(kind, vec![3], (hash40("param_special_n"), hash40("fireball_speed_mul")), 1.15); // costume 3 only
param_config::disable_kirby_copy(kind, vec![-1]);
```

## Engine

```rust
MY_FIGHTER.param("walk_speed_max").set(1.2);
MY_FIGHTER.param("param_motion").sub("flip").int(0);
MY_FIGHTER.param("common").sub("shield_max").set(20.0);
MY_FIGHTER.param("param_thrown").sub("held_offset").mul(1.2);
```

or, with no code, in `fighter.toml`:

```toml
[params]
walk_speed_max = 1.2
param_motion.flip = 0     # a whole number sets an integer
common.shield_max = 20.0

[params.mul]
weight = 1.05

[params.c03]
param_special_hi.y_spd_air = 2.0   # costume 3 only
```

The keys are in the [API reference](../API.md#parameters). ParamConfig must
be installed either way.

## Your own parameter files

Ship `fighter/<your name>/param/vl.prc` and it is used instead of the
base's. The camera and CPU parameter trees are all or nothing: set
`owns_param_resources` only once the whole tree is in your folder; a partial
tree crashes or loads forever.
