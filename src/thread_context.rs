use core::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};

const SLOTS: usize = 64;

struct Occupancy {
    label: &'static str,
    live: AtomicUsize,
    peak: AtomicUsize,
    exhausted: AtomicU32,
}

impl Occupancy {
    const fn new(label: &'static str) -> Self {
        Self {
            label,
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            exhausted: AtomicU32::new(0),
        }
    }

    fn claim(&self) {
        let live = self.live.fetch_add(1, Ordering::AcqRel) + 1;
        let mut peak = self.peak.load(Ordering::Acquire);
        while live > peak {
            match self
                .peak
                .compare_exchange(peak, live, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    report(format_args!(
                        "[ctxslots] {} peak {}/{}",
                        self.label, live, SLOTS
                    ));
                    break;
                }
                Err(seen) => peak = seen,
            }
        }
    }

    fn release(&self) {
        self.live.fetch_sub(1, Ordering::AcqRel);
    }

    fn exhaust(&self) {
        let count = self.exhausted.fetch_add(1, Ordering::AcqRel) + 1;
        report(format_args!(
            "[ctxslots] {} EXHAUSTED {} time(s): all {} slots busy, context lost",
            self.label, count, SLOTS
        ));
    }

    #[cfg(test)]
    fn peak(&self) -> usize {
        self.peak.load(Ordering::Acquire)
    }

    #[cfg(test)]
    fn exhausted(&self) -> u32 {
        self.exhausted.load(Ordering::Acquire)
    }
}

#[cfg(not(test))]
fn report(args: core::fmt::Arguments<'_>) {
    let text = std::fmt::format(args);
    unsafe { crate::dbg_out(&text) };
    skyline::println!("{}", text);
}

#[cfg(test)]
fn report(_args: core::fmt::Arguments<'_>) {}

#[allow(clippy::declare_interior_mutable_const)]
const FREE_THREAD: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const NO_KIND: AtomicI32 = AtomicI32::new(-1);
#[allow(clippy::declare_interior_mutable_const)]
const ZERO_DEPTH: AtomicI32 = AtomicI32::new(0);

pub(crate) struct ThreadReentrancyFlag {
    threads: [AtomicUsize; SLOTS],
    depths: [AtomicI32; SLOTS],
    occupancy: Occupancy,
}

impl ThreadReentrancyFlag {
    pub(crate) const fn new(label: &'static str) -> Self {
        Self {
            threads: [FREE_THREAD; SLOTS],
            depths: [ZERO_DEPTH; SLOTS],
            occupancy: Occupancy::new(label),
        }
    }

    pub(crate) fn is_active(&self, thread: usize) -> bool {
        if thread == 0 {
            return false;
        }
        self.threads
            .iter()
            .zip(self.depths.iter())
            .any(|(slot, depth)| {
                slot.load(Ordering::Acquire) == thread && depth.load(Ordering::Acquire) > 0
            })
    }

    pub(crate) fn enter(&self, thread: usize) -> ReentrancyGuard<'_> {
        if thread == 0 {
            return ReentrancyGuard { depth: None };
        }
        for (slot, depth) in self.threads.iter().zip(self.depths.iter()) {
            if slot.load(Ordering::Acquire) == thread {
                depth.fetch_add(1, Ordering::AcqRel);
                return ReentrancyGuard {
                    depth: Some((depth, None)),
                };
            }
        }
        for (slot, depth) in self.threads.iter().zip(self.depths.iter()) {
            if slot
                .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.occupancy.claim();
                depth.fetch_add(1, Ordering::AcqRel);
                return ReentrancyGuard {
                    depth: Some((depth, Some((slot, &self.occupancy)))),
                };
            }
        }
        self.occupancy.exhaust();
        ReentrancyGuard { depth: None }
    }
}

pub(crate) struct ReentrancyGuard<'a> {
    depth: Option<(&'a AtomicI32, Option<(&'a AtomicUsize, &'a Occupancy)>)>,
}

impl Drop for ReentrancyGuard<'_> {
    fn drop(&mut self) {
        let Some((depth, slot)) = self.depth else {
            return;
        };
        depth.fetch_sub(1, Ordering::AcqRel);
        if let Some((slot, occupancy)) = slot {
            slot.store(0, Ordering::Release);
            occupancy.release();
        }
    }
}

