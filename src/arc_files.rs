const ARC_SERVICE_GLOBAL: usize = 0x5331F20;
const ARC_SERVICE_ARC: usize = 0x78;
const ARC_FS_HEADER: usize = 0x40;
const ARC_FILE_PATHS: usize = 0x60;
const FS_HEADER_PATH_COUNT: usize = 0x04;
const FILE_PATH_STRIDE: usize = 0x20;
const VANILLA_FILE_COUNT: usize = 590_711;
const RESOURCE_NONE: u32 = 0x00FF_FFFF;
const DATA_RECORD_STRIDE: usize = 0x18;
const DATA_RECORD_STATE: usize = 0xd;
const STATE_LOADED: u8 = 3;
const MAX_COPY: usize = 64 << 20;

type DecompressedSizeFn = unsafe extern "C" fn(u64, *mut usize) -> bool;

unsafe fn service() -> Option<usize> {
    let service = core::ptr::read_volatile((crate::game::addr(ARC_SERVICE_GLOBAL)) as *const usize);
    (service != 0).then_some(service)
}

unsafe fn loaded_arc() -> Option<usize> {
    let holder = core::ptr::read_volatile((service()? + ARC_SERVICE_ARC) as *const usize);
    if holder == 0 {
        return None;
    }
    let arc = core::ptr::read_volatile(holder as *const usize);
    (arc != 0).then_some(arc)
}

pub(crate) unsafe fn file_path_index(hash: u64) -> Option<u32> {
    let arc = loaded_arc()?;
    let header = core::ptr::read_volatile((arc + ARC_FS_HEADER) as *const usize);
    let paths = core::ptr::read_volatile((arc + ARC_FILE_PATHS) as *const usize);
    if header == 0 || paths == 0 {
        return None;
    }
    let count = core::ptr::read_volatile((header + FS_HEADER_PATH_COUNT) as *const u32) as usize;
    if !(VANILLA_FILE_COUNT..=VANILLA_FILE_COUNT * 4).contains(&count) {
        return None;
    }
    for index in (0..count).rev() {
        let entry = paths + index * FILE_PATH_STRIDE;
        let low = core::ptr::read_volatile(entry as *const u32) as u64;
        let length = core::ptr::read_volatile((entry + 4) as *const u8) as u64;
        if (length << 32) | low == hash {
            return Some(index as u32);
        }
    }
    None
}

pub(crate) unsafe fn loaded_data(index: u32) -> Option<usize> {
    if index == RESOURCE_NONE {
        return None;
    }
    let service = service()?;
    if core::ptr::read_volatile((service + 0x18) as *const u32) <= index {
        return None;
    }
    let table = core::ptr::read_volatile((service + 0x08) as *const usize);
    if table == 0 || core::ptr::read_volatile((table + index as usize * 8 + 4) as *const u8) == 0 {
        return None;
    }
    let data_index = core::ptr::read_volatile((table + index as usize * 8) as *const u32);
    if data_index == RESOURCE_NONE || core::ptr::read_volatile((service + 0x1c) as *const u32) <= data_index {
        return None;
    }
    let records = core::ptr::read_volatile((service + 0x10) as *const usize);
    if records == 0 {
        return None;
    }
    let record = records + data_index as usize * DATA_RECORD_STRIDE;
    if core::ptr::read_volatile((record + DATA_RECORD_STATE) as *const u8) != STATE_LOADED {
        return None;
    }
    let data = core::ptr::read_volatile(record as *const usize);
    (data != 0).then_some(data)
}

pub(crate) fn decompressed_size(hash: u64) -> Option<usize> {
    let address = unsafe { crate::css_registration::lookup_symbol(b"arcrop_get_decompressed_size\0") }?;
    let function: DecompressedSizeFn = unsafe { core::mem::transmute(address) };
    let mut size = 0usize;
    (unsafe { function(hash, &mut size) } && size != 0 && size <= MAX_COPY).then_some(size)
}

pub(crate) unsafe fn loaded_copy(hash: u64, index: u32) -> Option<Vec<u8>> {
    let data = loaded_data(index)?;
    let size = decompressed_size(hash)?;
    Some(core::slice::from_raw_parts(data as *const u8, size).to_vec())
}
