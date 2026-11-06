#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Drift {
    pub index: usize,
    pub old: usize,
    pub now: usize,
    pub adopted: bool,
}

pub fn take(table: &mut [usize], seen: &mut [usize], vanilla: &[usize]) {
    let count = table.len().min(seen.len()).min(vanilla.len());
    table[..count].copy_from_slice(&vanilla[..count]);
    seen[..count].copy_from_slice(&vanilla[..count]);
}

pub fn install(table: &mut [usize], index: usize, function: usize) -> Option<usize> {
    if function == 0 {
        return None;
    }
    let entry = table.get_mut(index)?;
    Some(core::mem::replace(entry, function))
}

pub fn reconcile(table: &mut [usize], seen: &mut [usize], vanilla: &[usize]) -> Vec<Drift> {
    let count = table.len().min(seen.len()).min(vanilla.len());
    let mut drifts = Vec::new();
    for index in 0..count {
        let now = vanilla[index];
        let old = seen[index];
        if now == old {
            continue;
        }
        seen[index] = now;
        let adopted = table[index] == old;
        if adopted {
            table[index] = now;
        }
        drifts.push(Drift {
            index,
            old,
            now,
            adopted,
        });
    }
    drifts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy() -> ([usize; 4], [usize; 4], [usize; 4]) {
        let vanilla = [0x100, 0x200, 0x300, 0x400];
        let mut table = [0; 4];
        let mut seen = [0; 4];
        take(&mut table, &mut seen, &vanilla);
        (table, seen, vanilla)
    }

    #[test]
    fn an_unchanged_base_table_changes_nothing() {
        let (mut table, mut seen, vanilla) = copy();
        assert!(reconcile(&mut table, &mut seen, &vanilla).is_empty());
        assert_eq!(table, vanilla);
    }

    #[test]
    fn a_slot_nobody_overrode_takes_the_base_tables_new_entry() {
        let (mut table, mut seen, mut vanilla) = copy();
        vanilla[2] = 0x999;
        let drifts = reconcile(&mut table, &mut seen, &vanilla);
        assert_eq!(
            drifts,
            [Drift {
                index: 2,
                old: 0x300,
                now: 0x999,
                adopted: true
            }]
        );
        assert_eq!(table[2], 0x999);
        assert!(reconcile(&mut table, &mut seen, &vanilla).is_empty());
    }

    #[test]
    fn an_overridden_slot_keeps_the_override_and_reports_the_change() {
        let (mut table, mut seen, mut vanilla) = copy();
        assert_eq!(install(&mut table, 1, 0xabc), Some(0x200));
        vanilla[1] = 0x999;
        let drifts = reconcile(&mut table, &mut seen, &vanilla);
        assert_eq!(
            drifts,
            [Drift {
                index: 1,
                old: 0x200,
                now: 0x999,
                adopted: false
            }]
        );
        assert_eq!(table[1], 0xabc);
        assert!(reconcile(&mut table, &mut seen, &vanilla).is_empty());
    }

    #[test]
    fn a_second_override_on_one_slot_chains_to_the_first() {
        let (mut table, _, _) = copy();
        assert_eq!(install(&mut table, 3, 0xaaa), Some(0x400));
        assert_eq!(install(&mut table, 3, 0xbbb), Some(0xaaa));
        assert_eq!(install(&mut table, 4, 0xccc), None);
        assert_eq!(install(&mut table, 0, 0), None);
    }
}
