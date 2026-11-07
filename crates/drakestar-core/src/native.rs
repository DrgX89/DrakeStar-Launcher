//! El ABI C estable de los mods nativos.
//!
//! Un mod nativo es un `cdylib` que exporta `drakestar_mod_entry`. Aqui se
//! define el contrato: si algo de este archivo cambia de forma incompatible,
//! el `major` del ABI sube y los mods viejos dejan de cargar (que es exactamente
//! para lo que existe el versionado).
//!
//! Reglas que no se rompen sin subir el major:
//! - no se cambia el orden ni el tipo de los campos de las structs `repr(C)`;
//! - no se quitan funciones de las tablas;
//! - agregar campos **al final** o funciones nuevas con `Option` en `None` es
//!   compatible (los mods viejos las ignoran).

use std::os::raw::c_void;

/// Major del ABI C. Sube solo si se rompe el contrato.
///
/// v1: `find`/`log` sin `userdata`. v2: la tabla lleva `userdata` y las funciones
/// lo reciben para poder consultar el estado del motor. Como cambia la firma,
/// los mods de v1 **no cargan** (para eso esta el versionado).
pub const ABI_C_MAJOR: u32 = 2;

/// Simbolo que exporta todo mod nativo.
pub const MOD_ENTRY_SYMBOL: &str = "drakestar_mod_entry";

/// Nivel de log que acepta el motor.
pub const LOG_ERROR: u32 = 0;
pub const LOG_WARN: u32 = 1;
pub const LOG_INFO: u32 = 2;
pub const LOG_DEBUG: u32 = 3;

/// Tabla que publica el motor. El mod la recibe en `init` y puede consultarla;
/// no es dueña de ella (vive lo que el motor).
#[repr(C)]
pub struct HookTable {
    /// Version del ABI de la tabla. El mod debe rechazarla si no la reconoce.
    pub version: u32,
    /// Puntero al estado del motor (opaco para el mod): se lo pasan `find` y
    /// `log` para que puedan consultarlo. Patron C estandar.
    pub userdata: *mut c_void,
    /// Busca un hook por nombre (`"client:tick"`). `null` si no existe.
    /// El puntero devuelto es del motor: no liberarlo.
    pub find: Option<extern "C" fn(userdata: *mut c_void, name: *const std::os::raw::c_char) -> *const c_void>,
    /// Log al servicio del motor (nivel + mensaje UTF-8, `\0` al final).
    pub log: Option<extern "C" fn(userdata: *mut c_void, level: u32, msg: *const std::os::raw::c_char)>,
}

/// Lo que devuelve `drakestar_mod_entry`: la tabla de funciones del mod.
#[repr(C)]
pub struct ModVTable {
    /// ABI contra el que compilo el mod. El cargador lo verifica **antes** de
    /// llamar cualquier otra funcion.
    pub abi_version: u32,
    /// `init`: recibe la tabla de hooks. Devuelve 0 si ok, otro si falla.
    pub init: Option<extern "C" fn(table: *const HookTable) -> i32>,
    /// `shutdown`: al descargar el mod (o al cerrar el juego).
    pub shutdown: Option<extern "C" fn()>,
    /// `tick`: 20 veces por segundo, si el mod lo pidio.
    pub tick: Option<extern "C" fn()>,
}

/// Firma del punto de entrada.
pub type ModEntryFn = extern "C" fn() -> *const ModVTable;

/// Verifica que la version del ABI de un mod sea cargable por este cargador.
/// Misma regla que el intermediario: mismo major, minor del cargador >= minor
/// del mod.
pub const fn abi_ok(mod_abi: u32) -> bool {
    mod_abi >> 8 == ABI_C_MAJOR
}

/// Version legible desde el entero empaquetado (`1.0` = 0x0100).
pub const fn abi_parts(v: u32) -> (u32, u32) {
    ((v >> 8) & 0xff, v & 0xff)
}

/// Empaqueta major/minor como lo espera el ABI.
pub const fn abi_pack(major: u32, minor: u32) -> u32 {
    (major << 8) | (minor & 0xff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn abi_versioning_packs_and_compares() {
        let motor = ABI_C_MAJOR; // 2
        assert_eq!(abi_parts(abi_pack(2, 0)), (2, 0));
        assert_eq!(abi_pack(2, 3), 0x0203);
        // mismo major que el motor: cargable
        assert!(abi_ok(abi_pack(2, 0)));
        assert!(abi_ok(abi_pack(2, 9)));
        // v1 quedo atras (find/log sin userdata) y v3 no existe todavia
        assert!(!abi_ok(abi_pack(1, 0)));
        assert!(!abi_ok(abi_pack(3, 0)));
        assert!(!abi_ok(abi_pack(0, 9)));
        assert_eq!(motor, 2);
    }

    fn layout_is_stable() {
        // estos tamanos son parte del contrato: si cambian, hay que subir el ABI.
        // HookTable = u32 + userdata + 2 punteros; ModVTable = u32 + 3 punteros.
        assert_eq!(size_of::<HookTable>(), 32); // u32 + userdata + 2 punteros
        assert_eq!(size_of::<ModVTable>(), 32);
        assert_eq!(align_of::<HookTable>(), 8);
        assert_eq!(align_of::<ModVTable>(), 8);
    }

    #[test]
    fn entry_point_is_c_callable() {
        // un mod de prueba real: compila esta funcion y el cargador la busca
        extern "C" fn entry() -> *const ModVTable {
            static VT: ModVTable =
                ModVTable { abi_version: abi_pack(2, 0), init: None, shutdown: None, tick: None };
            &VT
        }
        let ptr = entry();
        assert!(!ptr.is_null());
        assert!(abi_ok(unsafe { (*ptr).abi_version }));
    }
}
