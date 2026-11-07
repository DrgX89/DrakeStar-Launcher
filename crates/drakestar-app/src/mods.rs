//! Cargador de mods: descubre, verifica, valida y ordena.
//!
//! No ejecuta nada: decide **que se carga**. La ejecucion (JVM o cdylib) es del
//! adaptador, y mientras ese exista, Fabric Loader sigue siendo quien corre los
//! mods Java: lo que cambia es que ahora quien decide y verifica es DrakeStar.
//!
//! Verificacion: cada mod se hashea (SHA-256) y se compara con `mods.lock.json`.
//! La primera vez se crea el lock; despues, un mod que cambio deja de cargar.
//! Eso es lo que hoy no hace ningun launcher: un mod corrupto o reemplazado no
//! entra al classpath.

use crate::{AppError, AppResult, Layout};
use drakestar_core::{inter, mods, profile::Profile};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Un mod descubierto en disco.
#[derive(Debug, Clone)]
pub struct FoundMod {
    /// id del manifiesto (`fabric.mod.json`).
    pub id: String,
    /// nombre del archivo (para mostrarlo).
    pub file: String,
    pub path: PathBuf,
    pub sha256: String,
    pub size: u64,
    pub manifest: mods::ModManifest,
    /// true si vino de un jar-in-jar de otro mod.
    pub nested: bool,
    /// true si esta como `<archivo>.jar.disabled` (no cargo la ultima vez).
    pub disabled: bool,
    /// Clases objetivo que declara en sus `mixins.json` (para detectar
    /// colisiones: dos mods tocando la misma clase es lo que hace crasear
    /// Fabric en caliente).
    pub mixin_targets: Vec<String>,
}

/// Lo que se guarda en `mods.lock.json`: el hash esperado de cada archivo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Lock {
    pub mods: Vec<LockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    pub file: String,
    pub sha256: String,
    pub size: u64,
}

pub fn lock_path(dir: &Path) -> PathBuf {
    dir.join("mods.lock.json")
}

/// Descubre los mods de `dir`: los jars de primer nivel y, recursivamente, sus
/// jar-in-jar (como hace el loader de Fabric). Un jar sin `fabric.mod.json` se
/// ignora (no es un mod de Fabric).
pub fn discover(dir: &Path) -> AppResult<(Vec<FoundMod>, Vec<String>)> {
    let mut out = vec![];
    let mut warns = vec![];
    if !dir.is_dir() {
        return Ok((out, warns));
    }
    let mut jars: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            n.ends_with(".jar") || n.ends_with(".jar.disabled")
        })
        .collect();
    jars.sort();
    for j in jars {
        let disabled = j.to_string_lossy().ends_with(".jar.disabled");
        collect(&j, false, disabled, &mut out, &mut warns)?;
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((out, warns))
}

