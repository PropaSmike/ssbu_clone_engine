#![allow(dead_code)]

pub(crate) const MAX_ANIMATIONS: usize = 3;
pub(crate) const HEADER_BYTES: usize = 24;

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0usize;
    while index < 256 {
        let mut value = index as u32;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 == 1 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

pub(crate) fn hash40(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    if bytes.len() > u8::MAX as usize {
        return None;
    }
    let mut hash = 0xffff_ffffu32;
    for byte in bytes {
        let lowered = byte.to_ascii_lowercase() as u32;
        hash = (hash >> 8) ^ CRC_TABLE[((lowered ^ hash) & 0xff) as usize];
    }
    Some(!hash as u64 | (bytes.len() as u64) << 32)
}

pub(crate) const MAGIC: u64 = 0x0006_f5fe_a1e8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Animation {
    pub name: u64,
    pub unk: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Extra {
    pub xlu_start: u8,
    pub xlu_end: u8,
    pub cancel_frame: u8,
    pub no_stop_intp: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Motion {
    pub game_script: u64,
    pub flags: u16,
    pub blend_frames: u8,
    pub animations: Vec<Animation>,
    pub scripts: Vec<u64>,
    pub extra: Option<Extra>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MotionList {
    pub motion_path: u64,
    pub entries: Vec<(u64, Motion)>,
}

impl MotionList {
    pub(crate) fn get(&self, kind: u64) -> Option<&Motion> {
        self.entries
            .iter()
            .find(|(known, _)| *known == kind)
            .map(|(_, motion)| motion)
    }

    pub(crate) fn insert(&mut self, kind: u64, motion: Motion) {
        match self.entries.iter_mut().find(|(known, _)| *known == kind) {
            Some(slot) => slot.1 = motion,
            None => self.entries.push((kind, motion)),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    TooShort,
    BadMagic,
    CountOverflow,
    AnimationCount,
    Truncated,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], ParseError> {
        let end = self.at.checked_add(count).ok_or(ParseError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(ParseError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ParseError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, ParseError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, ParseError> {
        let bytes = self.take(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(value))
    }

    fn align4(&mut self) -> Result<(), ParseError> {
        self.at = (self.at + 3) & !3;
        if self.at > self.bytes.len() {
            return Err(ParseError::Truncated);
        }
        Ok(())
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<MotionList, ParseError> {
    if bytes.len() < HEADER_BYTES {
        return Err(ParseError::TooShort);
    }
    let mut reader = Reader { bytes, at: 0 };
    if reader.u64()? != MAGIC {
        return Err(ParseError::BadMagic);
    }
    let motion_path = reader.u64()?;
    let count = reader.u64()?;
    if count > bytes.len() as u64 / 16 {
        return Err(ParseError::CountOverflow);
    }

    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let kind = reader.u64()?;
        entries.push((kind, read_motion(&mut reader)?));
    }
    Ok(MotionList {
        motion_path,
        entries,
    })
}

fn read_motion(reader: &mut Reader<'_>) -> Result<Motion, ParseError> {
    let game_script = reader.u64()?;
    let flags = reader.u16()?;
    let blend_frames = reader.u8()?;
    let animation_count = reader.u8()? as usize;
    if animation_count > MAX_ANIMATIONS {
        return Err(ParseError::AnimationCount);
    }
    let size = reader.u32()? as usize;

    let mut names = Vec::with_capacity(animation_count);
    for _ in 0..animation_count {
        names.push(reader.u64()?);
    }
    let mut animations = Vec::with_capacity(animation_count);
    for name in names {
        animations.push(Animation {
            name,
            unk: reader.u8()?,
        });
    }
    reader.align4()?;

    let mut scripts = Vec::with_capacity(size / 8);
    for _ in 0..size / 8 {
        scripts.push(reader.u64()?);
    }

    let extra = if size % 8 == 4 {
        Some(Extra {
            xlu_start: reader.u8()?,
            xlu_end: reader.u8()?,
            cancel_frame: reader.u8()?,
            no_stop_intp: reader.u8()? > 0,
        })
    } else {
        None
    };

    Ok(Motion {
        game_script,
        flags,
        blend_frames,
        animations,
        scripts,
        extra,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WriteError {
    AnimationCount,
}

pub(crate) fn serialize(list: &MotionList) -> Result<Vec<u8>, WriteError> {
    let mut out = Vec::with_capacity(HEADER_BYTES + list.entries.len() * 64);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&list.motion_path.to_le_bytes());
    out.extend_from_slice(&(list.entries.len() as u64).to_le_bytes());

    for (kind, motion) in &list.entries {
        if motion.animations.len() > MAX_ANIMATIONS {
            return Err(WriteError::AnimationCount);
        }
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&motion.game_script.to_le_bytes());
        out.extend_from_slice(&motion.flags.to_le_bytes());
        out.push(motion.blend_frames);
        out.push(motion.animations.len() as u8);

        let size = motion.scripts.len() * 8 + usize::from(motion.extra.is_some()) * 4;
        out.extend_from_slice(&(size as u32).to_le_bytes());

        for animation in &motion.animations {
            out.extend_from_slice(&animation.name.to_le_bytes());
        }
        for animation in &motion.animations {
            out.push(animation.unk);
        }
        while out.len() % 4 != 0 {
            out.push(0);
        }
        for script in &motion.scripts {
            out.extend_from_slice(&script.to_le_bytes());
        }
        if let Some(extra) = &motion.extra {
            out.push(extra.xlu_start);
            out.push(extra.xlu_end);
            out.push(extra.cancel_frame);
            out.push(u8::from(extra.no_stop_intp));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_matches_the_published_vectors() {
        assert_eq!(hash40(""), Some(0));
        assert_eq!(hash40("a"), Some(0x01e8_b7be_43));
        assert_eq!(hash40("A"), Some(0x01e8_b7be_43));
        assert_eq!(hash40("damage_max"), Some(0x0aa3_cb88_10));
        assert_eq!(hash40(&"x".repeat(256)), None);
    }

    #[test]
    fn the_magic_is_the_hash_of_motion() {
        assert_eq!(hash40("motion"), Some(MAGIC));
    }

    fn sample() -> MotionList {
        MotionList {
            motion_path: hash40("fighter/kirby/motion/body/c00").unwrap(),
            entries: vec![
                (
                    hash40("donkey_special_n").unwrap(),
                    Motion {
                        game_script: hash40("game_donkeyspecialn").unwrap(),
                        flags: 0b10,
                        blend_frames: 3,
                        animations: vec![Animation {
                            name: hash40("donkeyd00specialn.nuanmb").unwrap(),
                            unk: 1,
                        }],
                        scripts: vec![
                            hash40("sound_donkeyspecialn").unwrap(),
                            hash40("effect_donkeyspecialn").unwrap(),
                            hash40("expression_donkeyspecialn").unwrap(),
                        ],
                        extra: Some(Extra {
                            xlu_start: 0,
                            xlu_end: 0,
                            cancel_frame: 63,
                            no_stop_intp: false,
                        }),
                    },
                ),
                (
                    hash40("wait").unwrap(),
                    Motion {
                        game_script: hash40("game_wait").unwrap(),
                        flags: 0,
                        blend_frames: 0,
                        animations: vec![
                            Animation {
                                name: hash40("a00wait1.nuanmb").unwrap(),
                                unk: 0,
                            },
                            Animation {
                                name: hash40("a00wait2.nuanmb").unwrap(),
                                unk: 2,
                            },
                        ],
                        scripts: vec![],
                        extra: None,
                    },
                ),
            ],
        }
    }

    #[test]
    fn a_list_survives_a_write_then_read() {
        let original = sample();
        let bytes = serialize(&original).unwrap();
        assert_eq!(parse(&bytes).unwrap(), original);
    }

    #[test]
    fn the_bytes_are_stable_across_a_read_write_cycle() {
        let bytes = serialize(&sample()).unwrap();
        let reparsed = parse(&bytes).unwrap();
        assert_eq!(serialize(&reparsed).unwrap(), bytes);
    }

    #[test]
    fn every_entry_starts_on_a_four_byte_boundary() {
        let bytes = serialize(&sample()).unwrap();
        assert_eq!(bytes.len() % 4, 0);
        assert_eq!(HEADER_BYTES % 4, 0);
    }

    #[test]
    fn a_file_that_is_not_a_motion_list_is_refused() {
        assert_eq!(parse(&[]), Err(ParseError::TooShort));
        assert_eq!(parse(&[0u8; 64]), Err(ParseError::BadMagic));
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_read_past_the_end() {
        let bytes = serialize(&sample()).unwrap();
        for cut in HEADER_BYTES..bytes.len() {
            assert!(parse(&bytes[..cut]).is_err(), "accepted a {cut} byte file");
        }
    }

    #[test]
    fn a_count_larger_than_the_file_is_refused() {
        let mut bytes = serialize(&sample()).unwrap();
        bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(parse(&bytes), Err(ParseError::CountOverflow));
    }

    #[test]
    fn insert_replaces_a_name_that_is_already_there() {
        let mut list = sample();
        let before = list.entries.len();
        let kind = hash40("wait").unwrap();
        list.insert(kind, Motion::default());
        assert_eq!(list.entries.len(), before);
        assert_eq!(list.get(kind), Some(&Motion::default()));
    }
}
