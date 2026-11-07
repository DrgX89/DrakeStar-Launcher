//! El motor: dueño del ciclo de vida y de la tabla de hooks.
//!
//! Esto es lo que del otro lado esperaba el adaptador nativo. No es el cliente
//! de Minecraft: es el **motor de hooks** sobre el que un cliente se construye.
//! Publica hooks, mantiene el registro de mods nativos, y en cada tick llama a
//! los mods que se engancharon a cada hook.
//!
//! El estado vive en un `Arc<EngineInner>`; la tabla C apunta a él con
//! `userdata`, asi `find`/`log` pueden consultarlo. Al destruir el motor se
//! recupera el `Arc` (si no, se filtra memoria).

use crate::native::{HookTable, NativeRegistry};
use drakestar_core::native::{abi_pack, ABI_C_MAJOR, LOG_DEBUG, LOG_ERROR, LOG_INFO, LOG_WARN};
use std::collections::BTreeMap;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Estado compartido con las funciones C de la tabla.
pub struct EngineInner {
    /// hooks que el motor publica hoy.
    pub published: Mutex<Vec<String>>,
    /// hook -> mods enganchados (en orden de carga).
    pub hooked: Mutex<BTreeMap<String, Vec<String>>>,
    /// (nivel, mensaje) que los mods mandaron por `log`.
    pub logs: Mutex<Vec<(u32, String)>>,
    /// ticks corridos.
    pub ticks: AtomicU64,
}

/// El motor. Vive lo que los mods cargados.
pub struct Engine {
    inner: Arc<EngineInner>,
    /// LA tabla que se pasa a los mods. `userdata` apunta a `inner`.
    table: HookTable,
    registry: Arc<NativeRegistry>,
}

// las funciones C sólo tocan el interior detras de Mutex/Atomic
unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

/// Recupera el estado desde el `userdata` que recibieron las funciones C.
fn state(userdata: *mut c_void) -> &'static EngineInner {
    // SAFETY: el motor sólo construye la tabla con un Arc<EngineInner> vivo
    unsafe { &*(userdata as *const EngineInner) }
}

extern "C" fn find_hook(userdata: *mut c_void, name: *const c_char) -> *const c_void {
    if name.is_null() {
        return std::ptr::null();
    }
    let Ok(s) = unsafe { std::ffi::CStr::from_ptr(name) }.to_str() else {
        return std::ptr::null();
    };
    if state(userdata).published.lock().unwrap().iter().any(|h| h == s) {
        // puntero no nulo = "existe". Un motor real devolveria el callback.
        1 as *const c_void
    } else {
        std::ptr::null()
    }
}

extern "C" fn log(userdata: *mut c_void, level: u32, msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(msg) }.to_string_lossy().to_string();
    state(userdata).logs.lock().unwrap().push((level, s));
}

impl Engine {
    pub fn new(registry: Arc<NativeRegistry>) -> Self {
        let inner = Arc::new(EngineInner {
            published: Mutex::new(vec![]),
            hooked: Mutex::new(BTreeMap::new()),
            logs: Mutex::new(vec![]),
            ticks: AtomicU64::new(0),
        });
        let table = HookTable {
            version: abi_pack(ABI_C_MAJOR, 0),
            userdata: Arc::into_raw(inner.clone()) as *mut c_void,
            find: Some(find_hook),
            log: Some(log),
        };
        Self { inner, table, registry }
    }

    /// Publica un hook: a partir de ahora `find` lo encuentra.
    pub fn publish(&self, hook: &str) -> bool {
        let mut p = self.inner.published.lock().unwrap();
        if p.iter().any(|h| h == hook) {
            return false;
        }
        p.push(hook.to_string());
        p.sort();
        true
    }

    /// Hooks publicados.
    pub fn published(&self) -> Vec<String> {
        self.inner.published.lock().unwrap().clone()
    }

    /// La tabla que se pasa a los mods (vive lo que el motor).
    pub fn table(&self) -> &HookTable {
        &self.table
    }

