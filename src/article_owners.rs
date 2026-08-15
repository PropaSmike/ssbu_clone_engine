use std::sync::RwLock;

const MAX_RECORDS: usize = 128;

#[derive(Clone, Copy)]
struct Record {
    object_id: u32,
    weapon_kind: i32,
    clone_kind: i32,
}

#[derive(Clone, Copy)]
struct Death {
    weapon_kind: i32,
    clone_kind: i32,
    ticks_left: u8,
}

const DEATH_TTL: u8 = 4;

struct Table {
    records: Vec<Record>,
    cursor: usize,
    deaths: Vec<Death>,
}

fn table() -> &'static RwLock<Table> {
    static TABLE: std::sync::OnceLock<RwLock<Table>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        RwLock::new(Table {
            records: Vec::new(),
            cursor: 0,
            deaths: Vec::new(),
        })
    })
}

static COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

pub(crate) fn remember(object_id: u32, weapon_kind: i32, clone_kind: i32) {
    if object_id == 0 || clone_kind < 0 {
        return;
    }
    let Ok(mut table) = table().write() else {
        return;
    };
    if let Some(existing) = table
        .records
        .iter_mut()
        .find(|record| record.object_id == object_id)
    {
        existing.weapon_kind = weapon_kind;
        existing.clone_kind = clone_kind;
        return;
    }
    let record = Record {
        object_id,
        weapon_kind,
        clone_kind,
    };
    if table.records.len() < MAX_RECORDS {
        table.records.push(record);
    } else {
        let cursor = table.cursor;
        table.records[cursor] = record;
        table.cursor = (cursor + 1) % MAX_RECORDS;
    }
    COUNT.store(table.records.len(), core::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn sole_owner_of_kind(weapon_kind: i32) -> Option<i32> {
    if COUNT.load(core::sync::atomic::Ordering::Relaxed) == 0 {
        return None;
    }
    let table = table().read().ok()?;
    let mut owner = None;
    for record in table
        .records
        .iter()
        .filter(|record| record.weapon_kind == weapon_kind)
    {
        match owner {
            None => owner = Some(record.clone_kind),
            Some(existing) if existing == record.clone_kind => {}
            Some(_) => return None,
        }
    }
    owner
}

pub(crate) fn sweep(is_active: impl Fn(u32) -> bool) {
    let Ok(mut table) = table().write() else {
        return;
    };

    table.deaths.retain_mut(|death| {
        death.ticks_left = death.ticks_left.saturating_sub(1);
        death.ticks_left != 0
    });

    let mut died = Vec::new();
    table.records.retain(|record| {
        if is_active(record.object_id) {
            return true;
        }
        died.push((record.weapon_kind, record.clone_kind));
        false
    });
    for (weapon_kind, clone_kind) in died {
        table.deaths.push(Death {
            weapon_kind,
            clone_kind,
            ticks_left: DEATH_TTL,
        });
    }
    table.cursor = 0;
    COUNT.store(table.records.len(), core::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn recently_died(weapon_kind: i32) -> Option<i32> {
    let table = table().read().ok()?;
    table
        .deaths
        .iter()
        .rev()
        .find(|death| death.weapon_kind == weapon_kind)
        .map(|death| death.clone_kind)
}

pub(crate) fn is_tracked(object_id: u32) -> bool {
    if object_id == 0 || COUNT.load(core::sync::atomic::Ordering::Relaxed) == 0 {
        return false;
    }
    let Ok(table) = table().read() else {
        return false;
    };
    table
        .records
        .iter()
        .any(|record| record.object_id == object_id)
}

pub(crate) fn len() -> usize {
    COUNT.load(core::sync::atomic::Ordering::Relaxed)
}
