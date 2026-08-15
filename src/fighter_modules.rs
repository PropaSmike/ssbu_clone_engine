use std::collections::HashSet;
use std::sync::Mutex;

use crate::custom_articles;

macro_rules! module_log {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        #[allow(unused_unsafe)]
        unsafe { crate::dbg_out(&text) };
        skyline::println!("{}", text);
    }};
}

const DYNAMIC_MODULE_MANAGER: usize = 0x5326cd0;
const FILESYSTEM: usize = 0x5331f20;
const MODULE_TREE: usize = 0x38;
const COMMAND_DEQUE: usize = 0x80;
const MANAGER_EVENT: usize = 0x50;
const COMMAND_LOAD: u32 = 3;
const MODULE_REFCOUNT: usize = 0x134;

#[skyline::from_offset(0x22b59c0)]
fn extend_deque(deque: *mut Deque);

#[skyline::from_offset(0x353e330)]
fn get_search_path_index(index: &mut u32, bytes: *const u8);

#[skyline::from_offset(0x353e4e0)]
fn get_file_path_from_search_path(search_path: u32) -> u32;

#[skyline::from_offset(0x3540450)]
fn add_to_res_service(filesystem: *mut u64, file_path: u32);

extern "C" {
    #[link_name = "_ZN2nn2os11SignalEventEPNS0_9EventTypeE"]
    fn signal_event(event: u64);

    #[link_name = "_ZNSt3__127__tree_balance_after_insertIPNS_16__tree_node_baseIPvEEEEvT_S5_"]
    fn balance_after_insert(root: *mut u8, node: *mut u8);
}

#[repr(C)]
struct Module {
    module_object: *mut u8,
    state: u64,
    nro: *mut u8,
    bss: *mut u8,
    _x20: *mut u8,
    source_buffer: *mut u8,
    name: [u8; 0x100],
    _x130: u8,
    _x131: u8,
    is_loaded: bool,
    _x133: u8,
    refcount: u32,
}

#[repr(C)]
struct Command {
    id: u32,
    arg: u64,
}

#[repr(C)]
struct Deque {
    start: *mut *mut Command,
    begin: *mut *mut Command,
    end: *mut *mut Command,
    end_cap: *mut *mut Command,
    start_index: usize,
    len: usize,
}

#[repr(C)]
struct TreeNode {
    left: *mut TreeNode,
    right: *mut TreeNode,
    parent: *mut TreeNode,
    is_black: bool,
    _padding: [u8; 7],
    key: u64,
    value: *mut Module,
}

#[repr(C)]
struct Tree {
    end: *mut TreeNode,
    root: *mut TreeNode,
    length: usize,
}

impl Tree {
    unsafe fn find(&self, key: u64) -> Option<*mut Module> {
        let mut node = self.root;
        while !node.is_null() {
            match key.cmp(&(*node).key) {
                std::cmp::Ordering::Less => node = (*node).left,
                std::cmp::Ordering::Greater => node = (*node).right,
                std::cmp::Ordering::Equal => return Some((*node).value),
            }
        }
        None
    }

    unsafe fn insert(&mut self, key: u64, value: *mut Module) {
        let mut parent = std::ptr::null_mut::<TreeNode>();
        let mut link: *mut *mut TreeNode = &mut self.root;

        while !(*link).is_null() {
            parent = *link;
            link = if key < (*parent).key {
                &mut (*parent).left
            } else {
                &mut (*parent).right
            };
        }

        let node = Box::leak(Box::new(TreeNode {
            right: std::ptr::null_mut(),
            left: std::ptr::null_mut(),
            parent,
            is_black: false,
            _padding: [0; 7],
            key,
            value,
        })) as *mut TreeNode;

        *link = node;

        if !self.end.is_null() && !(*self.end).left.is_null() {
            self.end = (*self.end).left;
        }

        balance_after_insert(self.root as *mut u8, node as *mut u8);
        self.length += 1;
    }
}

unsafe fn manager() -> *mut u64 {
    let slot = (crate::text_base() + DYNAMIC_MODULE_MANAGER) as *const *mut *mut u64;
    let outer = core::ptr::read_volatile(slot);
    if outer.is_null() {
        return core::ptr::null_mut();
    }
    core::ptr::read_volatile(outer)
}

unsafe fn module_tree(manager: *mut u64) -> *mut Tree {
    (manager as *mut u8).add(MODULE_TREE) as *mut Tree
}

unsafe fn filesystem() -> *mut u64 {
    core::ptr::read_volatile((crate::text_base() + FILESYSTEM) as *const *mut u64)
}

pub unsafe fn load_file_reporting(path: &str) -> u32 {
    let mut owned = path.as_bytes().to_vec();
    owned.push(0);

    let mut search_path = 0u32;
    get_search_path_index(&mut search_path, owned.as_ptr());
    let file_path = get_file_path_from_search_path(search_path);
    if file_path != 0xFFFFFF {
        add_to_res_service(filesystem(), file_path);
    }
    file_path
}