    /// Arranca: llama `init` a cada mod nativo cargado con la tabla del motor.
    /// Devuelve los que fallaron (con su motivo) y los que arrancaron.
    pub fn start(&self) -> (Vec<String>, Vec<String>) {
        let mut ok = vec![];
        let mut fail = vec![];
        for id in self.registry.ids() {
            let Some(m) = self.registry.get(&id) else { continue };
            match m.init(self.table()) {
                Ok(0) => ok.push(id),
                Ok(n) => fail.push(format!("'{id}' devolvio {n}")),
                Err(e) => fail.push(format!("'{id}': {e}")),
            }
        }
        (ok, fail)
    }

    /// Engancha un mod a un hook: se llamara su `tick` cuando el hook se dispare.
    /// El hook tiene que estar publicado.
    pub fn hook(&self, hook: &str, mod_id: &str) -> bool {
        if !self.inner.published.lock().unwrap().iter().any(|h| h == hook) {
            return false;
        }
        self.inner
            .hooked
            .lock()
            .unwrap()
            .entry(hook.to_string())
            .or_default()
            .push(mod_id.to_string());
        true
    }

    /// Dispara un hook: corre el `tick` de todos los mods enganchados.
    /// Devuelve cuantos corrieron.
    pub fn fire(&self, hook: &str) -> usize {
        let ids: Vec<String> = self
            .inner
            .hooked
            .lock()
            .unwrap()
            .get(hook)
            .cloned()
            .unwrap_or_default();
        let mut n = 0;
        for id in ids {
            if let Some(m) = self.registry.get(&id) {
                let _ = m.tick();
                n += 1;
            }
        }
        n
    }

    /// Un tick del motor: dispara el hook de tick y cuenta.
    pub fn tick(&self) -> usize {
        self.inner.ticks.fetch_add(1, Ordering::SeqCst);
        self.fire("client:tick")
    }

    pub fn ticks(&self) -> u64 {
        self.inner.ticks.load(Ordering::SeqCst)
    }

    /// Corre N ticks y devuelve el total de llamadas a mods.
    pub fn run(&self, n: u32) -> usize {
        let mut total = 0;
        for _ in 0..n {
            total += self.tick();
        }
        total
    }

    /// Apaga: `shutdown` a todos y recupera el `Arc` de la tabla.
    pub fn stop(&self) {
        for id in self.registry.ids() {
            if let Some(m) = self.registry.get(&id) {
                let _ = m.shutdown();
            }
        }
    }

    /// Lo que los mods dijeron por `log`.
    pub fn logs(&self) -> Vec<(u32, String)> {
        self.inner.logs.lock().unwrap().clone()
    }

    pub fn hooked(&self, hook: &str) -> Vec<String> {
        self.inner.hooked.lock().unwrap().get(hook).cloned().unwrap_or_default()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // recuperar el Arc que se paso como userdata, o se filtra
        let p = self.table.userdata as *const EngineInner;
        if !p.is_null() {
            unsafe { Arc::from_raw(p) };
        }
    }
}

