#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Aegis,
    Trainer,
    Free,
}

pub const AEGIS_KINDS: [i32; 2] = [0x5a, 0x5b];
pub const TRAINER_KINDS: [i32; 3] = [0x24, 0x25, 0x26];
pub const AEGIS_BASES: [&str; 2] = ["eflame", "elight"];
pub const TRAINER_BASES: [&str; 3] = ["pzenigame", "pfushigisou", "plizardon"];
pub const NEEDS_ITS_SET: [i32; 2] = [0x4b, 0x4c];

pub fn in_a_game_set(base: i32) -> bool {
    AEGIS_KINDS.contains(&base) || TRAINER_KINDS.contains(&base)
}

fn is_permutation(bases: &[i32], of: &[i32]) -> bool {
    bases.len() == of.len()
        && of.iter().all(|kind| bases.iter().filter(|base| *base == kind).count() == 1)
}

pub fn shape(bases: &[i32]) -> Result<Shape, String> {
    if is_permutation(bases, &AEGIS_KINDS) {
        return Ok(Shape::Aegis);
    }
    if is_permutation(bases, &TRAINER_KINDS) {
        return Ok(Shape::Trainer);
    }
    if !(2..=3).contains(&bases.len()) {
        return Err(format!("a switch group holds two or three fighters; got {}", bases.len()));
    }
    if bases.iter().any(|base| in_a_game_set(*base)) {
        return Err(format!(
            "Pyra and Mythra (eflame, elight) and Pokemon Trainer's three (pzenigame, pfushigisou, plizardon) switch only within their own set; got kinds {bases:?}"
        ));
    }
    if bases.iter().any(|base| NEEDS_ITS_SET.contains(base)) {
        return Err(format!("the Ice Climbers cannot be in a switch group; got kinds {bases:?}"));
    }
    if bases.iter().enumerate().any(|(index, base)| bases[..index].contains(base)) {
        return Err(format!("each fighter in a switch group needs a different base; got kinds {bases:?}"));
    }
    Ok(Shape::Free)
}

pub fn rotate<T: Clone>(group: &[T], first: usize) -> Vec<T> {
    (0..group.len()).map(|step| group[(first + step) % group.len()].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_sets_keep_their_own_shape() {
        assert_eq!(shape(&[0x5b, 0x5a]), Ok(Shape::Aegis));
        assert_eq!(shape(&[0x26, 0x24, 0x25]), Ok(Shape::Trainer));
        assert!(shape(&[0x5a, 0x5a]).is_err());
        assert!(shape(&[0x24, 0x25]).is_err());
        assert!(shape(&[0x5a, 0x24]).is_err());
        assert!(shape(&[0x5a, 0x00]).is_err());
    }

    #[test]
    fn any_other_two_or_three_bases_switch_freely() {
        assert_eq!(shape(&[0x00, 0x08]), Ok(Shape::Free));
        assert_eq!(shape(&[0x01, 0x02, 0x03]), Ok(Shape::Free));
        assert!(shape(&[0x00]).is_err());
        assert!(shape(&[0x00, 0x01, 0x02, 0x03]).is_err());
        assert!(shape(&[0x00, 0x00]).unwrap_err().contains("different base"));
        assert!(shape(&[0x00, 0x4b]).unwrap_err().contains("Ice Climbers"));
    }

    #[test]
    fn the_selected_member_comes_first_and_the_cycle_order_is_kept() {
        assert_eq!(rotate(&[1, 2, 3], 1), vec![2, 3, 1]);
        assert_eq!(rotate(&[1, 2], 0), vec![1, 2]);
    }
}
