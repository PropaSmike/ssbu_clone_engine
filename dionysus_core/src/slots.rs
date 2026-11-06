pub const PARAM_NATIVE_KINDS: i32 = 94;
pub const CLONE_SLOTS: usize = 138;
pub const FIRST_CUSTOM_KIND: i32 = 118;
pub const LAST_BACKEND_KIND: i32 = 255;

pub fn base_row(base_kind: i32) -> Option<usize> {
    usize::try_from(base_kind)
        .ok()
        .filter(|row| *row < PARAM_NATIVE_KINDS as usize)
}

pub fn clone_row(clone_kind: i32) -> Option<usize> {
    clone_kind
        .checked_sub(FIRST_CUSTOM_KIND)
        .and_then(|row| usize::try_from(row).ok())
        .filter(|row| *row < CLONE_SLOTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_rows_cover_exactly_the_vanilla_roster() {
        assert_eq!(base_row(0), Some(0));
        assert_eq!(base_row(93), Some(93));
        assert_eq!(base_row(PARAM_NATIVE_KINDS), None);
        assert_eq!(base_row(PARAM_NATIVE_KINDS + 1), None);
    }

    #[test]
    fn a_negative_kind_is_rejected_not_wrapped() {
        assert_eq!(base_row(-1), None);
        assert_eq!(clone_row(-1), None);
        assert_eq!(base_row(i32::MIN), None);
        assert_eq!(clone_row(i32::MIN), None);
    }

    #[test]
    fn clone_rows_start_at_the_first_custom_kind() {
        assert_eq!(clone_row(FIRST_CUSTOM_KIND), Some(0));
        assert_eq!(clone_row(FIRST_CUSTOM_KIND + 1), Some(1));
        assert_eq!(clone_row(FIRST_CUSTOM_KIND - 1), None);
        assert_eq!(clone_row(0), None);
    }

    #[test]
    fn clone_rows_stop_at_the_slot_count() {
        assert_eq!(clone_row(FIRST_CUSTOM_KIND + CLONE_SLOTS as i32 - 1), Some(CLONE_SLOTS - 1));
        assert_eq!(clone_row(FIRST_CUSTOM_KIND + CLONE_SLOTS as i32), None);
        assert_eq!(clone_row(i32::MAX), None);
    }

    #[test]
    fn the_slot_range_reaches_the_last_backend_kind() {
        assert!(FIRST_CUSTOM_KIND + CLONE_SLOTS as i32 - 1 >= LAST_BACKEND_KIND);
        assert!(clone_row(LAST_BACKEND_KIND).is_some());
    }

    #[test]
    fn base_and_clone_ranges_never_overlap() {
        for kind in 0..PARAM_NATIVE_KINDS {
            assert!(clone_row(kind).is_none(), "kind {kind}");
        }
        for kind in FIRST_CUSTOM_KIND..=LAST_BACKEND_KIND {
            assert!(base_row(kind).is_none(), "kind {kind}");
        }
    }
}
