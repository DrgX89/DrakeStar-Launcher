//! Motor alternativo: el cliente nativo de Horizon (26.3) para el modo DrakesRust.
//!
//! Horizon (github.com/horizon-rust/horizon, LGPL-3.0+) es un cliente de
//! Minecraft reescrito en Rust. No se linkea ni se copia: el launcher baja sus
//! binarios del upstream con SHA-256 fijado, prepara los recursos con
//! `horizon-prepare` (modo launcher de terceros) desde los archivos que ya
//! descargamos de Mojang, y lo ejecuta como proceso aparte.
//!
//! Límites sabidos:
//! - el singleplayer corre en 26.3; para el servidor 26.2 propio hace falta el
//!   adaptador experimental (`HORIZON_OLDER_SERVERS=1`), que ellos marcaron
//!   como no validado para online-mode.
//! - `horizon-prepare` corre los exporters de Mojang: pide un **JDK** (con
//!   `javac`), no el JRE con el que jugamos.
//! - Horizon no tiene opción de título de ventana: se renombra por fuera con
//!   wmctrl (X11). Por eso el plan fuerza el backend X11; en Wayland puro el
//!   título se queda como está.

use crate::error::{McError, McResult};
use crate::version::VersionJson;
use crate::launch::Plan;
use crate::{Dirs, ProgressSink};
use drakestar_net::download_verified;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Título que se le pone a la ventana del juego.
pub const WINDOW_TITLE: &str = "DrakeStar 26.3(RUST) by Horizon";

/// Versión de Minecraft que juega el cliente nativo (los recursos se preparan
/// de esa versión; el resto se adapta).
pub const MC_NATIVE: &str = "26.3";

/// Mayor de Java que pide `horizon-prepare` (los exporters usan `javac`).
pub const PREPARE_JAVA: u32 = 25;

/// Binario por plataforma (en Windows llevan extensión).
#[cfg(windows)]
pub const CLIENT_BIN: &str = "horizon-client.exe";
#[cfg(not(windows))]
pub const CLIENT_BIN: &str = "horizon-client";
#[cfg(windows)]
pub const PREPARE_BIN: &str = "horizon-prepare.exe";
#[cfg(not(windows))]
pub const PREPARE_BIN: &str = "horizon-prepare";

/// Distancia de render por defecto: Horizon sugiere 12; 8 va sobrado en PCs
/// modestos (con 128 se vio un OOM) y deja margen con el resto abierto.
pub const DEFAULT_RENDER_DISTANCE: u32 = 8;

/// Dónde vive cada cosa.
pub struct Hz {
    /// Raíz compartida: binarios y recursos preparados (no son por instancia).
    pub shared: PathBuf,
    /// Saves/opciones/logs del jugador en esta instancia.
    pub home: PathBuf,
}

/// Solo la parte compartida (para instalar sin elegir instancia).
pub fn shared_dir(d: &Dirs) -> PathBuf {
    d.root().join("horizon")
}

pub fn paths(d: &Dirs, id: &str) -> Hz {
    Hz {
        shared: shared_dir(d),
        home: d.instance(id).join("horizon-home"),
    }
}

impl Hz {
    /// Carpeta del paquete de Horizon tal como se extrae: además de los
    /// binarios trae `research-lock.json`, que `horizon-prepare` exige junto
    /// al programa (o vía `--root`, que es lo que le pasamos).
    pub fn pkg(&self) -> PathBuf {
        self.shared.join("pkg")
    }
    pub fn client(&self) -> PathBuf {
        self.pkg().join(CLIENT_BIN)
    }
    pub fn prepare(&self) -> PathBuf {
        self.pkg().join(PREPARE_BIN)
    }
    /// Los recursos preparados se comparten por versión de Minecraft.
    pub fn resources(&self) -> PathBuf {
        self.shared.join("resources").join(MC_NATIVE)
    }
    pub fn installed(&self) -> bool {
        self.client().is_file() && self.prepare().is_file()
    }
}

// ---------- descarga ----------

