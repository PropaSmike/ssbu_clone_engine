# Fighters and Smashline

## Who does what

The Clone Engine Smashline fork registers fighter ACMD, statuses, OPFF,
lifecycle callbacks and weapon ACMD. Clone Engine registers the identity and
routes files, parameters, articles, Kirby data and ownership to it.

Register your scripts on your own agent name, never the base fighter's:

```rust
smashline::Agent::new("my_fighter")
    .game_acmd("game_attack11", game_attack11, smashline::Priority::Low)
    .status(smashline::Exec, FIGHTER_STATUS_KIND_WAIT, wait_exec)
    .on_line(smashline::Main, fighter_frame)
    .install();
```

## Identity

Build a `CloneRegistration` with `KIND_AUTO`, the vanilla base kind, your
`ui_chara_...` and `fighter_kind_...` names, your resource name, the base
resource name and your real color range. `allocate` returns the kind for this
installation.

Keep it in a `CloneKind`, which stores the number against your permanent name:

```rust
static KIND: clone_engine_api::CloneKind =
    clone_engine_api::CloneKind::new("fighter_kind_my_fighter");

KIND.store(clone_engine_api::allocate(&descriptor)?);

let Some(kind) = KIND.get() else { return false };
```

Then gate your callbacks on it: `is_kind` for the fighter, `is_owned_by_kind`
for weapons and articles. A callback that runs without a gate runs for the
vanilla base too.

## Order of registration

[`custom_fighters/template`](../../custom_fighters/template/) does this in
order, and the order matters:

1. capability and Smashline bridge checks;
2. Clone Engine identity allocation;
3. Smashline ACMD, status, OPFF and lifecycle registration;
4. ParamConfig and article registration;
5. optional Kirby and shared-hook registration;
6. the ARCropolis mount callback;
7. CSK publication.

Publishing the CSS row before the descriptor and the files exist gives the
player a row that selects a fighter the game cannot load.
