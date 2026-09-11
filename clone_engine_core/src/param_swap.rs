use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

use crate::slots::{base_row, clone_row, CLONE_SLOTS, PARAM_NATIVE_KINDS};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Instance {
    pub payload: usize,
    pub owner: usize,
    pub payload_b: usize,
    pub owner_b: usize,
}

impl Instance {
    pub const EMPTY: Self = Self {
        payload: 0,
        owner: 0,
        payload_b: 0,
        owner_b: 0,
    };

    pub fn payload(&self) -> usize {
        self.payload
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GuardMode {
    Idle,
    Restore,
    CaptureClone,
    CaptureVanilla,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LockOutcome {
    Acquired,
    Reentrant,
    Contended,
}

impl LockOutcome {
    pub fn owned(self) -> bool {
        matches!(self, LockOutcome::Acquired)
    }
}

pub struct BaseState {
    pub origin: AtomicUsize,
    pub vanilla_payload: AtomicUsize,
    pub vanilla_owner: AtomicUsize,
    pub vanilla_payload_b: AtomicUsize,
    pub vanilla_owner_b: AtomicUsize,
    pub separated: AtomicBool,
    pub saw_clone: AtomicBool,
    pub vanilla_own_context: AtomicBool,
}

impl BaseState {
    pub const fn new() -> Self {
        Self {
            origin: AtomicUsize::new(0),
            vanilla_payload: AtomicUsize::new(0),
            vanilla_owner: AtomicUsize::new(0),
            vanilla_payload_b: AtomicUsize::new(0),
            vanilla_owner_b: AtomicUsize::new(0),
            separated: AtomicBool::new(false),
            saw_clone: AtomicBool::new(false),
            vanilla_own_context: AtomicBool::new(false),
        }
    }

    pub fn forget(&self) {
        self.origin.store(0, Ordering::Relaxed);
        self.vanilla_payload.store(0, Ordering::Relaxed);
        self.vanilla_owner.store(0, Ordering::Relaxed);
        self.vanilla_payload_b.store(0, Ordering::Relaxed);
        self.vanilla_owner_b.store(0, Ordering::Relaxed);
        self.separated.store(false, Ordering::Relaxed);
        self.saw_clone.store(false, Ordering::Relaxed);
        self.vanilla_own_context.store(false, Ordering::Relaxed);
    }
}

impl Default for BaseState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CloneState {
    pub payload: AtomicUsize,
    pub owner: AtomicUsize,
    pub payload_b: AtomicUsize,
    pub owner_b: AtomicUsize,
}

impl CloneState {
    pub const fn new() -> Self {
        Self {
            payload: AtomicUsize::new(0),
            owner: AtomicUsize::new(0),
            payload_b: AtomicUsize::new(0),
            owner_b: AtomicUsize::new(0),
        }
    }

    pub fn forget(&self) {
        self.payload.store(0, Ordering::Relaxed);
        self.owner.store(0, Ordering::Relaxed);
        self.payload_b.store(0, Ordering::Relaxed);
        self.owner_b.store(0, Ordering::Relaxed);
    }
}

impl Default for CloneState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SwapRegistry {
    bases: [BaseState; PARAM_NATIVE_KINDS as usize],
    clones: [CloneState; CLONE_SLOTS],
    peak_clone_slot: AtomicI32,
    out_of_range: AtomicU32,
    lock: AtomicBool,
    owner: AtomicUsize,
}

impl SwapRegistry {
    pub const fn new() -> Self {
        Self {
            bases: [const { BaseState::new() }; PARAM_NATIVE_KINDS as usize],
            clones: [const { CloneState::new() }; CLONE_SLOTS],
            peak_clone_slot: AtomicI32::new(-1),
            out_of_range: AtomicU32::new(0),
            lock: AtomicBool::new(false),
            owner: AtomicUsize::new(0),
        }
    }

    pub fn base(&self, base_kind: i32) -> Option<&BaseState> {
        base_row(base_kind).and_then(|row| self.bases.get(row))
    }

    pub fn clone_slot(&self, clone_kind: i32) -> Option<&CloneState> {
        let Some(row) = clone_row(clone_kind) else {
            self.out_of_range.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        let Some(state) = self.clones.get(row) else {
            self.out_of_range.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        self.note_peak(row as i32);
        Some(state)
    }

    fn note_peak(&self, seen: i32) {
        let mut peak = self.peak_clone_slot.load(Ordering::Relaxed);
        while seen > peak {
            match self.peak_clone_slot.compare_exchange_weak(
                peak,
                seen,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => peak = observed,
            }
        }
    }

    pub fn peak(&self) -> (i32, u32) {
        (
            self.peak_clone_slot.load(Ordering::Relaxed),
            self.out_of_range.load(Ordering::Relaxed),
        )
    }

    pub fn try_lock(&self, thread: usize, spin_limit: u32) -> LockOutcome {
        if thread != 0 && self.owner.load(Ordering::Acquire) == thread {
            return LockOutcome::Reentrant;
        }
        for _ in 0..spin_limit {
            if self
                .lock
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                self.owner.store(thread, Ordering::Release);
                return LockOutcome::Acquired;
            }
            core::hint::spin_loop();
        }
        LockOutcome::Contended
    }

    pub fn unlock(&self, owned: bool) {
        if !owned {
            return;
        }
        self.owner.store(0, Ordering::Release);
        self.lock.store(false, Ordering::Release);
    }

    pub fn bases(&self) -> impl Iterator<Item = &BaseState> {
        self.bases.iter()
    }

    pub fn clones(&self) -> impl Iterator<Item = &CloneState> {
        self.clones.iter()
    }

    pub fn describe_root(&self, root: usize) -> &'static str {
        if root == 0 {
            return "unknown";
        }
        for state in self.bases() {
            if root == state.vanilla_payload.load(Ordering::Relaxed) {
                return "base-instance";
            }
            if root == state.origin.load(Ordering::Relaxed) {
                return "origin";
            }
        }
        for state in self.clones() {
            if root == state.payload.load(Ordering::Relaxed) {
                return "clone-instance";
            }
        }
        "unknown"
    }

    pub fn known_payload(&self, base_kind: i32, payload: usize, clone_kinds: &[i32]) -> bool {
        let Some(state) = self.base(base_kind) else {
            return false;
        };
        if payload == state.origin.load(Ordering::Relaxed)
            || payload == state.vanilla_payload.load(Ordering::Relaxed)
        {
            return true;
        }
        clone_kinds.iter().any(|kind| {
            self.clone_slot(*kind)
                .is_some_and(|clone| clone.payload.load(Ordering::Relaxed) == payload)
        })
    }

    pub fn forget_all(&self, base_kind: i32, clone_kinds: &[i32]) {
        if let Some(state) = self.base(base_kind) {
            state.forget();
        }
        for kind in clone_kinds {
            if let Some(clone) = self.clone_slot(*kind) {
                clone.forget();
            }
        }
    }
}

impl Default for SwapRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slots::FIRST_CUSTOM_KIND;

    #[test]
    fn a_base_kind_never_resolves_to_a_clone_slot_and_back() {
        let registry = SwapRegistry::new();
        for kind in 0..PARAM_NATIVE_KINDS {
            assert!(registry.base(kind).is_some(), "base {kind}");
            assert!(registry.clone_slot(kind).is_none(), "clone {kind}");
        }
        for kind in FIRST_CUSTOM_KIND..FIRST_CUSTOM_KIND + CLONE_SLOTS as i32 {
            assert!(registry.clone_slot(kind).is_some(), "clone {kind}");
            assert!(registry.base(kind).is_none(), "base {kind}");
        }
    }

    #[test]
    fn a_negative_kind_is_rejected_and_counted_not_wrapped() {
        let registry = SwapRegistry::new();
        assert!(registry.base(-1).is_none());
        assert!(registry.clone_slot(-1).is_none());
        assert!(registry.base(i32::MIN).is_none());
        assert!(registry.clone_slot(i32::MIN).is_none());
        assert_eq!(registry.peak(), (-1, 2));
    }

    #[test]
    fn the_peak_slot_only_ever_rises() {
        let registry = SwapRegistry::new();
        assert_eq!(registry.peak().0, -1);
        registry.clone_slot(FIRST_CUSTOM_KIND + 5);
        assert_eq!(registry.peak().0, 5);
        registry.clone_slot(FIRST_CUSTOM_KIND + 1);
        assert_eq!(registry.peak().0, 5);
        registry.clone_slot(FIRST_CUSTOM_KIND + 9);
        assert_eq!(registry.peak().0, 9);
    }

    #[test]
    fn the_same_thread_re_entering_does_not_take_the_lock_twice() {
        let registry = SwapRegistry::new();
        assert_eq!(registry.try_lock(0x1234, 16), LockOutcome::Acquired);
        assert_eq!(registry.try_lock(0x1234, 16), LockOutcome::Reentrant);
        assert!(!LockOutcome::Reentrant.owned());
        registry.unlock(false);
        assert_eq!(registry.try_lock(0x1234, 16), LockOutcome::Reentrant);
        registry.unlock(true);
        assert_eq!(registry.try_lock(0x1234, 16), LockOutcome::Acquired);
    }

    #[test]
    fn another_thread_gives_up_after_the_spin_budget_and_takes_no_ownership() {
        let registry = SwapRegistry::new();
        assert_eq!(registry.try_lock(0xaaaa, 16), LockOutcome::Acquired);
        assert_eq!(registry.try_lock(0xbbbb, 16), LockOutcome::Contended);
        assert_eq!(registry.owner.load(Ordering::Relaxed), 0xaaaa);
        registry.unlock(false);
        assert_eq!(registry.owner.load(Ordering::Relaxed), 0xaaaa);
        registry.unlock(true);
        assert_eq!(registry.try_lock(0xbbbb, 16), LockOutcome::Acquired);
    }

    #[test]
    fn a_zero_thread_key_never_counts_as_re_entrant() {
        let registry = SwapRegistry::new();
        assert_eq!(registry.try_lock(0, 16), LockOutcome::Acquired);
        assert_eq!(registry.try_lock(0, 16), LockOutcome::Contended);
    }

    #[test]
    fn a_payload_is_known_from_the_base_or_any_of_its_clones() {
        let registry = SwapRegistry::new();
        let base = registry.base(3).unwrap();
        base.origin.store(0x1000, Ordering::Relaxed);
        base.vanilla_payload.store(0x2000, Ordering::Relaxed);
        let clones = [FIRST_CUSTOM_KIND, FIRST_CUSTOM_KIND + 7];
        registry
            .clone_slot(clones[1])
            .unwrap()
            .payload
            .store(0x3000, Ordering::Relaxed);

        assert!(registry.known_payload(3, 0x1000, &clones));
        assert!(registry.known_payload(3, 0x2000, &clones));
        assert!(registry.known_payload(3, 0x3000, &clones));
        assert!(!registry.known_payload(3, 0x4000, &clones));
        assert!(!registry.known_payload(3, 0x3000, &clones[..1]));
    }

    #[test]
    fn forget_all_clears_the_base_and_only_the_listed_clones() {
        let registry = SwapRegistry::new();
        let base = registry.base(3).unwrap();
        base.origin.store(0x1000, Ordering::Relaxed);
        base.separated.store(true, Ordering::Relaxed);
        let mine = FIRST_CUSTOM_KIND + 2;
        let other = FIRST_CUSTOM_KIND + 3;
        registry
            .clone_slot(mine)
            .unwrap()
            .payload
            .store(0x5000, Ordering::Relaxed);
        registry
            .clone_slot(other)
            .unwrap()
            .payload
            .store(0x6000, Ordering::Relaxed);

        registry.forget_all(3, &[mine]);

        assert_eq!(base.origin.load(Ordering::Relaxed), 0);
        assert!(!base.separated.load(Ordering::Relaxed));
        assert_eq!(
            registry.clone_slot(mine).unwrap().payload.load(Ordering::Relaxed),
            0
        );
        assert_eq!(
            registry.clone_slot(other).unwrap().payload.load(Ordering::Relaxed),
            0x6000
        );
    }

    #[test]
    fn forget_all_on_an_unknown_base_touches_nothing_and_does_not_panic() {
        let registry = SwapRegistry::new();
        let mine = FIRST_CUSTOM_KIND + 4;
        registry
            .clone_slot(mine)
            .unwrap()
            .payload
            .store(0x7000, Ordering::Relaxed);
        registry.forget_all(PARAM_NATIVE_KINDS, &[]);
        assert_eq!(
            registry.clone_slot(mine).unwrap().payload.load(Ordering::Relaxed),
            0x7000
        );
    }

    #[test]
    fn describe_root_names_the_owner_and_never_matches_a_zero_root() {
        let registry = SwapRegistry::new();
        assert_eq!(registry.describe_root(0), "unknown");
        registry
            .base(3)
            .unwrap()
            .vanilla_payload
            .store(0x1000, Ordering::Relaxed);
        registry.base(4).unwrap().origin.store(0x2000, Ordering::Relaxed);
        registry
            .clone_slot(FIRST_CUSTOM_KIND)
            .unwrap()
            .payload
            .store(0x3000, Ordering::Relaxed);
        assert_eq!(registry.describe_root(0x1000), "base-instance");
        assert_eq!(registry.describe_root(0x2000), "origin");
        assert_eq!(registry.describe_root(0x3000), "clone-instance");
        assert_eq!(registry.describe_root(0x9999), "unknown");
    }

    #[test]
    fn an_empty_instance_is_distinguishable_from_a_populated_one() {
        assert!(Instance::EMPTY.is_empty());
        assert_eq!(Instance::default(), Instance::EMPTY);
        let live = Instance {
            payload: 0x10,
            ..Instance::EMPTY
        };
        assert!(!live.is_empty());
        assert_eq!(live.payload(), 0x10);
        assert_ne!(live, Instance::EMPTY);
    }
}