/// URL del paquete de binarios por plataforma.
pub fn package_url() -> McResult<&'static str> {
    #[cfg(not(windows))]
    {
        Ok("https://github.com/horizon-rust/horizon/releases/download/nightly/horizon-linux-x86_64.tar.gz")
    }
    #[cfg(windows)]
    {
        Err(McError::NotFound("Horizon para Windows: sin release fijado todavia".into()))
    }
}

/// El nightly se sobrescribe en cada publicacion: el hash no se puede fijar
/// (caduca en un dia). Se baja el `SHA256SUMS` que publica junto al paquete y
/// se verifica contra ESO, como haria un repo: integridad garantizada por el
/// editor, sin que el pin se venza.
fn nightly_sha256(url: &str) -> McResult<String> {
    let sums_url = url.rsplit_once('/').map(|(base, _)| format!("{base}/SHA256SUMS")).ok_or_else(|| McError::Invalid("url de Horizon sin checksums".into()))?;
    let text = drakestar_net::fetch_text(&sums_url)?;
    let name = url.rsplit_once('/').map(|(_, n)| n).unwrap_or("");
    sha_in_sums(&text, name).ok_or_else(|| McError::NotFound(format!("SHA256SUMS de Horizon no trae {name}")))
}

/// Lee el hash de `name` dentro de un `SHA256SUMS` (`hash  archivo`, con el
/// `*` opcional de binario).
pub fn sha_in_sums(sums: &str, name: &str) -> Option<String> {
    for line in sums.lines() {
        let mut f = line.split_whitespace();
        if let (Some(hash), Some(file)) = (f.next(), f.next()) {
            if file.trim_start_matches('*') == name && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(hash.to_string());
            }
        }
    }
    None
}

/// Baja y verifica el paquete. Se extrae **completo** a `pkg/`: además de los
/// binarios trae `research-lock.json`, que `horizon-prepare` necesita junto
/// al programa (o vía `--root`, que es lo que le pasamos al preparar).
/// Exige SHA-256 valido (el que publica el nightly): sin hash no se instala.
pub fn install(hz: &Hz) -> McResult<()> {
    let url = package_url()?;
    let sha = nightly_sha256(url)?;
    std::fs::create_dir_all(&hz.shared)?;
    let tmp = hz.shared.join("horizon.tar.gz");
    let _ = std::fs::remove_file(&tmp); // si quedo uno a medias de un intento fallido
    let n = download_verified(url, &tmp, &sha, None, |_| {})?;

    let pkg = hz.pkg();
    let _ = std::fs::remove_dir_all(&pkg);
    std::fs::create_dir_all(&pkg)?;
    // `--strip-components=1`: el tar trae una carpeta con el nombre de la
    // plataforma; su contenido (binarios + research-lock.json) queda en la
    // raíz de `pkg/`, que es lo que `--root` después le pasa a preparar.
    let st = std::process::Command::new("tar")
        .arg("xzf")
        .arg(&tmp)
        .arg("-C")
        .arg(&pkg)
        .arg("--strip-components=1")
        .status()
        .map_err(McError::Io)?;
    if !st.success() {
        let _ = std::fs::remove_dir_all(&pkg);
        let _ = std::fs::remove_file(&tmp);
        return Err(McError::Io(std::io::Error::other("falló la extracción del paquete de Horizon")));
    }
    let _ = std::fs::remove_file(&tmp);
    if !hz.installed() {
        let _ = std::fs::remove_dir_all(&pkg);
        return Err(McError::NotFound("el paquete de Horizon no trae los binarios".into()));
    }
    let _ = n;
    Ok(())
}

// ---------- insumos (lo que lee horizon-prepare) ----------

/// Los archivos que `horizon-prepare` lee en modo launcher de terceros.
pub struct Inputs {
    pub version_json: PathBuf,
    pub client_jar: PathBuf,
    pub libraries_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub server_jar: PathBuf,
}

/// Los pins que Horizon publica junto a sus binarios (`research-lock.json`):
/// qué build de cada archivo del juego acepta su preparador.
pub struct Lock {
    /// SHA-1 de los JSON de versión que acepta (Mojang republiega la 26.3).
    pub accepted_manifest: Vec<String>,
    /// El fijado, para bajarlo cuando el nuestro no está en la lista.
    pub manifest_sha1: Option<String>,
    pub client_sha1: Option<String>,
    pub server_sha1: Option<String>,
    pub java_major: u32,
}

