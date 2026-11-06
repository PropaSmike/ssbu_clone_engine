use crate::owner_param_words::{words_of, Word, WordKind};

pub const POPO: i32 = 0x4b;
const FIRST: u16 = 0xf8;
const LAST: u16 = 0x1a8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerParam {
    pub offset: u32,
    pub bits: u32,
}

pub fn words() -> impl Iterator<Item = &'static Word> {
    words_of(POPO)
        .iter()
        .filter(|word| (FIRST..=LAST).contains(&word.offset))
}

fn named(name: &str) -> Option<&'static Word> {
    let path = name.replace('.', "/");
    words().find(|word| {
        word.path() == path || (word.parent_name() == "param_private" && word.field_name() == name)
    })
}

pub fn resolve(name: &str, value: &str) -> Result<PartnerParam, String> {
    let word = named(name).ok_or_else(|| format!("{name} is not one of the Ice Climbers' partner params"))?;
    let value = value.trim();
    let bits = match word.kind {
        WordKind::F32 => value
            .parse::<f32>()
            .map_err(|_| format!("{name} must be a number"))?
            .to_bits(),
        WordKind::I32 => value
            .parse::<i32>()
            .map_err(|_| format!("{name} must be a whole number"))? as u32,
        WordKind::U32 | WordKind::Bool => value
            .parse::<u32>()
            .map_err(|_| format!("{name} must be a whole number"))?,
    };
    Ok(PartnerParam {
        offset: word.offset as u32,
        bits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_private_param_resolves_by_its_short_name() {
        let delay = resolve("nana_delay", "12").unwrap();
        assert_eq!(delay, PartnerParam { offset: 0x140, bits: 12 });
        let distance = resolve("param_private.nana_opt_dst", "30").unwrap();
        assert_eq!(distance, PartnerParam { offset: 0x13c, bits: 30.0f32.to_bits() });
    }

    #[test]
    fn a_jostle_param_needs_its_table() {
        let speed = resolve("jostle_data_nana.speed_x_gg", "0.5").unwrap();
        assert_eq!(speed, PartnerParam { offset: 0xf8, bits: 0.5f32.to_bits() });
        assert!(resolve("speed_x_gg", "0.5").is_err());
    }

    #[test]
    fn wrong_names_and_values_are_refused() {
        assert_eq!(
            resolve("nana_speed", "1.0").unwrap_err(),
            "nana_speed is not one of the Ice Climbers' partner params"
        );
        assert_eq!(resolve("nana_delay", "1.5").unwrap_err(), "nana_delay must be a whole number");
        assert!(resolve("map_collision_iceshot.up", "1.0").is_err());
    }
}
