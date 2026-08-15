#![allow(non_snake_case)]

macro_rules! console_only {
    ($name:literal) => {
        panic!(concat!(
            $name,
            " is a game-process symbol; a host test must not reach it"
        ))
    };
}

#[no_mangle]
pub extern "C" fn A64HookFunction(
    _symbol: *const (),
    _replace: *const (),
    _result: *mut *const (),
) {
    console_only!("A64HookFunction")
}

#[no_mangle]
pub extern "C" fn getRegionAddress(_region: i32) -> *const () {
    console_only!("getRegionAddress")
}

#[no_mangle]
pub extern "C" fn skyline_tcp_send_raw(_bytes: *const u8, _length: u64) {
    console_only!("skyline_tcp_send_raw")
}

#[export_name = "_ZN2nn2os16GetCurrentThreadEv"]
pub extern "C" fn nn_os_get_current_thread() -> *mut () {
    console_only!("nn::os::GetCurrentThread")
}

#[export_name = "_ZN2nn2os11SignalEventEPNS0_9EventTypeE"]
pub extern "C" fn nn_os_signal_event(_event: *mut ()) {
    console_only!("nn::os::SignalEvent")
}

#[export_name = "_ZN2nn2ro12LookupSymbolEPmPKc"]
pub extern "C" fn nn_ro_lookup_symbol(_out: *mut u64, _name: *const u8) -> u64 {
    console_only!("nn::ro::LookupSymbol")
}

#[export_name = "_ZN3app16sv_battle_object15module_accessorEj"]
pub extern "C" fn sv_battle_object_module_accessor(_id: u32) -> *mut () {
    console_only!("sv_battle_object::module_accessor")
}

#[export_name = "_ZN3app16sv_battle_object9is_activeEj"]
pub extern "C" fn sv_battle_object_is_active(_id: u32) -> bool {
    console_only!("sv_battle_object::is_active")
}

#[export_name = "_ZN3app7utility8get_kindEPKNS_26BattleObjectModuleAccessorE"]
pub extern "C" fn utility_get_kind(_boma: *const ()) -> i32 {
    console_only!("utility::get_kind")
}

#[export_name = "_ZN3app8lua_bind24WorkModule__get_int_implEPNS_26BattleObjectModuleAccessorEi"]
pub extern "C" fn work_module_get_int_impl(_boma: *mut (), _id: i32) -> i32 {
    console_only!("WorkModule::get_int")
}

#[export_name = "_ZN7lua2cpp12L2CAgentBase18sv_set_status_funcERKN3lib8L2CValueES4_Pv"]
pub extern "C" fn l2c_agent_sv_set_status_func(
    _agent: *mut (),
    _kind: *const (),
    _line: *const (),
    _function: *mut (),
) {
    console_only!("L2CAgentBase::sv_set_status_func")
}

#[export_name = "_ZNSt3__127__tree_balance_after_insertIPNS_16__tree_node_baseIPvEEEEvT_S5_"]
pub extern "C" fn tree_balance_after_insert(_root: *mut (), _node: *mut ()) {
    console_only!("std::__tree_balance_after_insert")
}