pub(crate) struct ThreadScopedKind {
    threads: [AtomicUsize; SLOTS],
    kinds: [AtomicI32; SLOTS],
    occupancy: Occupancy,
}

impl ThreadScopedKind {
    pub(crate) const fn new(label: &'static str) -> Self {
        Self {
            threads: [FREE_THREAD; SLOTS],
            kinds: [NO_KIND; SLOTS],
            occupancy: Occupancy::new(label),
        }
    }

    fn owned_slot(&self, thread: usize) -> Option<usize> {
        if thread == 0 {
            return None;
        }
        (0..SLOTS).find(|&index| self.threads[index].load(Ordering::Acquire) == thread)
    }

    pub(crate) fn active(&self, thread: usize) -> Option<i32> {
        let index = self.owned_slot(thread)?;
        let kind = self.kinds[index].load(Ordering::Acquire);
        (kind >= 0).then_some(kind)
    }

    pub(crate) fn enter(&self, thread: usize, kind: i32) -> ScopedKindGuard<'_> {
        if thread == 0 {
            return ScopedKindGuard { restore: None };
        }

        if let Some(index) = self.owned_slot(thread) {
            let previous = self.kinds[index].swap(kind, Ordering::AcqRel);
            return ScopedKindGuard {
                restore: Some((&self.kinds[index], previous, None)),
            };
        }

        for index in 0..SLOTS {
            if self.threads[index]
                .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.occupancy.claim();
                self.kinds[index].store(kind, Ordering::Release);
                return ScopedKindGuard {
                    restore: Some((
                        &self.kinds[index],
                        -1,
                        Some((&self.threads[index], &self.occupancy)),
                    )),
                };
            }
        }

        self.occupancy.exhaust();
        ScopedKindGuard { restore: None }
    }

    pub(crate) fn scope<R>(&self, thread: usize, kind: i32, callback: impl FnOnce() -> R) -> R {
        if thread == 0 {
            return callback();
        }

        if let Some(index) = self.owned_slot(thread) {
            let previous = self.kinds[index].swap(kind, Ordering::AcqRel);
            let result = callback();
            self.kinds[index].store(previous, Ordering::Release);
            return result;
        }

        for index in 0..SLOTS {
            if self.threads[index]
                .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.occupancy.claim();
                self.kinds[index].store(kind, Ordering::Release);
                let result = callback();
                self.kinds[index].store(-1, Ordering::Release);
                self.threads[index].store(0, Ordering::Release);
                self.occupancy.release();
                return result;
            }
        }

        self.occupancy.exhaust();
        callback()
    }
}

pub(crate) struct ScopedKindGuard<'a> {
    restore: Option<(&'a AtomicI32, i32, Option<(&'a AtomicUsize, &'a Occupancy)>)>,
}

