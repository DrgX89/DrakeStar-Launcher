//! Cargador de mods nativos: cdylib contra el ABI C estable.
//!
//! Un mod nativo viaja como jar (igual que los de Java) con su `fabric.mod.json`
//! y el binario dentro. El cargador lo descubre, verifica el SHA-256 como a
//! cualquier mod, y **antes de llamar una sola funcion** comprueba la version
//! del ABI: si el mod es de un major desconocido, no se carga.
//!
//! Los `Library` se guardan en el proceso: descargarlos es perder el codigo.

use crate::{AppError, AppResult};
use drakestar_core::native::{self, ModVTable};
pub use drakestar_core::native::HookTable;
use std::collections::BTreeMap;
use std::io::Read;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Nombre del binario dentro del jar, segun la plataforma.
pub fn native_lib_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "drakestar_mod.dll"
    } else if cfg!(target_os = "macos") {
        "drakestar_mod.dylib"
    } else {
        "drakestar_mod.so"
    }
}

/// Un mod nativo cargado: la libreria (que debe vivir) y su tabla.
#[derive(Debug)]
pub struct LoadedMod {
    pub id: String,
    pub abi: u32,
    pub dir: PathBuf,
    _lib: libloading::Library,
    vtable: *const ModVTable,
}

// el puntero apunta a memoria de la libreria, que tenemos viva en _lib
unsafe impl Send for LoadedMod {}
unsafe impl Sync for LoadedMod {}

impl LoadedMod {
    /// Version del ABI del mod, ya verificado.
    pub fn abi_label(&self) -> String {
        let (maj, min) = native::abi_parts(self.abi);
        format!("{maj}.{min}")
    }

    /// Llama `init` con la tabla del motor. Devuelve el codigo de error del mod.
    pub fn init(&self, table: &HookTable) -> AppResult<i32> {
        let f = unsafe { (*self.vtable).init }
            .ok_or_else(|| AppError::Invalid(format!("'{}' no declara init", self.id)))?;
        Ok(f(table as *const HookTable))
    }

    pub fn shutdown(&self) -> AppResult<()> {
        if let Some(f) = unsafe { (*self.vtable).shutdown } {
            f();
        }
        Ok(())
    }

    pub fn tick(&self) -> AppResult<()> {
        if let Some(f) = unsafe { (*self.vtable).tick } {
            f();
        }
        Ok(())
    }
}

/// Carga un mod nativo desde su jar: extrae el binario a un directorio propio
/// (para que el `.so` no quede suelto en mods/) y lo abre con `libloading`.
pub fn load_from_jar(jar: &Path, id: &str, dest_root: &Path) -> AppResult<LoadedMod> {
    let f = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(f)
        .map_err(|e| AppError::Invalid(format!("{}: no es un jar leible ({e})", jar.display())))?;
    let name = native_lib_name();
    let bin = {
        let Ok(mut e) = zip.by_name(name) else {
            return Err(AppError::NotFound(format!(
                "'{id}' no lleva {name} dentro del jar (mod nativo incompleto)"
            )));
        };
        let mut buf = vec![];
        e.read_to_end(&mut buf)
            .map_err(|e| AppError::Invalid(format!("'{id}': binario ilegible ({e})")))?;
        buf
    };
    let dir = dest_root.join(id);
    std::fs::create_dir_all(&dir)?;
    let so = dir.join(name);
    std::fs::write(&so, &bin)?;
    cargo_load(&so, id, dir)
}

