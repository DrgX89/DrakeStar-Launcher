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
    /// Resultado de análisis de seguridad de Blackwall Guard
    pub security: drakestar_core::guard::ScanResult,
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

pub fn allowlist_path(dir: &Path) -> PathBuf {
    dir.join("guard.allowlist.json")
}

pub fn read_allowlist(dir: &Path) -> drakestar_core::guard::GuardAllowlist {
    let p = allowlist_path(dir);
    if !p.is_file() {
        return drakestar_core::guard::GuardAllowlist::default();
    }
    std::fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn write_allowlist(dir: &Path, list: &drakestar_core::guard::GuardAllowlist) -> AppResult<()> {
    crate::layout::write_atomic(&allowlist_path(dir), serde_json::to_string_pretty(list)?.as_bytes())?;
    Ok(())
}

pub fn set_override(dir: &Path, item: &str, allow: bool) -> AppResult<()> {
    let mut list = read_allowlist(dir);
    if allow {
        if !list.allowed.contains(&item.to_string()) {
            list.allowed.push(item.to_string());
        }
    } else {
        list.allowed.retain(|x| x != item);
    }
    write_allowlist(dir, &list)
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
    let allowlist = read_allowlist(dir);
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
        collect(&j, false, disabled, &allowlist, &mut out, &mut warns)?;
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
            if let Some(c) = entry.get("class").and_then(|c| c.as_str()) {
                out.push(c.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn scan_zip_security(
    file_name: &str,
    zip: &mut zip::ZipArchive<std::fs::File>,
    allowlist: &drakestar_core::guard::GuardAllowlist,
    sha256: &str,
) -> drakestar_core::guard::ScanResult {
    let mut reasons = Vec::new();
    let mut suspicious_strings = std::collections::BTreeSet::new();
    let mut max_risk = drakestar_core::guard::RiskLevel::Safe;

    for i in 0..zip.len() {
        if let Ok(mut entry) = zip.by_index(i) {
            let name = entry.name().to_string();
            if name.ends_with(".exe") || name.ends_with(".bat") || name.ends_with(".sh") {
                reasons.push(format!("Contiene binario ejecutable externo: {name}"));
                suspicious_strings.insert(name.clone());
                max_risk = drakestar_core::guard::RiskLevel::Dangerous;
            }
            if name.ends_with(".class") {
                let mut buf = vec![0u8; 4096.min(entry.size() as usize)];
                let _ = entry.read(&mut buf);
                let text = String::from_utf8_lossy(&buf);
                if text.contains("java/lang/ProcessBuilder") {
                    reasons.push("Invocación no autorizada de procesos (ProcessBuilder)".into());
                    suspicious_strings.insert("ProcessBuilder".into());
                    max_risk = drakestar_core::guard::RiskLevel::Dangerous;
                }
                if text.contains("Runtime;->exec") || text.contains("getRuntime().exec") {
                    reasons.push("Ejecución de comandos de terminal (Runtime.exec)".into());
                    suspicious_strings.insert("Runtime.exec".into());
                    max_risk = drakestar_core::guard::RiskLevel::Dangerous;
                }
                if text.contains("discord.com/api/webhooks") {
                    reasons.push("Exfiltración potencial a Discord Webhook".into());
                    suspicious_strings.insert("discord-webhook".into());
                    max_risk = drakestar_core::guard::RiskLevel::Dangerous;
                }
                if text.contains(".ssh/id_") || text.contains("id_rsa") || text.contains("id_ed25519") {
                    reasons.push("Intento de acceso o robo de llaves SSH privadas".into());
                    suspicious_strings.insert("ssh-key-theft".into());
                    max_risk = drakestar_core::guard::RiskLevel::Dangerous;
                }
            }
        }
    }
    reasons.sort();
    reasons.dedup();

    let allowed = allowlist.is_allowed(file_name, sha256);
    let mut quarantine = max_risk == drakestar_core::guard::RiskLevel::Dangerous;
    if allowed {
        quarantine = false;
    }

    drakestar_core::guard::ScanResult {
        file: file_name.to_string(),
        risk: max_risk,
        reasons,
        suspicious_strings: suspicious_strings.into_iter().collect(),
        quarantine,
        user_override: allowed,
    }
}

fn collect(
    path: &Path,
    nested: bool,
    disabled: bool,
    allowlist: &drakestar_core::guard::GuardAllowlist,
    out: &mut Vec<FoundMod>,
    warns: &mut Vec<String>,
) -> AppResult<()> {
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
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let security = scan_zip_security(&file_name, &mut zip, allowlist, &sha256);
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
        file: file_name,
        path: path.to_path_buf(),
        sha256,
        size: meta.len(),
        manifest,
        nested,
        disabled,
        mixin_targets,
        security,
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
            let _ = collect(&tmp, true, false, allowlist, out, warns);
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
    /// Alertas y detecciones del motor de seguridad Blackwall Guard.
    pub security_alerts: Vec<drakestar_core::guard::ScanResult>,
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
    let mut security_alerts: Vec<drakestar_core::guard::ScanResult> = vec![];

    for m in &usable {
        if m.security.risk != drakestar_core::guard::RiskLevel::Safe {
            security_alerts.push(m.security.clone());
        }
        if m.security.quarantine && !m.security.user_override {
            not_loaded.push((
                m.id.clone(),
                format!("Bloqueado por DrakeStar Blackwall Guard: {}", m.security.reasons.join(", ")),
            ));
            continue;
        }

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
        security_alerts,
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
            security_alerts: vec![],
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

    #[test]
    fn blackwall_guard_blocks_dangerous_and_override_allows() {
        let dir = crate::testutil::tmpdir("mods-guard");
        std::fs::create_dir_all(&dir).unwrap();
        let evil_manifest = r#"{"schemaVersion":1,"id":"evilmod","version":"1.0.0","environment":"*","entrypoints":{"main":["evil.Main"]}}"#;
        let mut buf = vec![];
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts: zip::write::SimpleFileOptions = Default::default();
            use std::io::Write;
            z.start_file("fabric.mod.json", opts).unwrap();
            z.write_all(evil_manifest.as_bytes()).unwrap();
            z.start_file("evil/Main.class", opts).unwrap();
            z.write_all(b"CAFEBABE...java/lang/ProcessBuilder...cmd.exe").unwrap();
            z.finish().unwrap();
        }
        std::fs::write(dir.join("evilmod.jar"), &buf).unwrap();

        // 1. Descubrimiento: detectado como peligroso y en cuarentena
        let (found, _) = discover(&dir).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].security.risk, drakestar_core::guard::RiskLevel::Dangerous);
        assert!(found[0].security.quarantine);
        assert!(!found[0].security.user_override);

        // 2. Carga: bloqueado en not_loaded
        let l = Layout::new(crate::testutil::tmpdir("mods-guard-l"));
        let mut p = sample("cli", drakestar_core::Mode::DrakesJava);
        p.loader = drakestar_core::profile::Loader::Fabric;
        let inst = crate::fabric::instance_dir(&l, &p.id).join("mods");
        std::fs::create_dir_all(&inst).unwrap();
        std::fs::copy(dir.join("evilmod.jar"), inst.join("evilmod.jar")).unwrap();

        let report = load(&l, &p, drakestar_core::mods::Environment::Client).unwrap();
        assert!(report.order.is_empty());
        assert_eq!(report.not_loaded.len(), 1);
        assert!(report.not_loaded[0].1.contains("Blackwall Guard"));
        assert_eq!(report.security_alerts.len(), 1);

        // 3. El usuario activa la casilla de permitir ("Sé lo que estoy haciendo")
        set_override(&inst, "evilmod.jar", true).unwrap();

        // 4. Nueva carga: ahora se respeta la decisión del usuario
        let report_allowed = load(&l, &p, drakestar_core::mods::Environment::Client).unwrap();
        assert_eq!(report_allowed.security_alerts.len(), 1);
        assert!(report_allowed.security_alerts[0].user_override);
        assert_eq!(report_allowed.order, vec!["evilmod".to_string()]);
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
