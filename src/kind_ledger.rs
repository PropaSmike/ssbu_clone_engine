use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const LEDGER_DIRECTORY: &str = "sd:/ultimate/clone_engine";
const LEDGER_PATH: &str = "sd:/ultimate/clone_engine/kinds.txt";

const MAX_ENTRIES: usize = 512;

struct Ledger {
    reserved: HashMap<String, i32>,
    loaded: bool,
    dirty: bool,
    write_failed: bool,
}

static LEDGER: OnceLock<Mutex<Ledger>> = OnceLock::new();

fn ledger() -> &'static Mutex<Ledger> {
    LEDGER.get_or_init(|| {
        Mutex::new(Ledger {
            reserved: HashMap::new(),
            loaded: false,
            dirty: false,
            write_failed: false,
        })
    })
}

fn parse(contents: &str) -> HashMap<String, i32> {
    let mut reserved = HashMap::new();
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(kind), Some(identity)) = (fields.next(), fields.next()) else {
            continue;
        };
        if fields.next().is_some() {
            continue;
        }
        let Ok(kind) = kind.parse::<i32>() else {
            continue;
        };
        if kind < 0 || reserved.len() >= MAX_ENTRIES {
            continue;
        }
        reserved.insert(identity.to_string(), kind);
    }
    reserved
}

fn serialise(reserved: &HashMap<String, i32>) -> String {
    let mut entries = reserved.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(identity, kind)| (**kind, (*identity).clone()));
    let mut out = String::from(
        "# SSBU clone engine kind ledger.\n\
         # <kind> <identity>, one per line. Edit to pin a moveset to a number;\n\
         # a line is ignored when that kind is already taken this boot.\n",
    );
    for (identity, kind) in entries {
        out.push_str(&format!("{kind} {identity}\n"));
    }
    out
}

fn load_locked(ledger: &mut Ledger) {
    if ledger.loaded {
        return;
    }
    ledger.loaded = true;
    match std::fs::read_to_string(LEDGER_PATH) {
        Ok(contents) => {
            ledger.reserved = parse(&contents);
            skyline::println!(
                "[kindledger] loaded {} reservation(s) from {LEDGER_PATH}",
                ledger.reserved.len()
            );
        }
        Err(_) => {
            ledger.reserved = HashMap::new();
        }
    }
}

fn flush_locked(ledger: &mut Ledger) {
    if !ledger.dirty {
        return;
    }
    let contents = serialise(&ledger.reserved);
    let _ = std::fs::create_dir_all(LEDGER_DIRECTORY);
    match std::fs::write(LEDGER_PATH, contents) {
        Ok(()) => {
            ledger.dirty = false;
            skyline::println!(
                "[kindledger] wrote {} reservation(s) to {LEDGER_PATH}",
                ledger.reserved.len()
            );
        }
        Err(error) => {
            if !ledger.write_failed {
                ledger.write_failed = true;
                skyline::println!(
                    "[kindledger] cannot write {LEDGER_PATH} ({error}); kinds stay conflict-free but will not persist"
                );
            }
        }
    }
}

pub(crate) fn reserved_kind(identity: &str) -> Option<i32> {
    let mut ledger = ledger().lock().unwrap();
    load_locked(&mut ledger);
    ledger.reserved.get(identity).copied()
}

pub(crate) fn record(identity: &str, kind: i32) {
    if identity.is_empty() || identity.split_whitespace().count() != 1 {
        return;
    }
    let mut ledger = ledger().lock().unwrap();
    load_locked(&mut ledger);
    if ledger.reserved.get(identity) == Some(&kind) {
        return;
    }
    if ledger.reserved.len() >= MAX_ENTRIES && !ledger.reserved.contains_key(identity) {
        return;
    }
    ledger.reserved.insert(identity.to_string(), kind);
    ledger.dirty = true;
    flush_locked(&mut ledger);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_reservations() {
        let mut reserved = HashMap::new();
        reserved.insert("fighter_kind_example".to_string(), 119);
        reserved.insert("fighter_kind_clone_mario".to_string(), 121);
        let parsed = parse(&serialise(&reserved));
        assert_eq!(parsed, reserved);
    }

    #[test]
    fn skips_malformed_lines_without_losing_good_ones() {
        let parsed = parse(
            "# comment\n\
             \n\
             121 fighter_kind_clone_mario\n\
             not_a_number fighter_kind_bad\n\
             122\n\
             123 fighter_kind_extra trailing_field\n\
             -5 fighter_kind_negative\n\
             124 fighter_kind_ok # trailing comment\n",
        );
        assert_eq!(parsed.get("fighter_kind_clone_mario"), Some(&121));
        assert_eq!(parsed.get("fighter_kind_ok"), Some(&124));
        assert_eq!(parsed.get("fighter_kind_bad"), None);
        assert_eq!(parsed.get("fighter_kind_extra"), None);
        assert_eq!(parsed.get("fighter_kind_negative"), None);
        assert_eq!(parsed.len(), 2);
    }
}