/// Abre el `.so` y verifica el ABI antes de devolver nada utilizable.
pub fn cargo_load(so: &Path, id: &str, dir: PathBuf) -> AppResult<LoadedMod> {
    // SAFETY: abrimos un binario que el usuario puso en mods/. Por eso se
    // verifica el ABI antes de llamar cualquier funcion, y por eso el lock de
    // SHA-256 existe: un binario que no estaba declarado no debio llegar aca.
    let lib = unsafe { libloading::Library::new(so) }
        .map_err(|e| AppError::Invalid(format!("no se pudo cargar '{id}' ({e})")))?;
    let entry: libloading::Symbol<native::ModEntryFn> = unsafe { lib.get(native::MOD_ENTRY_SYMBOL.as_bytes()) }
        .map_err(|e| {
            AppError::Invalid(format!(
                "'{id}' no exporta {} ({e})",
                native::MOD_ENTRY_SYMBOL
            ))
        })?;
    let vtable = entry();
    if vtable.is_null() {
        return Err(AppError::Invalid(format!("'{id}' devolvio una tabla nula")));
    }
    let abi = unsafe { (*vtable).abi_version };
    if !native::abi_ok(abi) {
        let (maj, min) = native::abi_parts(abi);
        return Err(AppError::Invalid(format!(
            "'{id}' fue compilado contra el ABI C {maj}.{min} y este cargador habla {}.{}",
            native::ABI_C_MAJOR,
            0
        )));
    }
    Ok(LoadedMod { id: id.to_string(), abi, dir, _lib: lib, vtable })
}

/// Los mods nativos cargados, vivos mientras viva el cargador (descargar un
/// `Library` es perder el codigo).
#[derive(Default)]
pub struct NativeRegistry {
    mods: Mutex<BTreeMap<String, std::sync::Arc<LoadedMod>>>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, m: LoadedMod) {
        self.mods.lock().unwrap().insert(m.id.clone(), std::sync::Arc::new(m));
    }

    pub fn get(&self, id: &str) -> Option<std::sync::Arc<LoadedMod>> {
        self.mods.lock().unwrap().get(id).cloned()
    }

    pub fn ids(&self) -> Vec<String> {
        self.mods.lock().unwrap().keys().cloned().collect()
    }
}

