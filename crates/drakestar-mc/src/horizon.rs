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

pub fn paths(d: &Dirs, id: &str) -> Hz {
    Hz {
        shared: d.root().join("horizon"),
        home: d.instance(id).join("horizon-home"),
    }
}

impl Hz {
    pub fn client(&self) -> PathBuf {
        self.shared.join("bin").join(CLIENT_BIN)
    }
    pub fn prepare(&self) -> PathBuf {
        self.shared.join("bin").join(PREPARE_BIN)
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

/// Release fijado (nightly verificado: 2026-11-13). Se mueve al manifiesto
/// cuando Horizon publique una versión estable con la que nos quedemos.
pub fn pinned() -> McResult<(&'static str, &'static str, u64)> {
    #[cfg(not(windows))]
    {
        Ok((
            "https://github.com/horizon-rust/horizon/releases/download/nightly/horizon-linux-x86_64.tar.gz",
            // SHA-256 de horizon-linux-x86_64.tar.gz, del SHA256SUMS de ese nightly.
            "83d36c5a2f8b6690d4d77411dfc3f85753319184fffe738ee5f0a4c552a0eabe",
            55_636_015,
        ))
    }
    #[cfg(windows)]
    {
        Err(McError::NotFound("Horizon para Windows: sin release fijado todavia".into()))
    }
}

/// Baja y verifica los binarios. El tar.gz trae cuatro; se dejan los dos que
/// usamos. Exige SHA-256: sin hash válido no se instala.
pub fn install(hz: &Hz) -> McResult<()> {
    let (url, sha, size) = pinned()?;
    std::fs::create_dir_all(hz.shared.join("bin"))?;
    let tmp = hz.shared.join("horizon.tar.gz");
    let n = download_verified(url, &tmp, sha, Some(size), |_| {})?;

    let st = std::process::Command::new("tar")
        .arg("xzf")
        .arg(&tmp)
        .arg("-C")
        .arg(hz.shared.join("bin"))
        .arg("--strip-components=1")
        .arg(CLIENT_BIN)
        .arg(PREPARE_BIN)
        .status()
        .map_err(|e| McError::Io(e))?;
    if !st.success() {
        return Err(McError::Io(std::io::Error::other("falló la extracción del paquete de Horizon")));
    }
    let _ = std::fs::remove_file(&tmp);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for b in [hz.client(), hz.prepare()] {
            let mut perms = std::fs::metadata(&b)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&b, perms)?;
        }
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

/// Instala la vanilla `mc` (jar, librerías y assets compartidos, idempotente) y
/// baja el server jar, que Horizon pide y que el camino Java no usa.
pub fn ensure_inputs(d: &Dirs, mc: &str, sink: &dyn ProgressSink) -> McResult<Inputs> {
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
    crate::java::ensure_jdk(d, PREPARE_JAVA, sink)
}

/// Args de `horizon-prepare` (`--complete`: recursos + sonidos, falla si algo
/// falta; es idempotente gracias a su `--check` interno).
pub fn prepare_args(hz: &Hz, inputs: &Inputs, java_home: &Path) -> Vec<String> {
    let mut a = vec![
        "--prepare".to_string(),
        "--complete".to_string(),
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
        .map_err(|e| McError::Io(e))?;
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

/// Renombra la ventana del juego. Best-effort: si el compositor es Wayland
/// puro, wmctrl no ve la ventana y se deja el título original.
pub fn retitle(pid: u32, title: &str, rounds: usize, interval: Duration) -> bool {
    let mut renamed = false;
    for _ in 0..rounds.max(1) {
        if let Ok(out) = std::process::Command::new("wmctrl").arg("-lp").output() {
            if let Some(wid) = parse_wmctrl_window(&String::from_utf8_lossy(&out.stdout), pid) {
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
    fn wmctrl_por_pid() {
        let out = "0x02400007  0 1234 host Horizon\n0x02c00002  0 999 otro DrakeStar\n";
        assert_eq!(parse_wmctrl_window(out, 1234).as_deref(), Some("0x02400007"));
        assert_eq!(parse_wmctrl_window(out, 7), None);
        assert_eq!(parse_wmctrl_window("basura", 1), None);
    }

    #[test]
    fn pinned_tra_hash_y_url() {
        #[cfg(not(windows))]
        {
            let (url, sha, size) = pinned().unwrap();
            assert!(url.starts_with("https://github.com/horizon-rust/horizon/releases/"));
            assert_eq!(sha.len(), 64);
            assert!(size > 1_000_000);
        }
    }
}