pub const HISTORY: usize = 40;
pub const DEAD_ZONE: f32 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    QuarterForward,
    QuarterBack,
    DragonForward,
    DragonBack,
    HalfForward,
    HalfBack,
    DownDown,
}

impl Motion {
    pub const ALL: [Motion; 7] = [
        Motion::HalfForward,
        Motion::HalfBack,
        Motion::DragonForward,
        Motion::DragonBack,
        Motion::QuarterForward,
        Motion::QuarterBack,
        Motion::DownDown,
    ];

    pub const fn sequence(self) -> &'static [u8] {
        match self {
            Motion::QuarterForward => &[2, 3, 6],
            Motion::QuarterBack => &[2, 1, 4],
            Motion::DragonForward => &[6, 2, 3],
            Motion::DragonBack => &[4, 2, 1],
            Motion::HalfForward => &[4, 1, 2, 3, 6],
            Motion::HalfBack => &[6, 3, 2, 1, 4],
            Motion::DownDown => &[2, 5, 2],
        }
    }

    pub const fn window(self) -> usize {
        match self {
            Motion::HalfForward | Motion::HalfBack => 24,
            Motion::DownDown => 16,
            _ => 15,
        }
    }

    pub const fn notation(self) -> &'static str {
        match self {
            Motion::QuarterForward => "236",
            Motion::QuarterBack => "214",
            Motion::DragonForward => "623",
            Motion::DragonBack => "421",
            Motion::HalfForward => "41236",
            Motion::HalfBack => "63214",
            Motion::DownDown => "22",
        }
    }
}

pub fn numpad(x: f32, y: f32, lr: f32) -> u8 {
    let forward = x * lr;
    let h = if forward > DEAD_ZONE { 1 } else if forward < -DEAD_ZONE { -1 } else { 0 };
    let v = if y > DEAD_ZONE { 1 } else if y < -DEAD_ZONE { -1 } else { 0 };
    (5 + h + 3 * v) as u8
}

#[derive(Debug, Clone, Copy)]
pub struct InputHistory {
    dirs: [u8; HISTORY],
    next: usize,
    filled: usize,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHistory {
    pub const fn new() -> Self {
        Self { dirs: [5; HISTORY], next: 0, filled: 0 }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn push(&mut self, dir: u8) {
        self.dirs[self.next] = dir;
        self.next = (self.next + 1) % HISTORY;
        self.filled = (self.filled + 1).min(HISTORY);
    }

    fn back(&self, age: usize) -> u8 {
        self.dirs[(self.next + HISTORY - 1 - age) % HISTORY]
    }

    pub fn matches(&self, motion: Motion, end_within: usize) -> bool {
        let sequence = motion.sequence();
        let window = motion.window().min(self.filled);
        let mut want = sequence.len();
        let mut age = 0;
        while age < window && want > 0 {
            let dir = self.back(age);
            if dir == sequence[want - 1] {
                if want == sequence.len() && age > end_within {
                    return false;
                }
                want -= 1;
            }
            age += 1;
        }
        want == 0
    }

    pub fn recognise(&self, end_within: usize) -> Option<Motion> {
        Motion::ALL.into_iter().find(|motion| self.matches(*motion, end_within))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(dirs: &[u8]) -> InputHistory {
        let mut history = InputHistory::new();
        for dir in dirs {
            history.push(*dir);
        }
        history
    }

    #[test]
    fn a_quarter_circle_reads_as_236_and_its_mirror_as_214() {
        assert_eq!(history(&[5, 2, 2, 3, 6, 6]).recognise(4), Some(Motion::QuarterForward));
        assert_eq!(history(&[5, 2, 1, 1, 4]).recognise(4), Some(Motion::QuarterBack));
    }

    #[test]
    fn a_dragon_punch_wins_over_the_quarter_circle_inside_it() {
        assert_eq!(history(&[6, 5, 2, 3]).recognise(4), Some(Motion::DragonForward));
        assert_eq!(history(&[4, 1, 2, 3, 6]).recognise(4), Some(Motion::HalfForward));
    }

    #[test]
    fn a_motion_finished_too_long_ago_or_spread_too_wide_is_not_read() {
        let mut stale = history(&[2, 3, 6]);
        for _ in 0..8 {
            stale.push(5);
        }
        assert_eq!(stale.recognise(4), None);
        let mut slow = history(&[2]);
        for _ in 0..20 {
            slow.push(5);
        }
        slow.push(3);
        slow.push(6);
        assert!(!slow.matches(Motion::QuarterForward, 4));
    }

    #[test]
    fn down_down_needs_a_release_between_and_the_stick_reads_relative_to_facing() {
        assert_eq!(history(&[2, 5, 2]).recognise(2), Some(Motion::DownDown));
        assert_eq!(history(&[2, 2, 2]).recognise(2), None);
        assert_eq!(numpad(1.0, 0.0, -1.0), 4);
        assert_eq!(numpad(0.7, -0.7, 1.0), 3);
        assert_eq!(numpad(0.1, 0.1, 1.0), 5);
    }
}