#[allow(dead_code)]
unsafe fn load_file(path: &str) {
    let mut owned = path.as_bytes().to_vec();
    owned.push(0);

    let mut search_path = 0u32;
    get_search_path_index(&mut search_path, owned.as_ptr());

    let file_path = get_file_path_from_search_path(search_path);
    if file_path != 0xFFFFFF {
        add_to_res_service(filesystem(), file_path);
    }
}

pub fn search_path_index(path: &str) -> u32 {
    let mut owned = path.as_bytes().to_vec();
    owned.push(0);
    unsafe {
        let mut search_path = 0u32;
        get_search_path_index(&mut search_path, owned.as_ptr());
        search_path
    }
}

pub fn path_exists(path: &str) -> bool {
    let mut owned = path.as_bytes().to_vec();
    owned.push(0);

    unsafe {
        let mut search_path = 0u32;
        get_search_path_index(&mut search_path, owned.as_ptr());
        get_file_path_from_search_path(search_path) != 0xFFFFFF
    }
}

fn requested() -> &'static Mutex<HashSet<i32>> {
    static REQUESTED: std::sync::OnceLock<Mutex<HashSet<i32>>> = std::sync::OnceLock::new();
    REQUESTED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn module_key(fighter_kind: i32) -> Option<(u64, String)> {
    let name = custom_articles::fighter_name(fighter_kind)?
        .to_str()
        .ok()?
        .to_owned();
    Some((crate::hash40(&name), name))
}

pub fn is_loaded(fighter_kind: i32) -> bool {
    let Some((key, _)) = module_key(fighter_kind) else {
        return false;
    };
    unsafe {
        let manager = manager();
        if manager.is_null() {
            return false;
        }
        (*module_tree(manager))
            .find(key)
            .is_some_and(|module| (*module).is_loaded)
    }
}

pub fn request(fighter_kind: i32) -> bool {
    let Some((key, name)) = module_key(fighter_kind) else {
        return false;
    };

    unsafe {
        let manager = manager();
        if manager.is_null() {
            return false;
        }

        let Ok(mut requested) = requested().lock() else {
            return false;
        };

        let tree = module_tree(manager);

        if let Some(module) = (*tree).find(key) {
            if requested.insert(fighter_kind) {
                (*module).refcount += 1;
                module_log!(
                    "[fightermodule] lua2cpp_{name} already present; refcount now {}",
                    (*module).refcount
                );
            }
            return true;
        }

        if !requested.insert(fighter_kind) {
            return true;
        }

        load_file(&format!("prebuilt:/nro/release/lua2cpp_{name}.nro"));

        let mut module_name = [0u8; 0x100];
        module_name[..name.len()].copy_from_slice(name.as_bytes());

        let module = Box::leak(Box::new(Module {
            module_object: std::ptr::null_mut(),
            state: 0,
            nro: std::ptr::null_mut(),
            bss: std::ptr::null_mut(),
            _x20: std::ptr::null_mut(),
            source_buffer: std::ptr::null_mut(),
            name: module_name,
            _x130: 0,
            _x131: 0,
            is_loaded: false,
            _x133: 0,
            refcount: 1,
        })) as *mut Module;
        debug_assert_eq!(
            core::mem::offset_of!(Module, refcount),
            MODULE_REFCOUNT,
            "Module layout no longer matches the game's"
        );

        (*tree).insert(key, module);

        let deque = &mut *((manager as *mut u8).add(COMMAND_DEQUE) as *mut Deque);
        let distance = if deque.end.offset_from(deque.start) != 0 {
            deque.end.offset_from(deque.start) * 0x100 - 1
        } else {
            0
        };

        let next_index = deque.start_index + deque.len;
        if next_index as isize == distance {
            extend_deque(deque);
        }

        *(*deque.start.add(next_index / 0x100)).add(next_index & 0xFF) = Command {
            id: COMMAND_LOAD,
            arg: module as u64,
        };
        deque.len += 1;

        signal_event(**((manager as *mut u8).add(MANAGER_EVENT) as *const *const u64));

        module_log!("[fightermodule] queued lua2cpp_{name}.nro (fighter kind {fighter_kind})");
        true
    }
}

pub fn ensure_loaded(fighter_kind: i32, timeout_ms: u32) -> bool {
    if is_loaded(fighter_kind) {
        return true;
    }
    if !request(fighter_kind) {
        return false;
    }

    for _ in 0..timeout_ms {
        if is_loaded(fighter_kind) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    module_log!(
        "[fightermodule] lua2cpp for fighter kind {fighter_kind} did not load within {timeout_ms}ms"
    );
    false
}