/// Lee las clases objetivo de los `mixins.json` que declara un mod. Formato
/// real de Fabric: `{"mixins": [{"class": "paquete.Clase", ...}]}`.
fn read_mixin_targets(zip: &mut zip::ZipArchive<std::fs::File>) -> Vec<String> {
    let names: Vec<String> = zip
        .file_names()
        .filter(|n| n.ends_with(".mixins.json"))
        .map(str::to_string)
        .collect();
    let mut out = vec![];
    for n in names {
        let Ok(mut e) = zip.by_name(&n) else { continue };
        let mut text = String::new();
        if e.read_to_string(&mut text).is_err() {
            continue;
        }
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let Some(list) = json.get("mixins").and_then(|m| m.as_array()) else { continue };
        for entry in list {
            // el objetivo puede ser el nombre de la mezcla (que ya incluye el
            // paquete) o el campo "class"
            if let Some(c) = entry.get("class").and_then(|c| c.as_str()) {
                out.push(c.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn collect(path: &Path, nested: bool, disabled: bool, out: &mut Vec<FoundMod>, warns: &mut Vec<String>) -> AppResult<()> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Invalid(format!("{}: no es un jar leible ({e})", path.file_name().unwrap_or_default().to_string_lossy())))?;
    let text = {
        let Ok(mut e) = zip.by_name("fabric.mod.json") else {
            return Ok(()); // jar sin manifiesto: no es un mod de Fabric
        };
        let mut t = String::new();
        e.read_to_string(&mut t)
            .map_err(|e| AppError::Invalid(format!("{}: fabric.mod.json ilegible ({e})", path.display())))?;
        t
    };
    let manifest = match mods::ModManifest::from_json(&text) {
        Ok(m) => m,
        // un jar con manifiesto roto no es un mod de Fabric valido: se avisa y
        // se sigue, para que un archivo no tumbe toda la carpeta
        Err(e) => {
            warns.push(format!("{}: {e}", path.file_name().unwrap_or_default().to_string_lossy()));
            return Ok(());
        }
    };
    let meta = std::fs::metadata(path)?;
    let sha256 = drakestar_net::sha256_file(path).map_err(AppError::from)?;
    // targets de Mixin: los .mixins.json que declara el manifiesto
    let mixin_targets = read_mixin_targets(&mut zip);
    // sus jar-in-jar
    let nested_files: Vec<String> = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|m| m.get("jars").and_then(|j| j.as_array()).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|j| j.get("file").and_then(|f| f.as_str()).map(str::to_string))
        .collect();
    out.push(FoundMod {
        id: manifest.id.clone(),
        file: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        path: path.to_path_buf(),
        sha256,
        size: meta.len(),
        manifest,
        nested,
        disabled,
        mixin_targets,
    });
    for n in nested_files {
        let mut buf = vec![];
        if let Ok(mut e) = zip.by_name(&n) {
            let _ = e.read_to_end(&mut buf);
        }
        // el jar anidado se lee desde memoria y se escribe a un temporal,
        // porque el cargador trabaja con rutas
        let tmp = std::env::temp_dir().join(format!("drakestar-jij-{}-{}", std::process::id(), out.len()));
        if std::fs::write(&tmp, &buf).is_ok() {
            let _ = collect(&tmp, true, false, out, warns);
            let _ = std::fs::remove_file(&tmp);
        }
    }
    Ok(())
}

/// Verifica cada mod contra el lock. Sin lock, lo crea (primera carga).
/// Devuelve los ids que no verifican (cambiaron o no estan).
pub fn verify(dir: &Path, found: &[FoundMod]) -> AppResult<(Vec<String>, Vec<String>)> {
    let lock = read_lock(dir)?;
    let mut ok = vec![];
    let mut bad = vec![];
    let mut to_write = Lock { mods: vec![] };
    for m in found {
        let entry = LockEntry { file: m.file.clone(), sha256: m.sha256.clone(), size: m.size };
        to_write.mods.push(entry.clone());
        match lock.mods.iter().find(|e| e.file == m.file) {
            // ya estaba y coincide: verificado
            Some(e) if e == &entry => ok.push(m.id.clone()),
            // ya estaba y NO coincide: cambio el archivo, no carga
            Some(e) => bad.push(format!(
                "{}: el archivo cambio (lock {}/{} B, disco {}/{} B)",
                m.file,
                &e.sha256[..8.min(e.sha256.len())],
                e.size,
                &m.sha256[..8.min(m.sha256.len())],
                m.size
            )),
            // primera vez: se da de alta (el lock se escribe al final)
            None => ok.push(m.id.clone()),
        }
    }
    // primera vez (o con cambios): se reescribe para dejar constancia
    if lock.mods != to_write.mods {
        write_lock(dir, &to_write)?;
    }
    Ok((ok, bad))
}

pub fn read_lock(dir: &Path) -> AppResult<Lock> {
    let p = lock_path(dir);
    if !p.is_file() {
        return Ok(Lock::default());
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(p)?).unwrap_or_default())
}

pub fn write_lock(dir: &Path, lock: &Lock) -> AppResult<()> {
    crate::layout::write_atomic(&lock_path(dir), serde_json::to_string_pretty(lock)?.as_bytes())?;
    Ok(())
}

/// Entorno real: la version del loader sale del cache de Fabric.
pub fn env_for(p: &Profile, loader_version: &str) -> mods::Env {
    mods::Env {
        minecraft: p.minecraft.clone(),
        loader: loader_version.to_string(),
        java: p.java.major.map(|m| m.to_string()).unwrap_or_else(|| "25".into()),
    }
}

/// Resultado de cargar: que entra, en que orden, y por que lo que no entra.
#[derive(Debug, Clone, Serialize)]
pub struct LoadReport {
    pub discovered: usize,
    pub verified: usize,
    /// ids en orden de carga (dependencias primero).
    pub order: Vec<String>,
    /// id -> motivo por el que no carga.
    pub not_loaded: Vec<(String, String)>,
    /// id -> runtime y ABI, para mostrarlo.
    pub abi: Vec<(String, String, String)>,
    /// rutas de los jars a agregar al classpath, en orden.
    pub classpath: Vec<PathBuf>,
    pub warnings: Vec<(String, String)>,
    pub bytes: u64,
    /// jars que no eran mods de Fabric validos (con el motivo).
    pub skipped: Vec<String>,
    /// IDs duplicados y colisiones de clases objetivo de Mixin (lo que hace
    /// crasear Fabric en caliente: mejor avisar antes).
    pub collisions: Vec<String>,
}

/// Carga los mods de un perfil: descubre, verifica, valida con el intermediario
/// y resuelve el grafo. `side` es el lado que se va a ejecutar.
pub fn load(l: &Layout, p: &Profile, side: mods::Environment) -> AppResult<LoadReport> {
    let instance = crate::fabric::instance_dir(l, &p.id);
    let (found, skipped) = discover(&instance.join("mods"))?;
    let (verified, hash_errors) = verify(&instance.join("mods"), &found)?;
    let loader_version = crate::fabric::cached(l, &p.id)
        .map(|(_, c)| c.entry.loader.version.clone())
        .unwrap_or_else(|| "0".into());
    let env = env_for(p, &loader_version);

    // solo se considera lo que verifica
    let bad_files: Vec<String> = hash_errors
        .iter()
        .map(|e| e.split(':').next().unwrap_or("").to_string())
        .collect();
    let usable: Vec<&FoundMod> = found.iter().filter(|m| !bad_files.contains(&m.file)).collect();

    let catalog = inter::Catalog::abi_1();
    // validacion del intermediario, mod a mod
    let mut not_loaded: Vec<(String, String)> = vec![];
    let mut abi: Vec<(String, String, String)> = vec![];
    let mut manifests: Vec<mods::ModManifest> = vec![];
    for m in &usable {
        match inter::plan(&m.manifest, &catalog, side) {
            Ok(pl) => abi.push((pl.id.clone(), pl.runtime.label().to_string(), pl.abi.to_string())),
            Err(e) => not_loaded.push((m.id.clone(), e.to_string())),
        }
        manifests.push(m.manifest.clone());
    }
    // resolucion del grafo (dependencias, entorno, conflictos)
    let (order, warnings) = match mods::resolve(&manifests, &env, side) {
        Ok((order, warns)) => (order, warns),
        Err(reasons) => {
            for r in &reasons {
                not_loaded.push((r.mod_id.clone(), r.text.clone()));
            }
            // los que si cargan, para no quedarse sin nada
            (
                manifests
                    .iter()
                    .filter(|m| !reasons.iter().any(|r| r.mod_id == m.id))
                    .map(|m| m.id.clone())
                    .collect(),
                vec![],
            )
        }
    };
    let order: Vec<String> = order.into_iter().filter(|id| !not_loaded.iter().any(|(i, _)| i == id)).collect();
    // ids duplicados y colisiones de targets de Mixin (del estilo del analizador
    // de modsync, pero con los datos reales de cada jar)
    let mut collisions: Vec<String> = vec![];
    let mut seen_ids: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for m in &usable {
        *seen_ids.entry(m.id.as_str()).or_default() += 1;
    }
    for (id, n) in seen_ids {
        if n > 1 {
            collisions.push(format!("id '{id}' declarado por {n} mods"));
        }
    }
    let mut targets: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
    for m in &usable {
        for t in &m.mixin_targets {
            targets.entry(t.as_str()).or_default().push(m.id.as_str());
        }
    }
    for (t, mods_de) in targets {
        if mods_de.len() > 1 {
            collisions.push(format!("'{t}' modificado simultaneamente por: {}", mods_de.join(", ")));
        }
    }
    // classpath en el orden resuelto
    let by_id: std::collections::BTreeMap<String, &FoundMod> =
        usable.iter().map(|m| (m.id.clone(), *m)).collect();
    let classpath: Vec<PathBuf> = order.iter().filter_map(|id| by_id.get(id).map(|m| m.path.clone())).collect();
    let bytes = classpath.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum();
    Ok(LoadReport {
        collisions,
        skipped,
        discovered: found.len(),
        verified: verified.len(),
        order,
        not_loaded,
        abi,
        classpath,
        warnings: warnings.into_iter().map(|w| (w.mod_id, w.text)).collect(),
        bytes,
    })
}

/// Aplica las decisiones del cargador: lo que no cargo se deshabilita
/// (`<archivo>.disabled`, reversible) para que Fabric Loader no lo vea. Asi
/// **quien decide que se carga es DrakeStar**, no el classpath.
/// Devuelve cuantos quedaron deshabilitados.
pub fn apply_decisions(_dir: &Path, found: &[FoundMod], report: &LoadReport) -> AppResult<usize> {
    let mut n = 0;
    for m in found {
        let ok = report.order.contains(&m.id);
        match (ok, m.disabled) {
            // carga y estaba deshabilitado -> habilitar
            (true, true) => {
                let to = m.path.to_string_lossy().trim_end_matches(".disabled").to_string();
                std::fs::rename(&m.path, std::path::PathBuf::from(to))?;
                n += 1;
            }
            // no carga y estaba habilitado -> deshabilitar
            (false, false) => {
                let mut to = m.path.clone().into_os_string();
                to.push(".disabled");
                std::fs::rename(&m.path, std::path::PathBuf::from(to))?;
                n += 1;
            }
            // ya estaba en el estado correcto
            _ => {}
        }
    }
    Ok(n)
}

/// El classpath de mods ya resuelto (para agregar al del cliente).
pub fn mods_classpath(l: &Layout, p: &Profile, side: mods::Environment) -> AppResult<Vec<PathBuf>> {
    load(l, p, side).map(|r| r.classpath)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::sample;
    use std::io::Write;
    use drakestar_core::Channel;

    fn profile() -> Profile {
        let mut p = sample("run", drakestar_core::Mode::DrakesJava);
        p.loader = drakestar_core::profile::Loader::Fabric;
        p.channel = Channel::Stable;
        p
    }

    /// Crea un jar real: fabric.mod.json y, si se pide, jars anidados (tambien
    /// jars de verdad, que es como viajan en Fabric).
    fn jar_bytes(manifest: &str, nested: &[(String, String)]) -> Vec<u8> {
        let mut buf = vec![];
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts: zip::write::SimpleFileOptions = Default::default();
            z.start_file("fabric.mod.json", opts).unwrap();
            z.write_all(manifest.as_bytes()).unwrap();
            for (name, man) in nested {
                z.start_file(name.clone(), opts).unwrap();
                // el anidado es un jar de verdad: fabric.mod.json dentro
                z.write_all(&jar_bytes(man, &[])).unwrap();
            }
            z.finish().unwrap();
        }
        buf
    }
    fn make_jar(path: &Path, manifest: &str, nested: &[(String, String)]) {
        let inner = jar_bytes(manifest, nested);
        std::fs::write(path, &inner).unwrap();
    }

    #[test]
    fn discovers_mods_and_their_jar_in_jar() {
        let dir = crate::testutil::tmpdir("mods-disc");
        std::fs::create_dir_all(&dir).unwrap();
        make_jar(&dir.join("api.jar"), r#"{"schemaVersion":1,"id":"api","version":"1.0.0","environment":"*","entrypoints":{}}"#, &[]);
        make_jar(
            &dir.join("addon.jar"),
            r#"{"schemaVersion":1,"id":"addon","version":"2.0.0","environment":"*","entrypoints":{"main":["a.Main"]},"depends":{"api":"*"},"jars":[{"file":"META-INF/jars/inner.jar"}]}"#,
            &[("META-INF/jars/inner.jar".into(), r#"{"schemaVersion":1,"id":"inner","version":"9.9.9","environment":"*","entrypoints":{}}"#.into())],
        );
        make_jar(&dir.join("atajo.txt"), "no es un jar", &[]);
        let (found, _) = discover(&dir).unwrap();
        assert_eq!(found.len(), 3, "api + addon + inner");
        assert!(found.iter().any(|m| m.id == "inner" && m.nested));
        assert!(found.iter().all(|m| !m.sha256.is_empty()));
    }

    #[test]
    fn lock_detects_swapped_or_changed_mods() {
        let dir = crate::testutil::tmpdir("mods-lock");
        std::fs::create_dir_all(&dir).unwrap();
        make_jar(&dir.join("a.jar"), r#"{"schemaVersion":1,"id":"a","version":"1.0.0","environment":"*","entrypoints":{}}"#, &[]);
        let (found, _) = discover(&dir).unwrap();
        // primera vez: sin lock, se da de alta y verifica
        let (ok, bad) = verify(&dir, &found).unwrap();
        assert_eq!(ok.len(), 1);
        assert!(bad.is_empty());
        // el lock quedo escrito
        assert!(lock_path(&dir).is_file());
        // segunda vez: todo verifica
        let (ok, bad) = verify(&dir, &found).unwrap();
        assert_eq!(ok.len(), 1);
        assert!(bad.is_empty());
        // ahora cambia el archivo: ya no verifica
        make_jar(&dir.join("a.jar"), r#"{"schemaVersion":1,"id":"a","version":"2.0.0","environment":"*","entrypoints":{}}"#, &[]);
        let (found2, _) = discover(&dir).unwrap();
        let (ok, bad) = verify(&dir, &found2).unwrap();
        assert_eq!(ok.len(), 0);
        assert_eq!(bad.len(), 1);
        assert!(bad[0].contains("cambio"), "{}", bad[0]);
    }

    #[test]
    fn a_jar_without_manifest_is_not_a_mod() {
        let dir = crate::testutil::tmpdir("mods-fake");
        std::fs::create_dir_all(&dir).unwrap();
        make_jar(&dir.join("lib.jar"), r#"{"id":"no-es-fabric"}"#, &[]);
        make_jar(&dir.join("roto.jar"), "{no json", &[]);
        let (found, warns) = discover(&dir).unwrap();
        assert!(found.is_empty(), "ninguno es un mod valido");
        assert_eq!(warns.len(), 2, "los dos se avisan: {:?}", warns);
    }

    #[test]
    fn rejected_mods_are_disabled_reversibly() {
        let dir = crate::testutil::tmpdir("mods-apply");
        std::fs::create_dir_all(&dir).unwrap();
        make_jar(&dir.join("solo.jar"), r#"{"schemaVersion":1,"id":"solo","version":"1.0.0","environment":"*","entrypoints":{"main":["a.Main"]}}"#, &[]);
        make_jar(&dir.join("roto.jar"), r#"{"schemaVersion":1,"id":"roto","version":"1.0.0","environment":"*","entrypoints":{"main":["a.Main"]},"depends":{"no-existe":"*"}}"#, &[]);
        let (found, _) = discover(&dir).unwrap();
        let mut report = LoadReport {
            discovered: 2,
            verified: 2,
            order: vec!["solo".into()],
            not_loaded: vec![("roto".into(), "falta el mod 'no-existe'".into())],
            abi: vec![],
            classpath: vec![dir.join("solo.jar")],
            warnings: vec![],
            bytes: 0,
            skipped: vec![],
            collisions: vec![],
        };
        let n = apply_decisions(&dir, &found, &report).unwrap();
        assert_eq!(n, 1);
        assert!(dir.join("roto.jar.disabled").is_file());
        assert!(!dir.join("roto.jar").exists());
        assert!(dir.join("solo.jar").exists());
        // revertir: si ahora si carga, se vuelve a habilitar (con el estado real
        // del disco, que es lo que hace mods_decide)
        report.order.push("roto".into());
        let (found2, _) = discover(&dir).unwrap();
        let n = apply_decisions(&dir, &found2, &report).unwrap();
        assert_eq!(n, 1);
        assert!(dir.join("roto.jar").exists());
        assert!(!dir.join("roto.jar.disabled").exists());
        let _ = &found;
    }


    #[test]
    fn detects_duplicate_ids_and_mixin_target_collisions() {
        // dos mods que tocan la misma clase: lo que hace crasear Fabric
        let dir = crate::testutil::tmpdir("mods-collide");
        std::fs::create_dir_all(&dir).unwrap();
        let mixins = r#"{"mixins": [{"class": "net.minecraft.world.level.ChunkGenerator"}]}"#;
        for id in ["moda", "modb"] {
            let mut buf = vec![];
            {
                let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let opts: zip::write::SimpleFileOptions = Default::default();
                use std::io::Write;
                z.start_file("fabric.mod.json", opts).unwrap();
                z.write_all(
                    format!(r#"{{"schemaVersion":1,"id":"{id}","version":"1.0.0","environment":"*","entrypoints":{{}},"custom":{{"drakes":{{"abi":"2"}}}}}}"#)
                        .as_bytes(),
                )
                .unwrap();
                z.start_file(format!("{id}.mixins.json"), opts).unwrap();
                z.write_all(mixins.as_bytes()).unwrap();
                z.finish().unwrap();
            }
            std::fs::write(dir.join(format!("{id}.jar")), &buf).unwrap();
        }
        let (found, _) = discover(&dir).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|m| m.mixin_targets == vec!["net.minecraft.world.level.ChunkGenerator".to_string()]));
        // el reporte lo ve
        let l = Layout::new(crate::testutil::tmpdir("mods-collide-l"));
        let mut p = sample("cli", drakestar_core::Mode::DrakesJava);
        p.loader = drakestar_core::profile::Loader::Fabric;
        let inst = crate::fabric::instance_dir(&l, &p.id).join("mods");
        std::fs::create_dir_all(&inst).unwrap();
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            std::fs::copy(e.path(), inst.join(e.file_name())).unwrap();
        }
        let report = load(&l, &p, drakestar_core::mods::Environment::Client).unwrap();
        assert!(
            report.collisions.iter().any(|c| c.contains("ChunkGenerator") && c.contains("moda") && c.contains("modb")),
            "colisiones: {:?}",
            report.collisions
        );
    }

    /// El cargador real: instala el cliente y carga los mods de una carpeta.
    /// `#[ignore]` porque necesita red. Uso:
    ///   cargo test -p drakestar-app mods::tests::real_load -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_load() {
        let mods = std::env::var("DS_MODS").unwrap_or_else(|_| "/home/nixcobt/Descargas/mods".into());
        if !Path::new(&mods).is_dir() {
            eprintln!("omitido: no existe {mods}");
            return;
        }
        let l = Layout::new(crate::testutil::tmpdir("mods-real"));
        let p = profile();
        // el cliente tiene que estar instalado: se reusa el cache de otra corrida
        // si existe; si no, se instala (descarga)
        let inst = crate::fabric::instance_dir(&l, &p.id);
        let cached_mods = Path::new("/tmp/opencode/drakestar-run/profiles/run/instance");
        if inst.join("client.jar").is_file() {
            // ya instalado
        } else if cached_mods.join("client.jar").is_file() {
            for e in ["client.jar", "libraries", "version.json", "fabric.json"] {
                let from = cached_mods.join(e);
                if from.is_dir() {
                    copy_dir(&from, &inst.join(e));
                } else if from.is_file() {
                    std::fs::create_dir_all(inst.clone()).unwrap();
                    std::fs::copy(&from, inst.join(e)).unwrap();
                }
            }
        }
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        for e in std::fs::read_dir(&mods).unwrap().flatten() {
            if e.path().extension().map(|x| x == "jar").unwrap_or(false) {
                std::fs::copy(e.path(), inst.join("mods").join(e.file_name())).unwrap();
            }
        }
        let report = load(&l, &p, drakestar_core::mods::Environment::Client).unwrap();
        eprintln!("descubiertos: {}", report.discovered);
        eprintln!("verificados: {}", report.verified);
        eprintln!("orden ({}):", report.order.len());
        for (i, id) in report.order.iter().enumerate() {
            eprintln!("  {:<3} {}", format!("{}.", i + 1), id);
        }
        eprintln!("no cargan: {}", report.not_loaded.len());
        for (id, why) in &report.not_loaded {
            eprintln!("  x [{id}] {why}");
        }
        for (id, runtime, abi) in &report.abi {
            eprintln!("  · {id}: {runtime} ABI {abi}");
        }
        // tiene que cargar distanthorizons y fabric-api
        assert!(report.order.contains(&"distanthorizons".to_string()));
        assert!(report.order.contains(&"fabric-api".to_string()));
        assert!(report.not_loaded.is_empty(), "algo no debia fallar: {:?}", report.not_loaded);
    }

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let t = to.join(e.file_name());
            if e.path().is_dir() {
                copy_dir(&e.path(), &t);
            } else {
                std::fs::copy(e.path(), t).unwrap();
            }
        }
    }
}
