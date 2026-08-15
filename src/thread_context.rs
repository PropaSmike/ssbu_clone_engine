use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

const SLOTS: usize = 16;

#[allow(clippy::declare_interior_mutable_const)]
const FREE_THREAD: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const NO_KIND: AtomicI32 = AtomicI32::new(-1);
#[allow(clippy::declare_interior_mutable_const)]
const ZERO_DEPTH: AtomicI32 = AtomicI32::new(0);

pub(crate) struct ThreadReentrancyFlag {
    threads: [AtomicUsize; SLOTS],
    depths: [AtomicI32; SLOTS],
}

impl ThreadReentrancyFlag {
    pub(crate) const fn new() -> Self {
        Self {
            threads: [FREE_THREAD; SLOTS],
            depths: [ZERO_DEPTH; SLOTS],
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
                depth.fetch_add(1, Ordering::AcqRel);
                return ReentrancyGuard {
                    depth: Some((depth, Some(slot))),
                };
            }
        }
        ReentrancyGuard { depth: None }
    }
}

pub(crate) struct ReentrancyGuard<'a> {
    depth: Option<(&'a AtomicI32, Option<&'a AtomicUsize>)>,
}

impl Drop for ReentrancyGuard<'_> {
    fn drop(&mut self) {
        let Some((depth, slot)) = self.depth else {
            return;
        };
        depth.fetch_sub(1, Ordering::AcqRel);
        if let Some(slot) = slot {
            slot.store(0, Ordering::Release);
        }
    }
}

pub(crate) struct ThreadScopedKind {
    threads: [AtomicUsize; SLOTS],
    kinds: [AtomicI32; SLOTS],
}

impl ThreadScopedKind {
    pub(crate) const fn new() -> Self {
        Self {
            threads: [FREE_THREAD; SLOTS],
            kinds: [NO_KIND; SLOTS],
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
                self.kinds[index].store(kind, Ordering::Release);
                return ScopedKindGuard {
                    restore: Some((&self.kinds[index], -1, Some(&self.threads[index]))),
                };
            }
        }

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
                self.kinds[index].store(kind, Ordering::Release);
                let result = callback();
                self.kinds[index].store(-1, Ordering::Release);
                self.threads[index].store(0, Ordering::Release);
                return result;
            }
        }

        callback()
    }
}

pub(crate) struct ScopedKindGuard<'a> {
    restore: Option<(&'a AtomicI32, i32, Option<&'a AtomicUsize>)>,
}

impl Drop for ScopedKindGuard<'_> {
    fn drop(&mut self) {
        let Some((kind, previous, slot)) = self.restore else {
            return;
        };
        kind.store(previous, Ordering::Release);
        if let Some(slot) = slot {
            slot.store(0, Ordering::Release);
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
        let flag = ThreadReentrancyFlag::new();
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
        let flag = ThreadReentrancyFlag::new();
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
        let flag = ThreadReentrancyFlag::new();
        for _ in 0..(SLOTS * 4) {
            let _guard = flag.enter(THREAD_A);
            assert!(flag.is_active(THREAD_A));
        }
        assert!(!flag.is_active(THREAD_A));
    }

    #[test]
    fn publishes_and_clears() {
        let ctx = ThreadScopedKind::new();
        assert_eq!(ctx.active(THREAD_A), None);
        ctx.scope(THREAD_A, 123, || {
            assert_eq!(ctx.active(THREAD_A), Some(123));
        });
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn nests_a_different_kind_on_the_same_thread() {
        let ctx = ThreadScopedKind::new();
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
        let ctx = ThreadScopedKind::new();
        ctx.scope(THREAD_A, 123, || {
            ctx.scope(THREAD_A, 123, || {
                assert_eq!(ctx.active(THREAD_A), Some(123));
            });
            assert_eq!(ctx.active(THREAD_A), Some(123));
        });
    }

    #[test]
    fn threads_are_independent_and_never_block() {
        let ctx = ThreadScopedKind::new();
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
        let ctx = ThreadScopedKind::new();
        let mut ran = false;
        ctx.scope(0, 123, || {
            ran = true;
            assert_eq!(ctx.active(0), None);
        });
        assert!(ran);
    }

    #[test]
    fn guard_publishes_and_clears() {
        let ctx = ThreadScopedKind::new();
        {
            let _guard = ctx.enter(THREAD_A, 123);
            assert_eq!(ctx.active(THREAD_A), Some(123));
        }
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn guard_nests_by_save_and_restore() {
        let ctx = ThreadScopedKind::new();
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
        let ctx = ThreadScopedKind::new();
        for _ in 0..(SLOTS * 4) {
            let _guard = ctx.enter(THREAD_A, 123);
            assert_eq!(ctx.active(THREAD_A), Some(123));
        }
        assert_eq!(ctx.active(THREAD_A), None);
    }

    #[test]
    fn guard_threads_are_independent() {
        let ctx = ThreadScopedKind::new();
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
        let ctx = ThreadScopedKind::new();
        for slot in 0..SLOTS {
            ctx.threads[slot].store(0x9000 + slot, Ordering::Release);
        }
        let _guard = ctx.enter(THREAD_A, 123);
        assert_eq!(ctx.active(THREAD_A), None, "a full table must not block");
    }

    #[test]
    fn guard_and_closure_forms_interoperate() {
        let ctx = ThreadScopedKind::new();
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
        let ctx = ThreadScopedKind::new();
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
}
