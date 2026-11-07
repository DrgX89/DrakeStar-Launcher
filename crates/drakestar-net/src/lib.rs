//! Descarga y verificacion de artefactos. Politica:
//! - solo `https://` (o `http://` hacia loopback, para pruebas locales);
//! - se escribe a `<dest>.part`, se calcula el hash en streaming y SOLO si coincide
//!   (y el tamano, si se conoce) se renombra a `dest`; si no, se borra el parcial;
//! - nunca se ejecuta nada descargado.
//!
//! El hash esperado puede ser SHA-256 (nuestro manifiesto) o SHA-1 (los JSON de
//! version y de Fabric de Mojang); se distingue por la longitud y se calculan
//! ambos mientras se escribe, asi el streaming no depende de cual se pidio.

use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Hash esperado de un artefacto, con el algoritmo implicito en su longitud.
#[derive(Debug, Clone, PartialEq)]
pub enum Hash {
    /// 64 hex: nuestro manifiesto de releases.
    Sha256(String),
    /// 40 hex: los JSON de Mojang y de Fabric.
    Sha1(String),
}

impl Hash {
    /// `None` si no es un hex de 40 o 64 caracteres.
    pub fn parse(s: &str) -> Option<Hash> {
        let s = s.trim();
        let hex = |n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit());
        if hex(64) {
            Some(Hash::Sha256(s.to_ascii_lowercase()))
        } else if hex(40) {
            Some(Hash::Sha1(s.to_ascii_lowercase()))
        } else {
            None
        }
    }

    fn matches(&self, actual: &str) -> bool {
        let expected = match self {
            Hash::Sha256(e) | Hash::Sha1(e) => e,
        };
        actual.eq_ignore_ascii_case(expected)
    }
}

#[derive(Debug)]
pub enum NetError {
    InsecureUrl(String),
    BadSha(String),
    Http(String),
    Io(std::io::Error),
    ShaMismatch { expected: String, actual: String },
    SizeMismatch { expected: u64, actual: u64 },
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsecureUrl(u) => write!(f, "URL no permitida (se exige https): {u}"),
            Self::BadSha(s) => write!(f, "SHA-256 esperado malformado: '{s}'"),
            Self::Http(e) => write!(f, "error HTTP: {e}"),
            Self::Io(e) => write!(f, "error de E/S: {e}"),
            Self::ShaMismatch { expected, actual } => {
                write!(f, "SHA-256 no coincide: esperado {expected}, obtenido {actual}")
            }
            Self::SizeMismatch { expected, actual } => {
                write!(f, "tamano no coincide: esperado {expected}, obtenido {actual}")
            }
        }
    }
}
impl std::error::Error for NetError {}
impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub fn sha256_wellformed(s: &str) -> bool {
    matches!(Hash::parse(s), Some(Hash::Sha256(_)))
}

/// SHA-1 bien formado (lo que publican los JSON de Mojang/Fabric).
pub fn sha1_wellformed(s: &str) -> bool {
    matches!(Hash::parse(s), Some(Hash::Sha1(_)))
}

pub fn sha256_file(path: &Path) -> Result<String, NetError> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

pub fn sha1_file(path: &Path) -> Result<String, NetError> {
    let mut f = File::open(path)?;
    let mut h = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// https siempre; http solo hacia 127.0.0.1 / localhost.
pub fn url_allowed(url: &str) -> bool {
    if url.starts_with("https://") {
        return true;
    }
    if let Some(rest) = url.strip_prefix("http://") {
        let host = rest.split(['/', ':']).next().unwrap_or("");
        return host == "127.0.0.1" || host == "localhost";
    }
    false
}

/// Verifica un reader contra el hash/tamano esperados y escribe a `dest` atomicamente.
/// Separado de la red para poder probarlo sin sockets.
pub fn store_verified<R: Read>(
    mut src: R,
    dest: &Path,
    expected_sha256: &str,
    expected_size: Option<u64>,
    mut progress: impl FnMut(u64),
) -> Result<u64, NetError> {
    let expected = Hash::parse(expected_sha256).ok_or_else(|| NetError::BadSha(expected_sha256.into()))?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let part: PathBuf = {
        let mut s = dest.as_os_str().to_owned();
        s.push(".part");
        s.into()
    };
    let result = (|| -> Result<u64, NetError> {
        let mut out = File::create(&part)?;
        let mut h256 = Sha256::new();
        let mut h1 = Sha1::new();
        let mut total = 0u64;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = src.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h256.update(&buf[..n]);
            h1.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            total += n as u64;
            progress(total);
        }
        out.flush()?;
        let actual = match expected {
            Hash::Sha256(_) => hex(&h256.finalize()),
            Hash::Sha1(_) => hex(&h1.finalize()),
        };
        if !expected.matches(&actual) {
            return Err(NetError::ShaMismatch {
                expected: match &expected {
                    Hash::Sha256(e) | Hash::Sha1(e) => e.clone(),
                },
                actual,
            });
        }
        if let Some(sz) = expected_size {
            if sz != 0 && sz != total {
                return Err(NetError::SizeMismatch { expected: sz, actual: total });
            }
        }
        Ok(total)
    })();
    match result {
        Ok(total) => {
            fs::rename(&part, dest)?;
            Ok(total)
        }
        Err(e) => {
            let _ = fs::remove_file(&part);
            Err(e)
        }
    }
}

