# Articles and Kirby

## Articles of your own

`clone_article` copies a vanilla weapon into a new weapon kind that belongs to
your fighter. Clone Engine owns its identity, files, ownership and weapon
statuses; Smashline registers its ACMD under its own agent name. Each
article needs a file directory name of its own.

Keep the `ArticleHandle` you get back. Its weapon kind is permanent. Its
article-table index is not, because other mods can add articles and move it, so
resolve the index immediately before you use it and never fall back to zero.

Weapon callbacks belong to the owner, so gate them with `is_owned_by_kind`.

## Articles with collision

Four vanilla articles carry an `.lvd` file for their collision. Cloning one needs
no extra registration, but the file name is not yours to change: put it in your
own article directory under the base article's original file name.

```text
fighter/<your fighter>/model/<your article name>/c00/<base article name>.lvd
```

Rename the file to match your directory and the article loads with no collision
at all.

## Kirby copies

Your fighter can ship its own Kirby copy status scripts and either an ordinary
hat or a full-body copy. Clone Engine creates the copy record, routes the model,
motion and article files, and sends only the copy of your fighter to your
scripts.

The fighter template has a working example.

The copy gets one model. If your hat is several pieces, or its meshes and bones
are split up, declare the extra models with `clone_copy_model`, up to three.
Anything in a model you did not declare is never loaded.

## Copy motions

The copy animates from the base fighter's copy animations. `clone_copy_motion`
adds animations of your own beside them, so you only ship files for the motions
you change.

The animation goes in `fighter/kirby/motion/<your resource name>body/c00/`,
listed in `new-dir-files` under your `kirbycopy/cNN/bodymotion` group, and the
motion name starts with your resource name so two packs cannot collide. Its ACMD
belongs on `Agent::new("kirby")`.

The fighter template registers two and ships no animation, because those are
yours to make. The builder is in the Kirby section of the [API guide](../API.md).
