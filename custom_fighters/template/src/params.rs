pub fn install(kind: i32) {
    use clone_engine_api::ParamOp;

    let mut bridged = clone_engine_api::param_override(kind, "run_speed_max", ParamOp::Mul, 1.10);
    bridged &= clone_engine_api::param_override(kind, "weight", ParamOp::Set, 100.0);
    bridged &= clone_engine_api::param_override_full(
        kind,
        clone_engine_api::ANY_SLOT,
        "param_special_n",
        "fireball_speed_mul",
        ParamOp::Mul,
        1.15,
    );
    bridged &= clone_engine_api::param_override_slot(kind, 7, "scale", ParamOp::Mul, 0.95);
    bridged &= clone_engine_api::param_int_override(kind, "jump_squat_frame", 10);
    if !bridged {
        clone_engine_api::elog!("[template] ParamConfig bridge unavailable");
    }
}
