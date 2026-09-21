//! Stats. ParamConfig's own calls for `vl.prc` values (the game reads them
//! through the getters ParamConfig hooks); the engine's `param()` for the
//! values the game reads past those getters: `fighter_param.prc` attributes,
//! `fighter_param_motion.prc`, the `fighter/common/param/` files and throw
//! positions. `param()` pushes `set` values to ParamConfig as well.

use crate::smash;
use crate::TEMPLATE;
use smash::hash40;

pub fn install(kind: i32) {
    let last = crate::COSTUMES as i32 - 1;
    let others: Vec<i32> = (0..last).collect(); // every costume but the last; vec![-1] would be all of them

    // vl.prc values: ParamConfig, with the clone's kind where *FIGHTER_KIND_MARIO was.
    // ParamConfig answers with the first entry that covers the costume and holds
    // the field, so the one-costume entry is registered before the general one.
    // (kind, costumes, (param table, field), value)
    param_config::update_float(kind, vec![last], (hash40("param_special_n"), hash40("fireball_speed_mul")), 1.3);
    param_config::update_float(kind, others.clone(), (hash40("param_special_n"), hash40("fireball_speed_mul")), 1.15);
    // multiply the base's value instead of setting it
    param_config::update_attribute_mul(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.1);

    // fighter_param.prc attribute: read straight out of the row by main, so the engine
    let applied = TEMPLATE.param("run_speed_max").mul(1.10);
    // fighter_param_motion.prc field: ("param_motion", field)
    let applied = applied & TEMPLATE.param("param_motion").sub("escape_air_slide_distance").set(60.0);
    // one of the six fighter/common/param/ files: ("common", field); set shield_reset with shield_max
    let applied = applied & TEMPLATE.param("common").sub("shield_max").set(20.0);
    let applied = applied & TEMPLATE.param("common").sub("shield_reset").set(7.5);
    // where a fighter this clone throws sits: forward throw, Y
    let applied = applied & TEMPLATE.param("param_thrown").sub("offset_f_y").set(8.0);
    if !applied {
        clone_engine_api::elog!("[template_v2] a param was refused; ParamConfig missing, or see the log");
    }
}