/// Lee una URL https (o loopback) como texto: los JSON de Mojang y de Fabric.
/// La verificacion de esos JSON corre por cuenta de quien los usa (hashes de
/// artefactos o firma, cuando exista).
pub fn fetch_text(url: &str) -> Result<String, NetError> {
    if !url_allowed(url) {
        return Err(NetError::InsecureUrl(url.into()));
    }
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| NetError::Http(e.to_string()))?;
    resp.into_string().map_err(|e| NetError::Http(e.to_string()))
}

/// Descarga `url` a `dest` verificando SHA-256 (y tamano si `expected_size` es `Some(>0)`).
pub fn download_verified(
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    expected_size: Option<u64>,
    progress: impl FnMut(u64),
) -> Result<u64, NetError> {
    if !url_allowed(url) {
        return Err(NetError::InsecureUrl(url.into()));
    }
    if Hash::parse(expected_sha256).is_none() {
        return Err(NetError::BadSha(expected_sha256.into()));
    }
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| NetError::Http(e.to_string()))?;
    store_verified(resp.into_reader(), dest, expected_sha256, expected_size, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::net::TcpListener;

    const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("drakestar-net-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn stores_when_hash_matches() {
        let d = tmp("ok");
        let dest = d.join("a.jar");
        let n = store_verified(Cursor::new(b"hello"), &dest, HELLO_SHA, Some(5), |_| {}).unwrap();
        assert_eq!(n, 5);
        assert_eq!(fs::read(&dest).unwrap(), b"hello");
        assert_eq!(sha256_file(&dest).unwrap(), HELLO_SHA);
        assert!(!d.join("a.jar.part").exists());
    }

    #[test]
    fn stores_when_sha1_matches() {
        let hello_sha1 = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        let d = tmp("sha1");
        let dest = d.join("a.jar");
        let n = store_verified(Cursor::new(b"hello"), &dest, hello_sha1, Some(5), |_| {}).unwrap();
        assert_eq!(n, 5);
        assert_eq!(fs::read(&dest).unwrap(), b"hello");
        assert!(!d.join("a.jar.part").exists());
        // sha1 mal formado o que no coincide: se rechaza antes de escribir
        let e = store_verified(Cursor::new(b"hello"), &dest, "aaf4c61d", None, |_| {}).unwrap_err();
        assert!(matches!(e, NetError::BadSha(_)));
        let bad = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434e";
        assert!(store_verified(Cursor::new(b"hello"), &d.join("b.jar"), bad, None, |_| {}).is_err());
        assert!(sha1_wellformed(hello_sha1) && !sha1_wellformed(&"a".repeat(64)));
    }

    #[test]
    fn rejects_and_cleans_on_mismatch() {
        let d = tmp("bad");
        let dest = d.join("a.jar");
        let e = store_verified(Cursor::new(b"hellx"), &dest, HELLO_SHA, None, |_| {}).unwrap_err();
        assert!(matches!(e, NetError::ShaMismatch { .. }));
        assert!(!dest.exists());
        assert!(!d.join("a.jar.part").exists());
    }

    #[test]
    fn rejects_size_mismatch_and_bad_sha() {
        let d = tmp("size");
        let dest = d.join("a.jar");
        let e = store_verified(Cursor::new(b"hello"), &dest, HELLO_SHA, Some(9), |_| {}).unwrap_err();
        assert!(matches!(e, NetError::SizeMismatch { .. }));
        let e = store_verified(Cursor::new(b"hello"), &dest, "xyz", None, |_| {}).unwrap_err();
        assert!(matches!(e, NetError::BadSha(_)));
    }

    #[test]
    fn url_policy() {
        assert!(url_allowed("https://example.net/x.jar"));
        assert!(url_allowed("http://127.0.0.1:8080/x"));
        assert!(url_allowed("http://localhost/x"));
        assert!(!url_allowed("http://example.net/x.jar"));
        assert!(!url_allowed("ftp://x/y"));
        assert!(!url_allowed("http://127.0.0.1.evil.net/x"));
    }

    #[test]
    fn download_from_local_http_server() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let _ = s.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                );
            }
        });
        let d = tmp("http");
        let dest = d.join("dl.bin");
        let url = format!("http://127.0.0.1:{port}/dl.bin");
        let n = download_verified(&url, &dest, HELLO_SHA, Some(5), |_| {}).unwrap();
        assert_eq!(n, 5);
        assert_eq!(fs::read(&dest).unwrap(), b"hello");
    }

    #[test]
    fn insecure_url_refused_before_any_io() {
        let d = tmp("insecure");
        let e = download_verified("http://example.net/x", &d.join("x"), HELLO_SHA, None, |_| {})
            .unwrap_err();
        assert!(matches!(e, NetError::InsecureUrl(_)));
    }
}