/// Convierte un `&str` a lo que espera el ABI C (buffer con `\0` al final).
pub fn to_c_string(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Puntero a un buffer C, valido mientras viva el buffer.
pub fn as_c_char(v: &mut Vec<u8>) -> *const c_char {
    v.push(0);
    v.as_ptr() as *const c_char
}


#[cfg(test)]
mod tests {
    use super::*;
    use drakestar_core::native::{self, ABI_C_MAJOR};
    use std::io::Write;
    use std::sync::Mutex;

    static LOGGED: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());

    extern "C" fn log(_userdata: *mut std::os::raw::c_void, level: u32, msg: *const std::os::raw::c_char) {
        if msg.is_null() {
            return;
        }
        let s = unsafe { std::ffi::CStr::from_ptr(msg) }.to_string_lossy().to_string();
        LOGGED.lock().unwrap().push((level, s));
    }
    extern "C" fn find(_userdata: *mut std::os::raw::c_void, _name: *const std::os::raw::c_char) -> *const std::os::raw::c_void {
        std::ptr::null()
    }

    /// Tabla que publica un motor de prueba.
    fn test_table() -> HookTable {
        HookTable {
            version: native::abi_pack(ABI_C_MAJOR, 0),
            userdata: std::ptr::null_mut(),
            find: Some(find),
            log: Some(log),
        }
    }

    fn example_so() -> Option<std::path::PathBuf> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/libdrakestar_native_example.so");
        p.exists().then_some(p)
    }

    fn jar_with(so: &[u8], id: &str) -> std::path::PathBuf {
        let dir = crate::testutil::tmpdir("native-jar");
        let manifest = format!(
            r#"{{"schemaVersion":1,"id":"{id}","version":"0.1.0","environment":"*","entrypoints":{{"main":["x"]}},
                 "custom":{{"drakes":{{"abi":"1","runtime":"native"}}}}}}"#
        );
        let jar = dir.join(format!("{id}.jar"));
        {
            let f = std::fs::File::create(&jar).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let opts: zip::write::SimpleFileOptions = Default::default();
            z.start_file("fabric.mod.json", opts).unwrap();
            z.write_all(manifest.as_bytes()).unwrap();
            z.start_file(native_lib_name(), opts).unwrap();
            z.write_all(so).unwrap();
            z.finish().unwrap();
        }
        jar
    }

    /// El mod nativo real: se compila aparte (`cargo build -p
    /// drakestar-native-example`), viaja como jar y se carga contra el ABI.
    #[test]
    fn loads_a_real_native_mod() {
        let Some(so) = example_so() else {
            eprintln!("omitido: falta compilar el ejemplo (cargo build -p drakestar-native-example)");
            return;
        };
        let so_bytes = std::fs::read(&so).unwrap();
        let jar = jar_with(&so_bytes, "ejemplo");
        let dest = crate::testutil::tmpdir("native-out");
        LOGGED.lock().unwrap().clear();

        let m = load_from_jar(&jar, "ejemplo", &dest).unwrap();
        assert_eq!(m.abi_label(), "2.0");
        // el binario quedo extraido en su propio directorio, no suelto en mods/
        assert!(m.dir.join(native_lib_name()).is_file());

        // init con la tabla del motor de prueba
        let table = test_table();
        assert_eq!(m.init(&table).unwrap(), 0);
        assert!(LOGGED.lock().unwrap().iter().any(|(_, l)| l.contains("init ok")));

        // tick: ejecuta de verdad (lo verifica el contador del mod)
        for _ in 0..3 {
            m.tick().unwrap();
        }
        let lib = unsafe { libloading::Library::new(m.dir.join(native_lib_name())) }.unwrap();
        let ticks: libloading::Symbol<extern "C" fn() -> u32> =
            unsafe { lib.get(b"drakestar_example_ticks") }.unwrap();
        assert_eq!(ticks(), 3, "el mod corrio 3 ticks");

        // shutdown y el contador vuelve a 0
        m.shutdown().unwrap();
        assert_eq!(ticks(), 0);
    }

    /// Un mod compilado contra un ABI futuro no se carga.
    #[test]
    fn rejects_a_future_abi() {
        let Some(so) = example_so() else { return };
        // se parchea la version del .so? No: se prueba la regla directo
        // (un mod con otro major ni siquiera exportaria el mismo simbolo)
        assert!(native::abi_ok(native::abi_pack(2, 0)));
        // v1 quedo atras: find/log no recibian userdata
        assert!(!native::abi_ok(native::abi_pack(1, 0)));
        assert!(!native::abi_ok(native::abi_pack(9, 0)));
        let _ = so;
    }

    /// Un jar sin el binario dentro no es un mod nativo valido.
    #[test]
    fn missing_binary_is_reported() {
        let dir = crate::testutil::tmpdir("native-missing");
        let jar = dir.join("nada.jar");
        {
            let f = std::fs::File::create(&jar).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let opts: zip::write::SimpleFileOptions = Default::default();
            z.start_file("fabric.mod.json", opts).unwrap();
            z.write_all(br#"{"schemaVersion":1,"id":"nada","version":"1","environment":"*","entrypoints":{"main":["x"]}}"#).unwrap();
            z.finish().unwrap();
        }
        let e = load_from_jar(&jar, "nada", &dir.join("out")).unwrap_err();
        assert!(e.to_string().contains("no lleva"), "{e}");
    }

    /// Un .so que no exporta el simbolo de entrada: error claro.
    #[test]
    fn a_binary_without_the_entry_symbol_is_rejected() {
        let dir = crate::testutil::tmpdir("native-nosym");
        let so = dir.join("lib.so");
        std::fs::write(&so, b"no es un binario").unwrap();
        let e = cargo_load(&so, "roto", dir.join("out")).unwrap_err();
        assert!(e.to_string().contains("no exporta") || e.to_string().contains("no se pudo cargar"), "{e}");
    }
}
