# Getting started

Clone Engine gives your mod an identity of its own while it inherits behavior
from a vanilla base. It does not convert an existing moveset for you.

## Install the runtime

Clone Engine and its Smashline fork go in the global plugin directory:

```text
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/
```

A pack's own code goes inside that pack's ARCropolis mod directory as
`plugin.nro`, never in the global directory.

## Start from a template

- [Fighter](../../custom_fighters/template/)
- [Item](../../custom_items/template/)
- [Stage](../../custom_stages/template/)

Copy the whole template, then change every identity name in it before you add
content. Those names are permanent; the numbers the engine hands back are not,
so never write one into a file name or a config file.