/// Helpers para motores: los niveles de log como texto.
pub fn log_level_label(l: u32) -> &'static str {
    match l {
        LOG_ERROR => "ERR",
        LOG_WARN => "WRN",
        LOG_INFO => "INF",
        LOG_DEBUG => "DBG",
        _ => "???",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drakestar_core::native::abi_parts;

    fn registry() -> Arc<NativeRegistry> {
        Arc::new(NativeRegistry::new())
    }

    #[test]
    fn publishes_hooks_and_finds_them() {
        let e = Engine::new(registry());
        assert!(!e.published().contains(&"client:tick".to_string()));
        assert!(e.publish("client:tick"));
        // no se publica dos veces
        assert!(!e.publish("client:tick"));
        assert!(e.published().contains(&"client:tick".to_string()));
        // desde el lado C: find responde por userdata
        let t = e.table();
        let hook = crate::native::to_c_string("client:tick");
        let p = (t.find.unwrap())(t.userdata, hook.as_ptr() as *const c_char);
        assert!(!p.is_null());
        let other = crate::native::to_c_string("server:tick");
        let p2 = (t.find.unwrap())(t.userdata, other.as_ptr() as *const c_char);
        assert!(p2.is_null(), "no publicado -> no existe");
    }

    #[test]
    fn log_reaches_the_engine_through_the_table() {
        let e = Engine::new(registry());
        let t = e.table();
        let msg = crate::native::to_c_string("hola desde el mod");
        (t.log.unwrap())(t.userdata, LOG_INFO, msg.as_ptr() as *const c_char);
        assert_eq!(e.logs(), vec![(LOG_INFO, "hola desde el mod".to_string())]);
    }

    #[test]
    fn hooks_are_dispatched_only_to_hooked_mods() {
        // sin mods cargados, disparar no corre nada pero cuenta el tick
        let e = Engine::new(registry());
        // sin publicar, no se puede enganchar
        assert!(!e.hook("client:tick", "alguien"));
        assert!(e.publish("client:tick"));
        assert!(e.hook("client:tick", "alguien"));
        assert_eq!(e.fire("client:tick"), 0, "no hay mod cargado");
        assert_eq!(e.run(3), 0);
        assert_eq!(e.ticks(), 3);
        // un hook no publicado no acepta enganches
        assert!(!e.hook("client:render", "alguien"));
    }

    #[test]
    fn abi_of_the_table_is_the_one_we_speak() {
        let e = Engine::new(registry());
        let (maj, min) = abi_parts(e.table().version);
        assert_eq!((maj, min), (ABI_C_MAJOR, 0));
    }

    /// Motor + mod nativo real: init, hooks y ticks de punta a punta.
    #[test]
    fn drives_a_real_native_mod() {
        let so = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/libdrakestar_native_example.so");
        if !so.exists() {
            eprintln!("omitido: falta compilar el ejemplo");
            return;
        }
        // empaquetar como jar (como viajan los mods)
        let dir = crate::testutil::tmpdir("engine-mod");
        let jar = dir.join("ejemplo.jar");
        {
            use std::io::Write;
            let f = std::fs::File::create(&jar).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let opts: zip::write::SimpleFileOptions = Default::default();
            z.start_file("fabric.mod.json", opts).unwrap();
            z.write_all(br#"{"schemaVersion":1,"id":"ejemplo","version":"0.1.0","environment":"*","entrypoints":{"main":["x"]},"custom":{"drakes":{"abi":"2"}}}"#).unwrap();
            z.start_file("drakestar_mod.so", opts).unwrap();
            z.write_all(&std::fs::read(&so).unwrap()).unwrap();
            z.finish().unwrap();
        }
        let reg = registry();
        reg.insert(crate::native::load_from_jar(&jar, "ejemplo", &dir.join("out")).unwrap());
        let e = Engine::new(reg.clone());

        // el motor publica sus hooks
        assert!(e.publish("mod:init"));
        assert!(e.publish("client:tick"));

        // init: el mod recibe la tabla y responde
        let (ok, fail) = e.start();
        assert_eq!(ok, vec!["ejemplo".to_string()]);
        assert!(fail.is_empty());

        // el mod engancha su hook favorito y el motor lo dispara
        assert!(e.hook("client:tick", "ejemplo"));
        let corridos = e.run(4);
        assert_eq!(corridos, 4, "4 ticks x 1 mod");
        assert_eq!(e.ticks(), 4);

        // lo que el mod dijo por la tabla (init + que encontro su hook)
        let logs = e.logs();
        assert!(logs.iter().any(|(_, m)| m.contains("init ok")), "{logs:?}");
        assert!(logs.iter().any(|(_, m)| m.contains("client:tick")), "{logs:?}");

        e.stop();
    }
}
