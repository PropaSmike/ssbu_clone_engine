# Articles and Kirby

## Articles of your own

`clone_article` copies a vanilla weapon into a new weapon kind that belongs to
your fighter. Clone Engine owns its identity, files, ownership and weapon
statuses; the Smashline fork registers its ACMD under its own agent name. Each
article needs a file directory name of its own.

Keep the `ArticleHandle` you get back. Its weapon kind is permanent. Its
article-table index is not, because other mods can add articles and move it, so
resolve the index immediately before you use it and never fall back to zero.

Weapon callbacks belong to the owner, so gate them with `is_owned_by_kind`.

## Kirby copies

Your fighter can ship its own Kirby copy status scripts and either an ordinary
hat or a full-body copy. Clone Engine creates the copy record, routes the model,
motion and article files, and sends only the copy of your fighter to your
scripts.

The fighter template has a working example.
