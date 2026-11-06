pub fn hash40(name: &str) -> u64 {
    const POLY: u32 = 0xEDB8_8320;
    let mut crc = 0xFFFF_FFFFu32;
    for byte in name.as_bytes() {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
        }
    }
    ((name.len() as u64) << 32) | u64::from(!crc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_fighter_param_field_names() {
        assert_eq!(hash40("jump_y"), 0x6_a0d8_2dad);
        assert_eq!(hash40("dash_speed"), 0xa_dec4_7ea3);
        assert_eq!(hash40("walk_speed_max"), 0xe_e2ec_2860);
        assert_eq!(hash40("ground_brake"), 0xc_8cc9_db76);
        assert_eq!(hash40("run_speed_max"), 0xd_5aaa_bf9b);
        assert_eq!(hash40("mini_jump_y"), 0xb_42b7_b19f);
        assert_eq!(hash40("jump_aerial_y"), 0xd_8679_8cae);
        assert_eq!(hash40("jump_initial_y"), 0xe_d1f9_fdb8);
    }

    #[test]
    fn the_top_byte_is_the_length() {
        for name in ["a", "weight", "fighter_kind_mario", "walk_speed_max"] {
            assert_eq!(hash40(name) >> 32, name.len() as u64, "{name}");
        }
    }

    #[test]
    fn an_empty_name_is_a_bare_crc_seed() {
        assert_eq!(hash40(""), 0);
    }

    #[test]
    fn hashing_is_case_sensitive_here_so_callers_must_lowercase() {
        assert_ne!(hash40("Weight"), hash40("weight"));
    }
}