impl Drop for ScopedKindGuard<'_> {
    fn drop(&mut self) {
        let Some((kind, previous, slot)) = self.restore else {
            return;
        };
        kind.store(previous, Ordering::Release);
        if let Some((slot, occupancy)) = slot {
            slot.store(0, Ordering::Release);
            occupancy.release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREAD_A: usize = 0x1111;
    const THREAD_B: usize = 0x2222;

    #[test]
    fn reentrancy_flag_marks_only_the_entering_thread() {
        let flag = ThreadReentrancyFlag::new("test");
        assert!(!flag.is_active(THREAD_A));
        {
            let _guard = flag.enter(THREAD_A);
            assert!(flag.is_active(THREAD_A));
            assert!(!flag.is_active(THREAD_B));
        }
        assert!(!flag.is_active(THREAD_A));
    }

    #[test]
    fn reentrancy_flag_counts_nesting() {
        let flag = ThreadReentrancyFlag::new("test");
        let outer = flag.enter(THREAD_A);
        {
            let _inner = flag.enter(THREAD_A);
            assert!(flag.is_active(THREAD_A));
        }
        assert!(
            flag.is_active(THREAD_A),
            "inner drop cleared the outer mark"
        );
        drop(outer);
        assert!(!flag.is_active(THREAD_A));
    }

    #[test]
    fn reentrancy_flag_releases_its_slot() {
        let flag = ThreadReentrancyFlag::new("test");
        for _ in 0..(SLOTS * 4) {
            let _guard = flag.enter(THREAD_A);
            assert!(flag.is_active(THREAD_A));
        }
        assert!(!flag.is_active(THREAD_A));
    }

    #[test]
    fn publishes_and_clears() {
        let ctx = ThreadScopedKind::new("test");
        assert_eq!(ctx.active(THREAD_A), None);
        ctx.scope(THREAD_A, 123, || {
            assert_eq!(ctx.active(THREAD_A), Some(123));
        });
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn nests_a_different_kind_on_the_same_thread() {
        let ctx = ThreadScopedKind::new("test");
        ctx.scope(THREAD_A, 123, || {
            assert_eq!(ctx.active(THREAD_A), Some(123));
            ctx.scope(THREAD_A, 119, || {
                assert_eq!(ctx.active(THREAD_A), Some(119));
            });
            assert_eq!(ctx.active(THREAD_A), Some(123));
        });
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn nests_the_same_kind_on_the_same_thread() {
        let ctx = ThreadScopedKind::new("test");
        ctx.scope(THREAD_A, 123, || {
            ctx.scope(THREAD_A, 123, || {
                assert_eq!(ctx.active(THREAD_A), Some(123));
            });
            assert_eq!(ctx.active(THREAD_A), Some(123));
        });
    }

    #[test]
    fn threads_are_independent_and_never_block() {
        let ctx = ThreadScopedKind::new("test");
        ctx.scope(THREAD_A, 123, || {
            assert_eq!(ctx.active(THREAD_B), None);
            ctx.scope(THREAD_B, 119, || {
                assert_eq!(ctx.active(THREAD_B), Some(119));
                assert_eq!(ctx.active(THREAD_A), Some(123));
            });
            assert_eq!(ctx.active(THREAD_B), None);
            assert_eq!(ctx.active(THREAD_A), Some(123));
        });
    }

    #[test]
    fn thread_key_zero_is_never_scoped() {
        let ctx = ThreadScopedKind::new("test");
        let mut ran = false;
        ctx.scope(0, 123, || {
            ran = true;
            assert_eq!(ctx.active(0), None);
        });
        assert!(ran);
    }

    #[test]
    fn guard_publishes_and_clears() {
        let ctx = ThreadScopedKind::new("test");
        {
            let _guard = ctx.enter(THREAD_A, 123);
            assert_eq!(ctx.active(THREAD_A), Some(123));
        }
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn guard_nests_by_save_and_restore() {
        let ctx = ThreadScopedKind::new("test");
        let outer = ctx.enter(THREAD_A, 123);
        {
            let _inner = ctx.enter(THREAD_A, 119);
            assert_eq!(ctx.active(THREAD_A), Some(119));
        }
        assert_eq!(
            ctx.active(THREAD_A),
            Some(123),
            "inner drop clobbered the outer scope"
        );
        drop(outer);
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn guard_releases_its_slot() {
        let ctx = ThreadScopedKind::new("test");
        for _ in 0..(SLOTS * 4) {
            let _guard = ctx.enter(THREAD_A, 123);
            assert_eq!(ctx.active(THREAD_A), Some(123));
        }
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn guard_threads_are_independent() {
        let ctx = ThreadScopedKind::new("test");
        let _a = ctx.enter(THREAD_A, 123);
        assert_eq!(ctx.active(THREAD_B), None);
        {
            let _b = ctx.enter(THREAD_B, 119);
            assert_eq!(ctx.active(THREAD_B), Some(119));
            assert_eq!(ctx.active(THREAD_A), Some(123));
        }
        assert_eq!(ctx.active(THREAD_B), None);
        assert_eq!(ctx.active(THREAD_A), Some(123));
    }

    #[test]
    fn guard_is_inert_when_the_table_is_full() {
        let ctx = ThreadScopedKind::new("test");
        for slot in 0..SLOTS {
            ctx.threads[slot].store(0x9000 + slot, Ordering::Release);
        }
        let _guard = ctx.enter(THREAD_A, 123);
        assert_eq!(ctx.active(THREAD_A), None, "a full table must not block");
    }

    #[test]
    fn guard_and_closure_forms_interoperate() {
        let ctx = ThreadScopedKind::new("test");
        let outer = ctx.enter(THREAD_A, 123);
        ctx.scope(THREAD_A, 119, || {
            assert_eq!(ctx.active(THREAD_A), Some(119));
        });
        assert_eq!(ctx.active(THREAD_A), Some(123));
        drop(outer);
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn runs_the_callback_when_the_table_is_full() {
        let ctx = ThreadScopedKind::new("test");
        for slot in 0..SLOTS {
            ctx.threads[slot].store(0x9000 + slot, Ordering::Release);
        }
        let mut ran = false;
        ctx.scope(THREAD_A, 123, || {
            ran = true;
            assert_eq!(ctx.active(THREAD_A), None);
        });
        assert!(ran, "a full table must not block or skip the callback");
    }

    #[test]
    fn scoped_kind_counts_exhaustion_and_peak() {
        let ctx = ThreadScopedKind::new("test");
        for slot in 0..SLOTS {
            ctx.threads[slot].store(0x9000 + slot, Ordering::Release);
            ctx.occupancy.claim();
        }
        assert_eq!(ctx.occupancy.peak(), SLOTS);
        assert_eq!(ctx.occupancy.exhausted(), 0);
        let _guard = ctx.enter(THREAD_A, 123);
        assert_eq!(ctx.active(THREAD_A), None);
        assert_eq!(ctx.occupancy.exhausted(), 1);
        ctx.scope(THREAD_A, 123, || {});
        assert_eq!(ctx.occupancy.exhausted(), 2);
    }

    #[test]
    fn scoped_kind_releases_occupancy_with_the_slot() {
        let ctx = ThreadScopedKind::new("test");
        {
            let _guard = ctx.enter(THREAD_A, 123);
            assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 1);
        }
        assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 0);
        ctx.scope(THREAD_A, 123, || {
            assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 1);
        });
        assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 0);
        assert_eq!(ctx.occupancy.peak(), 1);
        assert_eq!(ctx.occupancy.exhausted(), 0);
    }

    #[test]
    fn nesting_does_not_consume_a_second_slot() {
        let ctx = ThreadScopedKind::new("test");
        let outer = ctx.enter(THREAD_A, 123);
        let inner = ctx.enter(THREAD_A, 119);
        assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 1);
        assert_eq!(ctx.occupancy.peak(), 1);
        drop(inner);
        drop(outer);
        assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 0);

        let flag = ThreadReentrancyFlag::new("test");
        let outer = flag.enter(THREAD_A);
        let inner = flag.enter(THREAD_A);
        assert_eq!(flag.occupancy.live.load(Ordering::Acquire), 1);
        drop(inner);
        drop(outer);
        assert_eq!(flag.occupancy.live.load(Ordering::Acquire), 0);
    }

    #[test]
    fn reentrancy_flag_counts_exhaustion() {
        let flag = ThreadReentrancyFlag::new("test");
        for slot in 0..SLOTS {
            flag.threads[slot].store(0x9000 + slot, Ordering::Release);
        }
        let _guard = flag.enter(THREAD_A);
        assert!(!flag.is_active(THREAD_A));
        assert_eq!(flag.occupancy.exhausted(), 1);
    }

    #[test]
    fn the_table_holds_more_threads_than_the_game_runs() {
        assert!(
            SLOTS >= 64,
            "a full table silently drops clone context; keep headroom over the process thread count"
        );
        let ctx = ThreadScopedKind::new("test");
        let guards: Vec<_> = (0..SLOTS).map(|n| ctx.enter(0x1000 + n, 7)).collect();
        for n in 0..SLOTS {
            assert_eq!(ctx.active(0x1000 + n), Some(7));
        }
        assert_eq!(ctx.occupancy.exhausted(), 0);
        drop(guards);
        assert_eq!(ctx.occupancy.live.load(Ordering::Acquire), 0);
    }
}
