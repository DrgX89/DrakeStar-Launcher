//! Instalacion del cliente Rust gestionado (modo DrakesRust).
//!
//! Mismo camino que el JAR del servidor local (docs de `release.rs`): se resuelve
//! el artefacto `drakes-rust-client` por canal/plataforma/Minecraft y se descarga
//! verificado por SHA-256 a `profiles/<id>/instance/`, dejando el binario
//! ejecutable. La ejecucion la hace el `ProcessManager`, que ya es generico.

use crate::{layout::write_atomic, release, AppError, AppResult, Layout};
use drakestar_core::release::Manifest;
use drakestar_core::{Mode, Profile};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Nombre del binario por plataforma: en Windows hace falta la extension.
#[cfg(windows)]
pub const CLIENT_BIN: &str = "drakestar-rust-client.exe";
#[cfg(not(windows))]
pub const CLIENT_BIN: &str = "drakestar-rust-client";

/// Lo que se guarda junto al binario para poder mostrarlo en la UI.
#[derive(Debug, Serialize, Deserialize)]
struct Installed {
    version: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RustClientStatus {
    pub profile: String,
    pub binary: String,
    pub installed: bool,
    pub executable: bool,
    /// Version de la release instalada (del manifiesto), si se instalo por ahi.
    pub version: Option<String>,
    pub ready: bool,
    pub reason: Option<String>,
}

pub fn bin_path(l: &Layout, id: &str) -> PathBuf {
    l.instance_dir(id).join(CLIENT_BIN)
}

fn installed_json(l: &Layout, id: &str) -> PathBuf {
    l.instance_dir(id).join("rust-client.json")
}

#[cfg(unix)]
fn make_executable(p: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(p)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(p, perms)?;
    Ok(())
}
#[cfg(not(unix))]
fn make_executable(_p: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}
#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

pub fn status(l: &Layout, p: &Profile) -> RustClientStatus {
    let bin = bin_path(l, &p.id);
    let installed = bin.is_file();
    let executable = is_executable(&bin);
    let version = std::fs::read_to_string(installed_json(l, &p.id))
        .ok()
        .and_then(|t| serde_json::from_str::<Installed>(&t).ok())
        .map(|i| i.version);
    let (ready, reason) = if !installed {
        (false, Some("el cliente Rust no esta instalado".into()))
    } else if !executable {
        (false, Some("el binario no tiene permiso de ejecucion".into()))
    } else {
        (true, None)
    };
    RustClientStatus {
        profile: p.id.clone(),
        binary: bin.to_string_lossy().to_string(),
        installed,
        executable,
        version,
        ready,
        reason,
    }
}

/// Descarga el binario resuelto para el perfil. Exige SHA-256 valido en el
/// manifiesto: sin hash no se instala (igual que el JAR del servidor).
pub fn install(l: &Layout, p: &Profile, manifest: Option<&Manifest>) -> AppResult<RustClientStatus> {
    if p.mode != Mode::DrakesRust {
        return Err(AppError::Invalid("solo los perfiles drakes-rust instalan el cliente Rust".into()));
    }
    let res = release::resolve(manifest, p);
    let rel = res.resolved.ok_or_else(|| AppError::NotFound(res.reason.unwrap_or_else(|| "release del cliente".into())))?;
    if !rel.sha256_wellformed {
        return Err(AppError::Invalid("el manifiesto no trae un SHA-256 valido: no se instala".into()));
    }
    let dest = bin_path(l, &p.id);
    let n = drakestar_net::download_verified(&rel.url, &dest, &rel.sha256, Some(rel.size), |_| {})?;
    make_executable(&dest)?;
    write_atomic(
        &installed_json(l, &p.id),
        serde_json::to_string_pretty(&Installed { version: rel.version.clone(), sha256: rel.sha256.clone() })?.as_bytes(),
    )?;
    let st = status(l, p);
    if !st.ready {
        return Err(AppError::Io(format!("el binario descargado ({n} bytes) no quedo ejecutable")));
    }
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;
    use drakestar_core::Channel;
    use std::io::{Read, Write};

    fn manifest(url: &str, sha: &str, size: u64) -> String {
        format!(
            r#"{{"schema":1,"releases":[
 {{"version":"1.0.0","channel":"stable","minecraft":["26.2"],"artifacts":[{{"component":"drakes-rust-client","platform":"{}","url":"{url}","sha256":"{sha}","size":{size}}}]}},
 {{"version":"1.1.0-nightly","channel":"nightly","minecraft":["26.2"],"artifacts":[{{"component":"drakes-rust-client","platform":"{}","url":"{url}","sha256":"{sha}","size":{size}}}]}}
 ]}}"#,
            crate::release::platform(),
            crate::release::platform()
        )
    }

    /// Sirve `body` por HTTP tantas veces como se pida.
    fn http_once(body: &'static str) -> u16 {
        let lst = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = lst.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for _ in 0..4 {
                if let Ok((mut s, _)) = lst.accept() {
                    let mut b = [0u8; 1024];
                    let _ = s.read(&mut b);
                    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes());
                    let _ = s.write_all(body.as_bytes());
                }
            }
        });
        port
    }

    fn rust_profile(id: &str) -> Profile {
        let mut p = crate::store::tests::sample(id, Mode::DrakesRust);
        p.channel = Channel::Stable;
        p.server = Some("localhost:25565".into());
        p
    }

    #[test]
    fn install_downloads_verified_and_marks_executable() {
        // sha256 de "hello" (el mismo de los tests de drakestar-net)
        let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let l = Layout::new(crate::testutil::tmpdir("rc-inst"));
        let port = http_once("hello");
        let url = format!("http://127.0.0.1:{port}/client");
        let m = Manifest::from_json(&manifest(&url, sha, 5)).unwrap();
        let p = rust_profile("r1");
        let st = install(&l, &p, Some(&m)).unwrap();
        assert_eq!(st.version.as_deref(), Some("1.0.0"));
        assert!(st.ready && st.installed && st.executable);
        assert_eq!(std::fs::read(bin_path(&l, "r1")).unwrap(), b"hello");
        assert!(status(&l, &p).ready);
    }

    #[test]
    fn install_refuses_bad_sha_and_non_rust_profiles() {
        let l = Layout::new(crate::testutil::tmpdir("rc-bad"));
        let port = http_once("hola");
        let url = format!("http://127.0.0.1:{port}/client");
        let bad = Manifest::from_json(&manifest(&url, &"b".repeat(64), 4)).unwrap();
        let p = rust_profile("r2");
        assert!(install(&l, &p, Some(&bad)).is_err());
        assert!(!bin_path(&l, "r2").exists());
        // un perfil de otro modo no instala el cliente Rust
        let j = crate::store::tests::sample("j", Mode::DrakesJava);
        assert!(install(&l, &j, None).is_err());
        // sin manifiesto no hay release que resolver
        assert!(install(&l, &p, None).is_err());
    }

    #[test]
    fn status_reports_not_installed() {
        let l = Layout::new(crate::testutil::tmpdir("rc-status"));
        let p = rust_profile("r3");
        let st = status(&l, &p);
        assert!(!st.ready && !st.installed && st.version.is_none());
        assert!(st.reason.is_some());
    }
}
