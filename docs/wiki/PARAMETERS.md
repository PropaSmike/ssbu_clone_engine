# Fighter parameters

ParamConfig applies the values. Clone Engine hands it your allocated kind, which
is the part ParamConfig cannot work out on its own for a custom fighter. See the parameter section of the [API guide](../API.md).

Files your fighter owns outright, such as `vl.prc`, `expression_vl.prc` and the
camera and AI parameter trees, can live in your own folders instead. Set
`FLAG_OWNS_PARAM_RESOURCES` only once the complete tree is there. A partial tree
does not fall back to the base; it crashes or loads forever.
