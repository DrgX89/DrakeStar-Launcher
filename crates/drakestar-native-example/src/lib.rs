//! Mod nativo de prueba: cdylib que exporta `drakestar_mod_entry`.
//!
//! **A proposito no depende de `drakestar-core`**: solo replica las structs
//! `repr(C)` del ABI. Si un mod puede compilarse contra el contrato sin linkear
//! nada nuestro, el ABI es estable de verdad.
//!
//! Uso: `cargo build -p drakestar-native-example` deja el `.so` en
//! `target/debug/libdrakestar_native_example.so`.

use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicU32, Ordering};

/// ABI C que habla este mod (major 1, minor 0).
pub const ABI: u32 = 0x0200;

#[repr(C)]
pub struct HookTable {
    pub version: u32,
    pub userdata: *mut c_void,
    pub find: Option<extern "C" fn(userdata: *mut c_void, name: *const c_char) -> *const c_void>,
    pub log: Option<extern "C" fn(userdata: *mut c_void, level: u32, msg: *const c_char)>,
}

#[repr(C)]
pub struct ModVTable {
    pub abi_version: u32,
    pub init: Option<extern "C" fn(table: *const HookTable) -> i32>,
    pub shutdown: Option<extern "C" fn()>,
    pub tick: Option<extern "C" fn()>,
}

static TICKS: AtomicU32 = AtomicU32::new(0);

extern "C" fn init(table: *const HookTable) -> i32 {
    if table.is_null() {
        return 1;
    }
    // el mod verifica la version de la tabla antes de usarla: si el motor habla
    // otro major, no carga
    if unsafe { (*table).version } >> 8 != 2 {
        return 2;
    }
    let userdata = unsafe { (*table).userdata };
    if let Some(log) = unsafe { (*table).log } {
        let msg = b"mod nativo de prueba: init ok\0";
        log(userdata, 2, msg.as_ptr() as *const c_char);
    }
    // el mod tambien pregunta si el motor publica su hook favorito
    if let Some(find) = unsafe { (*table).find } {
        let hook = b"client:tick\0";
        if !find(userdata, hook.as_ptr() as *const c_char).is_null() {
            let msg = b"mod: el motor publica client:tick\0";
            if let Some(l) = unsafe { (*table).log } {
                l(userdata, 2, msg.as_ptr() as *const c_char);
            }
        }
    }
    0
}

extern "C" fn shutdown() {
    TICKS.store(0, Ordering::SeqCst);
}

extern "C" fn tick() {
    TICKS.fetch_add(1, Ordering::SeqCst);
}

/// Cuantos ticks corrieron: lo usa el test para verificar que el mod ejecuta.
#[no_mangle]
pub extern "C" fn drakestar_example_ticks() -> u32 {
    TICKS.load(Ordering::SeqCst)
}

static VT: ModVTable =
    ModVTable { abi_version: ABI, init: Some(init), shutdown: Some(shutdown), tick: Some(tick) };

/// Punto de entrada que busca el cargador.
#[no_mangle]
pub extern "C" fn drakestar_mod_entry() -> *const ModVTable {
    &VT
}