/// Lee el lock del paquete instalado. Sin lock no se puede preparar: es lo que
/// le dice a Horizon que nuestros archivos son los que él espera.
pub fn read_lock(hz: &Hz) -> McResult<Lock> {
    let path = hz.pkg().join("research-lock.json");
    let text = std::fs::read_to_string(&path).map_err(|_| McError::NotFound("el paquete de Horizon no trae research-lock.json".into()))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| McError::Invalid(format!("research-lock.json ilegible: {e}")))?;
    let m = v.get("minecraft").ok_or_else(|| McError::Invalid("research-lock.json sin seccion minecraft".into()))?;
    let list = |key: &str| -> Vec<String> {
        m.get(key)
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    Ok(Lock {
        accepted_manifest: list("accepted_manifest_sha1"),
        manifest_sha1: m.get("manifest_sha1").and_then(|s| s.as_str()).map(String::from),
        client_sha1: m.get("client_sha1").and_then(|s| s.as_str()).map(String::from),
        server_sha1: m.get("server_sha1").and_then(|s| s.as_str()).map(String::from),
        java_major: m.get("java_major").and_then(|n| n.as_u64()).unwrap_or(PREPARE_JAVA as u64) as u32,
    })
}

fn sha1_of(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(&bytes);
    Some(bytes_to_hex(h.finalize().as_slice()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Deja en `dirs` un JSON de versión que Horizon acepta. Mojang republiega la
/// 26.3 (cambia solo el assetIndex): si el que tenemos no está en su lista, se
/// baja el que ellos fijan por su SHA-1.
pub fn ensure_pinned_manifest(hz: &Hz, d: &Dirs, mc: &str, sink: &dyn ProgressSink) -> McResult<()> {
    let lock = read_lock(hz)?;
    let path = d.version_json(mc);
    if let Some(h) = sha1_of(&path) {
        if lock.accepted_manifest.contains(&h) || lock.manifest_sha1.as_deref() == Some(h.as_str()) {
            return Ok(());
        }
    }
    let sha = lock.manifest_sha1.ok_or_else(|| McError::NotFound("Horizon no fija un manifest para esa version".into()))?;
    sink.stage("minecraft", &format!("Bajando la build de {mc} que Horizon acepta"));
    // el hash del paquete de piston ES el sha1 de su contenido
    let url = format!("https://piston-meta.mojang.com/v1/packages/{sha}/{mc}.json");
    download_verified(&url, &path, &sha, None, |_| {})?;
    Ok(())
}

/// Resuelve los insumos: deja el JSON de versión que Horizon acepta, instala la
/// vanilla `mc` (jar, librerías y assets compartidos, idempotente) y baja el
/// server jar, que Horizon pide y que el camino Java no usa.
pub fn ensure_inputs(hz: &Hz, d: &Dirs, mc: &str, sink: &dyn ProgressSink) -> McResult<Inputs> {
    ensure_pinned_manifest(hz, d, mc, sink)?;
    crate::install::install(d, mc, sink)?;
    let vjson_path = d.version_json(mc);
    let v: VersionJson =
        serde_json::from_str(&std::fs::read_to_string(&vjson_path)?).map_err(|e| McError::Invalid(format!("version JSON ilegible: {e}")))?;
    let server_jar = d.version_dir(mc).join("server.jar");
    match v.downloads.as_ref().and_then(|d| d.server.as_ref()) {
        Some(f) if !f.url.is_empty() => {
            download_verified(&f.url, &server_jar, &f.sha1, Some(f.size), |_| {})?;
        }
        _ => return Err(McError::NotFound(format!("la version {mc} no trae server jar"))),
    }
    Ok(Inputs {
        version_json: vjson_path,
        client_jar: d.version_jar(mc),
        libraries_dir: d.libraries(),
        assets_dir: d.assets(),
        server_jar,
    })
}

/// El JDK que pide la preparación (no el JRE del juego).
pub fn prepare_java(d: &Dirs, sink: &dyn ProgressSink) -> McResult<PathBuf> {
    let exe = crate::java::ensure_jdk(d, PREPARE_JAVA, sink)?;
    // ensure_jdk devuelve `.../bin/java(.exe)`, pero horizon-prepare espera JAVA_HOME
    Ok(exe
        .parent()
        .and_then(|bin| bin.parent())
        .map(Path::to_path_buf)
        .unwrap_or(exe))
}

/// Args de `horizon-prepare` (`--complete`: recursos + sonidos, falla si algo
/// falta; es idempotente gracias a su `--check` interno). `--root` apunta a
/// la carpeta del paquete, donde vive el `research-lock.json` con los pins.
pub fn prepare_args(hz: &Hz, inputs: &Inputs, java_home: &Path) -> Vec<String> {
    let mut a = vec![
        "--prepare".to_string(),
        "--complete".to_string(),
        "--root".to_string(),
        hz.pkg().to_string_lossy().into_owned(),
        "--version-json".to_string(),
        inputs.version_json.to_string_lossy().into_owned(),
        "--client-jar".to_string(),
        inputs.client_jar.to_string_lossy().into_owned(),
        "--libraries-dir".to_string(),
        inputs.libraries_dir.to_string_lossy().into_owned(),
        "--assets-dir".to_string(),
        inputs.assets_dir.to_string_lossy().into_owned(),
        "--server-jar".to_string(),
        inputs.server_jar.to_string_lossy().into_owned(),
        "--java-home".to_string(),
        java_home.to_string_lossy().into_owned(),
        "--output".to_string(),
        hz.resources().to_string_lossy().into_owned(),
    ];
    a.shrink_to_fit();
    a
}

/// Corre la preparación. Devuelve error si Horizon informa que falta algo.
pub fn prepare(hz: &Hz, inputs: &Inputs, java_home: &Path) -> McResult<()> {
    let args = prepare_args(hz, inputs, java_home);
    let st = std::process::Command::new(hz.prepare())
        .args(&args)
        .output()
        .map_err(McError::Io)?;
    if !st.status.success() {
        let msg = String::from_utf8_lossy(&st.stderr);
        return Err(McError::Io(std::io::Error::other(format!(
            "horizon-prepare falló: {}",
            msg.lines().last().unwrap_or("").trim()
        ))));
    }
    Ok(())
}

// ---------- lanzamiento ----------

/// Plan de lanzamiento del cliente nativo.
///
/// `server` es `host[:puerto]` (modo online, adaptador experimental) y
/// `session` el archivo con la sesión de la cuenta de Microsoft que produce
/// nuestro login (`drakestar-rust-client --login` + el puente de sesión).
pub fn launch_plan(hz: &Hz, server: Option<&str>, session: Option<&Path>, render_distance: u32) -> Plan {
    let mut args = vec![
        "--resources".to_string(),
        hz.resources().to_string_lossy().into_owned(),
        "--home".to_string(),
        hz.home.to_string_lossy().into_owned(),
        "--render-distance".to_string(),
        render_distance.clamp(2, 128).to_string(),
    ];
    let mut envs = std::collections::BTreeMap::new();
    // X11 para que el renombrado de ventana funcione; en Wayland puro el
    // título se queda como está (no hay forma desde fuera).
    envs.insert("WINIT_UNIX_BACKEND".to_string(), "x11".to_string());
    envs.insert("HORIZON_HOME".to_string(), hz.home.to_string_lossy().into_owned());
    if let Some(s) = server {
        args.push("--server".to_string());
        args.push(s.to_string());
        envs.insert("HORIZON_OLDER_SERVERS".to_string(), "1".to_string());
        if let Some(f) = session {
            args.push("--session-file".to_string());
            args.push(f.to_string_lossy().into_owned());
        }
    }
    Plan {
        program: hz.client(),
        args,
        cwd: hz.home.clone(),
        envs,
    }
}

// ---------- título de ventana ----------

/// Saca el id de ventana de un pid de la salida de `wmctrl -lp`
/// (`0x02400007  0 1234  host  Título`).
pub fn parse_wmctrl_window(out: &str, pid: u32) -> Option<String> {
    for line in out.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 4 && f[2].parse::<u32>().ok() == Some(pid) {
            return Some(f[0].to_string());
        }
    }
    None
}

/// Ventana por título (el juego arranca como "Horizon"): el pid que tenemos no
/// siempre es el de la ventana —el proceso se bifurca—, pero el título sí.
pub fn parse_wmctrl_by_title(out: &str, title: &str) -> Option<String> {
    let want = title.to_lowercase();
    for line in out.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 5 {
            // f[0]=ventana, f[1]=escritorio, f[2]=pid, f[3]=host, f[4..]=título
            let name = f[4..].join(" ").to_lowercase();
            if name == want || name.starts_with(&format!("{want} ")) {
                return Some(f[0].to_string());
            }
        }
    }
    None
}

/// Renombra la ventana del juego. Best-effort: si el compositor es Wayland
/// puro, wmctrl no ve la ventana y se deja el título original.
pub fn retitle(pid: u32, title: &str, rounds: usize, interval: Duration) -> bool {
    let mut renamed = false;
    for _ in 0..rounds.max(1) {
        if let Ok(out) = std::process::Command::new("wmctrl").arg("-lp").output() {
            let text = String::from_utf8_lossy(&out.stdout);
            let wid = parse_wmctrl_window(&text, pid).or_else(|| parse_wmctrl_by_title(&text, "Horizon"));
            if let Some(wid) = wid {
                let _ = std::process::Command::new("wmctrl")
                    .args(["-i", "-r", &wid, "-T", title])
                    .status();
                renamed = true;
            }
        }
        std::thread::sleep(interval);
    }
    renamed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hz() -> Hz {
        Hz { shared: PathBuf::from("/data/horizon"), home: PathBuf::from("/data/instances/x/horizon-home") }
    }
    fn inputs() -> Inputs {
        Inputs {
            version_json: PathBuf::from("/data/versions/26.3/26.3.json"),
            client_jar: PathBuf::from("/data/versions/26.3/26.3.jar"),
            libraries_dir: PathBuf::from("/data/libraries"),
            assets_dir: PathBuf::from("/data/assets"),
            server_jar: PathBuf::from("/data/versions/26.3/server.jar"),
        }
    }

    #[test]
    fn prepare_usa_archivos_nombrados_y_complete() {
        let a = prepare_args(&hz(), &inputs(), Path::new("/opt/jdk25"));
        let j = a.join(" ");
        assert!(j.contains("--prepare --complete"), "debe ser idempotente y completo: {j}");
        for f in [
            "--root /data/horizon/pkg",
            "--version-json /data/versions/26.3/26.3.json",
            "--client-jar",
            "--libraries-dir /data/libraries",
            "--assets-dir /data/assets",
            "--server-jar",
            "--java-home /opt/jdk25",
            "--output /data/horizon/resources/26.3",
        ] {
            assert!(j.contains(f), "falta {f} en {j}");
        }
    }

    #[test]
    fn plan_singleplayer_sin_servidor() {
        let p = launch_plan(&hz(), None, None, DEFAULT_RENDER_DISTANCE);
        let j = p.args.join(" ");
        assert!(j.contains(&format!("--resources /data/horizon/resources/{MC_NATIVE}")));
        assert!(j.contains("--render-distance 8"));
        assert!(!j.contains("--server"));
        assert!(!j.contains("--session-file"));
        assert_eq!(p.envs.get("WINIT_UNIX_BACKEND").map(String::as_str), Some("x11"), "fuerza X11 para poder renombrar la ventana");
        assert_eq!(p.envs.get("HORIZON_HOME").map(String::as_str), Some(hz().home.to_string_lossy().as_ref()));
    }

    #[test]
    fn plan_servidor_enciende_adaptador_y_sesion() {
        let p = launch_plan(&hz(), Some("mc.drakescraft.cl:25565"), Some(Path::new("/inst/sesion.json")), 12);
        let j = p.args.join(" ");
        assert!(j.contains("--server mc.drakescraft.cl:25565 --session-file /inst/sesion.json"));
        assert_eq!(p.envs.get("HORIZON_OLDER_SERVERS").map(String::as_str), Some("1"), "26.2 va por el adaptador experimental");
    }

    #[test]
    fn distancia_de_render_se_acota() {
        let cerca = launch_plan(&hz(), None, None, 1).args.join(" ");
        assert!(cerca.contains("--render-distance 2"), "{cerca}");
        let lejos = launch_plan(&hz(), None, None, 900).args.join(" ");
        assert!(lejos.contains("--render-distance 128"), "{lejos}");
    }

    #[test]
    fn wmctrl_por_titulo() {
        let out = "0x02400007  0 1234 host Horizon\n0x02c00002  0 999 otro DrakeStar\n";
        assert_eq!(parse_wmctrl_by_title(out, "Horizon").as_deref(), Some("0x02400007"));
        assert_eq!(parse_wmctrl_by_title(out, "DrakeStar 26.3").as_deref(), None);
    }

    #[test]
    fn wmctrl_por_pid() {
        let out = "0x02400007  0 1234 host Horizon\n0x02c00002  0 999 otro DrakeStar\n";
        assert_eq!(parse_wmctrl_window(out, 1234).as_deref(), Some("0x02400007"));
        assert_eq!(parse_wmctrl_window(out, 7), None);
        assert_eq!(parse_wmctrl_window("basura", 1), None);
    }

    #[test]
    fn lock_del_paquete_se_lee() {
        let dir = std::env::temp_dir().join(format!("hz-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pkg = dir.join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("research-lock.json"),
            r#"{"minecraft":{"version":"26.3","manifest_sha1":"11f1aaa","accepted_manifest_sha1":["969fbbb","4fe1ccc"],"client_sha1":"e877","server_sha1":"3368","java_major":25}}"#,
        )
        .unwrap();
        let hz = Hz { shared: dir.clone(), home: dir.join("home") };
        let lock = read_lock(&hz).unwrap();
        assert_eq!(lock.java_major, 25);
        assert_eq!(lock.manifest_sha1.as_deref(), Some("11f1aaa"));
        assert_eq!(lock.accepted_manifest.len(), 2);
        assert!(lock.accepted_manifest.contains(&"4fe1ccc".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_que_horizon_acepta_no_se_vuelve_a_bajar() {
        let dir = std::env::temp_dir().join(format!("hz-man-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pkg = dir.join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        let d = crate::Dirs::new(dir.join("data"));
        std::fs::create_dir_all(d.version_dir("26.3")).unwrap();
        let json = d.version_json("26.3");
        std::fs::write(&json, br#"{"id":"26.3"}"#).unwrap();
        let h = sha1_of(&json).unwrap();
        std::fs::write(
            pkg.join("research-lock.json"),
            format!(r#"{{"minecraft":{{"accepted_manifest_sha1":["{h}"]}}}}"#),
        )
        .unwrap();
        let hz = Hz { shared: dir.clone(), home: dir.join("home") };
        // está en la lista: no hace falta red y no toca el archivo
        assert!(ensure_pinned_manifest(&hz, &d, "26.3", &crate::NullSink).is_ok());
        assert_eq!(std::fs::read(&json).unwrap(), br#"{"id":"26.3"}"#);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha_del_nightly_se_lee_de_sus_sums() {
        let sums = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  horizon-server-linux-x86_64.tar.gz\n\
                    83d36c5a2f8b6690d4d77411dfc3f85753319184fffe738ee5f0a4c552a0eabe  horizon-linux-x86_64.tar.gz\n";
        assert_eq!(sha_in_sums(sums, "horizon-linux-x86_64.tar.gz").as_deref(), Some("83d36c5a2f8b6690d4d77411dfc3f85753319184fffe738ee5f0a4c552a0eabe"));
        assert_eq!(sha_in_sums(sums, "no-existe.tar.gz"), None);
        assert_eq!(sha_in_sums("basura", "horizon-linux-x86_64.tar.gz"), None);
    }
}