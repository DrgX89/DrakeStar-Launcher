//! Administracion de instancias: cada perfil tiene un directorio de datos
//! (servidor o instancia de cliente) y logs. Aqui se crean, miden y limpian.

use crate::server::{dir_size, modified_ms, ServerDir};
use crate::{layout::safe_file_name, AppError, AppResult, Layout};
use drakestar_core::{Mode, Profile};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InstanceInfo {
    pub id: String,
    pub path: String,
    pub data_dir: String,
    pub data_exists: bool,
    pub size_bytes: u64,
    pub modified: Option<u64>,
    pub server_jar: Option<String>,
    pub launcher_logs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogFile {
    /// "launcher" (logs del proceso capturados por DrakeStar) o "server" (logs/ del servidor).
    pub source: String,
    pub name: String,
    pub size: u64,
    pub modified: Option<u64>,
}

/// Directorio de datos segun el modo.
pub fn data_dir(l: &Layout, p: &Profile) -> std::path::PathBuf {
    match p.mode {
        Mode::LocalServer => l.server_dir(&p.id),
        _ => l.instance_dir(&p.id),
    }
}

/// Crea la estructura en disco del perfil (idempotente). Los modos Online no tienen datos.
pub fn ensure_dirs(l: &Layout, p: &Profile) -> AppResult<()> {
    std::fs::create_dir_all(l.logs_dir(&p.id))?;
    match p.mode {
        Mode::LocalServer => ServerDir::new(l.server_dir(&p.id)).ensure()?,
        Mode::Online => {}
        _ => std::fs::create_dir_all(l.instance_dir(&p.id))?,
    }
    Ok(())
}

pub fn info(l: &Layout, p: &Profile) -> InstanceInfo {
    let dd = data_dir(l, p);
    let pd = l.profile_dir(&p.id);
    InstanceInfo {
        id: p.id.clone(),
        path: pd.to_string_lossy().to_string(),
        data_dir: dd.to_string_lossy().to_string(),
        data_exists: dd.is_dir(),
        size_bytes: dir_size(&pd),
        modified: modified_ms(&pd),
        server_jar: if p.mode == Mode::LocalServer { ServerDir::new(&dd).jar().selected } else { None },
        launcher_logs: list_logs(l, p).iter().filter(|f| f.source == "launcher").count(),
    }
}

pub fn list_logs(l: &Layout, p: &Profile) -> Vec<LogFile> {
    let mut v = vec![];
    let mut scan = |dir: std::path::PathBuf, source: &str, exts: &[&str]| {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if e.path().is_file() && exts.iter().any(|x| name.ends_with(x)) {
                    v.push(LogFile {
                        source: source.into(),
                        name,
                        size: e.metadata().map(|m| m.len()).unwrap_or(0),
                        modified: modified_ms(&e.path()),
                    });
                }
            }
        }
    };
    scan(l.logs_dir(&p.id), "launcher", &[".log"]);
    if p.mode == Mode::LocalServer {
        scan(l.server_dir(&p.id).join("logs"), "server", &[".log"]);
    }
    v.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    v
}

/// Ultimas `max_lines` lineas de un log (lectura acotada a 2 MiB finales).
pub fn read_log_tail(l: &Layout, p: &Profile, source: &str, name: &str, max_lines: usize) -> AppResult<Vec<String>> {
    if !safe_file_name(name) || !name.ends_with(".log") {
        return Err(AppError::Invalid("nombre de log invalido".into()));
    }
    let dir = match source {
        "launcher" => l.logs_dir(&p.id),
        "server" if p.mode == Mode::LocalServer => l.server_dir(&p.id).join("logs"),
        _ => return Err(AppError::Invalid("origen de log invalido".into())),
    };
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(dir.join(name)).map_err(|_| AppError::NotFound(format!("log '{name}'")))?;
    let len = f.metadata()?.len();
    const MAX: u64 = 2 * 1024 * 1024;
    f.seek(SeekFrom::Start(len.saturating_sub(MAX)))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(max_lines);
    Ok(lines[skip..].iter().map(|s| s.to_string()).collect())
}

/// Borra los logs del launcher del perfil. Devuelve cuantos borro.
pub fn clear_launcher_logs(l: &Layout, p: &Profile) -> AppResult<usize> {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(l.logs_dir(&p.id)) {
        for e in rd.flatten() {
            if e.path().is_file() && e.file_name().to_string_lossy().ends_with(".log") {
                std::fs::remove_file(e.path())?;
                n += 1;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::sample;

    #[test]
    fn dirs_info_and_logs() {
        let l = Layout::new(crate::testutil::tmpdir("inst"));
        let p = sample("srv", Mode::LocalServer);
        ensure_dirs(&l, &p).unwrap();
        assert!(l.server_dir("srv").join("plugins").is_dir());
        std::fs::create_dir_all(l.profile_dir("srv")).unwrap();
        std::fs::write(l.profile_dir("srv").join("profile.json"), "{}").unwrap();
        std::fs::write(l.server_dir("srv").join("drakespurpur.jar"), vec![0u8; 100]).unwrap();
        std::fs::write(l.logs_dir("srv").join("launcher-1.log"), "a\nb\nc\n").unwrap();
        std::fs::create_dir_all(l.server_dir("srv").join("logs")).unwrap();
        std::fs::write(l.server_dir("srv").join("logs/latest.log"), "x\ny\n").unwrap();
        let i = info(&l, &p);
        assert!(i.data_exists && i.size_bytes >= 100 + 6 + 4);
        assert_eq!(i.server_jar.as_deref(), Some("drakespurpur.jar"));
        assert_eq!(i.launcher_logs, 1);
        assert_eq!(list_logs(&l, &p).len(), 2);
        assert_eq!(read_log_tail(&l, &p, "launcher", "launcher-1.log", 2).unwrap(), vec!["b", "c"]);
        assert_eq!(read_log_tail(&l, &p, "server", "latest.log", 10).unwrap(), vec!["x", "y"]);
        assert!(read_log_tail(&l, &p, "launcher", "../profile.json", 5).is_err());
        assert!(read_log_tail(&l, &p, "otro", "a.log", 5).is_err());
        assert_eq!(clear_launcher_logs(&l, &p).unwrap(), 1);
        assert_eq!(list_logs(&l, &p).len(), 1);
    }

    #[test]
    fn online_has_no_data_dir_and_clients_get_instance_dir() {
        let l = Layout::new(crate::testutil::tmpdir("inst2"));
        let o = sample("on", Mode::Online);
        ensure_dirs(&l, &o).unwrap();
        assert!(!l.instance_dir("on").exists());
        let j = sample("jv", Mode::DrakesJava);
        ensure_dirs(&l, &j).unwrap();
        assert!(l.instance_dir("jv").is_dir());
    }
}
