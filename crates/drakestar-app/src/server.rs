//! Gestion del directorio del servidor local: JAR, EULA, server.properties,
//! plugins y mundos. Todo opera sobre una carpeta concreta (testeable con tmpdir).

use crate::{layout::{safe_file_name, write_atomic}, AppError, AppResult};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_JAR: &str = "drakespurpur.jar";

#[derive(Debug, Clone)]
pub struct ServerDir {
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct JarInfo {
    /// Nombre elegido (relativo al directorio del servidor), si hay uno inequivoco.
    pub selected: Option<String>,
    /// Todos los .jar de primer nivel.
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Prop {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub file: String,
    pub name: String,
    pub enabled: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldInfo {
    pub name: String,
    pub size: u64,
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub dir: String,
    pub exists: bool,
    pub jar: JarInfo,
    /// `None` = no existe eula.txt; `Some(false)` = existe sin aceptar.
    pub eula: Option<bool>,
    pub has_properties: bool,
    pub plugins: usize,
    pub worlds: usize,
}

impl ServerDir {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn ensure(&self) -> AppResult<()> {
        std::fs::create_dir_all(self.dir.join("plugins"))?;
        Ok(())
    }

    pub fn jar(&self) -> JarInfo {
        let mut candidates: Vec<String> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_file())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.to_lowercase().ends_with(".jar"))
                    .collect()
            })
            .unwrap_or_default();
        candidates.sort();
        let selected = if candidates.iter().any(|c| c == DEFAULT_JAR) {
            Some(DEFAULT_JAR.to_string())
        } else if candidates.len() == 1 {
            Some(candidates[0].clone())
        } else {
            None
        };
        JarInfo { selected, candidates }
    }

    pub fn eula(&self) -> Option<bool> {
        let s = std::fs::read_to_string(self.dir.join("eula.txt")).ok()?;
        Some(
            s.lines()
                .map(str::trim)
                .any(|l| l.eq_ignore_ascii_case("eula=true")),
        )
    }

    /// Escribe `eula=true`. La UI solo debe llamarlo tras una accion explicita del usuario.
    pub fn accept_eula(&self) -> AppResult<()> {
        std::fs::create_dir_all(&self.dir)?;
        write_atomic(
            &self.dir.join("eula.txt"),
            b"# Aceptado por el usuario desde DrakeStar. https://aka.ms/MinecraftEULA\neula=true\n",
        )?;
        Ok(())
    }

    pub fn read_properties(&self) -> Vec<Prop> {
        let s = std::fs::read_to_string(self.dir.join("server.properties")).unwrap_or_default();
        parse_properties(&s)
    }

    /// Aplica `updates` preservando comentarios y orden; las claves nuevas se anaden al final.
    pub fn write_properties(&self, updates: &BTreeMap<String, String>) -> AppResult<()> {
        for (k, v) in updates {
            if k.is_empty() || k.contains(['=', '\n', '\r', '#', ' ']) {
                return Err(AppError::Invalid(format!("clave de server.properties invalida: '{k}'")));
            }
            if v.contains(['\n', '\r']) {
                return Err(AppError::Invalid(format!("valor invalido para '{k}': sin saltos de linea")));
            }
        }
        let path = self.dir.join("server.properties");
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        write_atomic(&path, apply_properties(&old, updates).as_bytes())?;
        Ok(())
    }

    fn plugins_dir(&self) -> PathBuf {
        self.dir.join("plugins")
    }

    pub fn plugins(&self) -> Vec<PluginInfo> {
        let mut v: Vec<PluginInfo> = std::fs::read_dir(self.plugins_dir())
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_file())
                    .filter_map(|e| {
                        let file = e.file_name().to_string_lossy().to_string();
                        let (name, enabled) = if let Some(n) = file.strip_suffix(".jar") {
                            (n.to_string(), true)
                        } else if let Some(n) = file.strip_suffix(".jar.disabled") {
                            (n.to_string(), false)
                        } else {
                            return None;
                        };
                        Some(PluginInfo { file, name, enabled, size: e.metadata().map(|m| m.len()).unwrap_or(0) })
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    /// Activa/desactiva renombrando `x.jar` <-> `x.jar.disabled`. Devuelve el nuevo nombre.
    pub fn toggle_plugin(&self, file: &str) -> AppResult<String> {
        if !safe_file_name(file) {
            return Err(AppError::Invalid("nombre de plugin invalido".into()));
        }
        let from = self.plugins_dir().join(file);
        if !from.is_file() {
            return Err(AppError::NotFound(format!("plugin '{file}'")));
        }
        let new = if let Some(n) = file.strip_suffix(".jar") {
            format!("{n}.jar.disabled")
        } else if let Some(n) = file.strip_suffix(".jar.disabled") {
            format!("{n}.jar")
        } else {
            return Err(AppError::Invalid("no es un plugin .jar".into()));
        };
        let to = self.plugins_dir().join(&new);
        if to.exists() {
            return Err(AppError::Exists(format!("'{new}'")));
        }
        std::fs::rename(from, to)?;
        Ok(new)
    }

    pub fn worlds(&self) -> Vec<WorldInfo> {
        let mut v: Vec<WorldInfo> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_dir() && e.path().join("level.dat").is_file())
                    .map(|e| WorldInfo {
                        name: e.file_name().to_string_lossy().to_string(),
                        size: dir_size(&e.path()),
                        modified: modified_ms(&e.path()),
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn status(&self) -> ServerStatus {
        ServerStatus {
            dir: self.dir.to_string_lossy().to_string(),
            exists: self.dir.is_dir(),
            jar: self.jar(),
            eula: self.eula(),
            has_properties: self.dir.join("server.properties").is_file(),
            plugins: self.plugins().iter().filter(|p| p.enabled).count(),
            worlds: self.worlds().len(),
        }
    }

    /// Crea una copia de seguridad completa de un mundo (`level.dat`) dentro de `dest_dir/<world>-backup-<ts>`.
    pub fn backup_world(&self, world_name: &str, dest_dir: &Path) -> AppResult<PathBuf> {
        if !safe_file_name(world_name) {
            return Err(AppError::Invalid("nombre de mundo invalido".into()));
        }
        let src = self.dir.join(world_name);
        if !src.join("level.dat").is_file() {
            return Err(AppError::NotFound(format!("mundo '{world_name}' (sin level.dat)")));
        }
        std::fs::create_dir_all(dest_dir)?;
        let dst = dest_dir.join(format!("{world_name}-backup-{}", crate::now_ms()));
        copy_dir_recursive(&src, &dst)?;
        Ok(dst)
    }

    /// Instala un preset de ecosistema Slimefun (JAR principal + suites/addons opcionales) en `plugins/`.
    pub fn install_slimefun_preset(&self, slimefun_jar: &Path, extra_plugins: &[PathBuf]) -> AppResult<Vec<String>> {
        if !slimefun_jar.is_file() {
            return Err(AppError::NotFound(format!("Slimefun JAR '{}'", slimefun_jar.display())));
        }
        self.ensure()?;
        let pdir = self.plugins_dir();
        let mut installed = Vec::new();
        std::fs::copy(slimefun_jar, pdir.join("Slimefun.jar"))?;
        installed.push("Slimefun.jar".to_string());
        for extra in extra_plugins {
            if !extra.is_file() {
                return Err(AppError::NotFound(format!("plugin extra '{}'", extra.display())));
            }
            let fname = extra
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| AppError::Invalid("nombre de archivo de plugin invalido".into()))?;
            if !safe_file_name(fname) || !fname.to_lowercase().ends_with(".jar") {
                return Err(AppError::Invalid(format!("nombre de plugin invalido: '{fname}'")));
            }
            std::fs::copy(extra, pdir.join(fname))?;
            installed.push(fname.to_string());
        }
        Ok(installed)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

pub fn parse_properties(s: &str) -> Vec<Prop> {
    s.lines()
        .filter_map(|l| {
            let t = l.trim_start();
            if t.is_empty() || t.starts_with('#') {
                return None;
            }
            let (k, v) = t.split_once('=')?;
            Some(Prop { key: k.trim().to_string(), value: v.to_string() })
        })
        .collect()
}

pub fn apply_properties(old: &str, updates: &BTreeMap<String, String>) -> String {
    let mut done: Vec<&String> = vec![];
    let mut out: Vec<String> = old
        .lines()
        .map(|l| {
            let t = l.trim_start();
            if !t.starts_with('#') {
                if let Some((k, _)) = t.split_once('=') {
                    if let Some((uk, uv)) = updates.get_key_value(k.trim()) {
                        done.push(uk);
                        return format!("{uk}={uv}");
                    }
                }
            }
            l.to_string()
        })
        .collect();
    for (k, v) in updates {
        if !done.contains(&k) {
            out.push(format!("{k}={v}"));
        }
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

pub fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                total += dir_size(&e.path());
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

pub fn modified_ms(p: &Path) -> Option<u64> {
    p.metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_preserve_comments_and_order() {
        let old = "#Minecraft server properties\nmotd=Hola\nserver-port=25565\n# fin\nonline-mode=true\n";
        let mut up = BTreeMap::new();
        up.insert("motd".to_string(), "DrakeStar Dev".to_string());
        up.insert("max-players".to_string(), "8".to_string());
        let out = apply_properties(old, &up);
        assert_eq!(
            out,
            "#Minecraft server properties\nmotd=DrakeStar Dev\nserver-port=25565\n# fin\nonline-mode=true\nmax-players=8\n"
        );
        let props = parse_properties(&out);
        assert_eq!(props.len(), 4);
        assert_eq!(props[0], Prop { key: "motd".into(), value: "DrakeStar Dev".into() });
    }

    #[test]
    fn properties_reject_injection() {
        let s = ServerDir::new(crate::testutil::tmpdir("props"));
        let mut up = BTreeMap::new();
        up.insert("motd".to_string(), "a\nop=true".to_string());
        assert!(s.write_properties(&up).is_err());
        let mut up = BTreeMap::new();
        up.insert("a=b".to_string(), "x".to_string());
        assert!(s.write_properties(&up).is_err());
        let mut ok = BTreeMap::new();
        ok.insert("server-port".to_string(), "25570".to_string());
        s.write_properties(&ok).unwrap();
        assert_eq!(s.read_properties(), vec![Prop { key: "server-port".into(), value: "25570".into() }]);
    }

    #[test]
    fn eula_flow() {
        let s = ServerDir::new(crate::testutil::tmpdir("eula"));
        assert_eq!(s.eula(), None);
        std::fs::write(s.dir.join("eula.txt"), "#x\neula=false\n").unwrap();
        assert_eq!(s.eula(), Some(false));
        s.accept_eula().unwrap();
        assert_eq!(s.eula(), Some(true));
    }

    #[test]
    fn jar_selection() {
        let s = ServerDir::new(crate::testutil::tmpdir("jar"));
        assert_eq!(s.jar().selected, None);
        std::fs::write(s.dir.join("paper-1.jar"), b"x").unwrap();
        assert_eq!(s.jar().selected.as_deref(), Some("paper-1.jar"));
        std::fs::write(s.dir.join("other.jar"), b"x").unwrap();
        assert_eq!(s.jar().selected, None); // ambiguo: no se adivina
        assert_eq!(s.jar().candidates.len(), 2);
        std::fs::write(s.dir.join(DEFAULT_JAR), b"x").unwrap();
        assert_eq!(s.jar().selected.as_deref(), Some(DEFAULT_JAR));
    }

    #[test]
    fn plugins_list_and_toggle() {
        let s = ServerDir::new(crate::testutil::tmpdir("plug"));
        s.ensure().unwrap();
        std::fs::write(s.dir.join("plugins/Slimefun.jar"), b"abc").unwrap();
        std::fs::write(s.dir.join("plugins/Old.jar.disabled"), b"abc").unwrap();
        std::fs::write(s.dir.join("plugins/readme.txt"), b"abc").unwrap();
        let p = s.plugins();
        assert_eq!(p.len(), 2);
        assert!(p.iter().find(|x| x.name == "Slimefun").unwrap().enabled);
        assert!(!p.iter().find(|x| x.name == "Old").unwrap().enabled);
        assert_eq!(s.toggle_plugin("Slimefun.jar").unwrap(), "Slimefun.jar.disabled");
        assert_eq!(s.toggle_plugin("Slimefun.jar.disabled").unwrap(), "Slimefun.jar");
        assert!(s.toggle_plugin("../x.jar").is_err());
        assert!(s.toggle_plugin("readme.txt").is_err());
        assert!(matches!(s.toggle_plugin("nada.jar"), Err(AppError::NotFound(_))));
    }

    #[test]
    fn worlds_need_level_dat() {
        let s = ServerDir::new(crate::testutil::tmpdir("worlds"));
        std::fs::create_dir_all(s.dir.join("world")).unwrap();
        std::fs::write(s.dir.join("world/level.dat"), b"12345").unwrap();
        std::fs::create_dir_all(s.dir.join("plugins")).unwrap();
        let w = s.worlds();
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].name, "world");
        assert_eq!(w[0].size, 5);
        let st = s.status();
        assert!(st.exists && st.worlds == 1 && st.eula.is_none());
    }

    #[test]
    fn world_backup_and_slimefun_preset_install() {
        let root = crate::testutil::tmpdir("preset_backup");
        let s = ServerDir::new(root.join("srv"));
        s.ensure().unwrap();
        std::fs::create_dir_all(s.dir.join("world/region")).unwrap();
        std::fs::write(s.dir.join("world/level.dat"), b"nbt-root").unwrap();
        std::fs::write(s.dir.join("world/region/r.0.0.mca"), b"chunk-0-0").unwrap();

        let bdir = s.backup_world("world", &root.join("backups")).unwrap();
        assert!(bdir.join("level.dat").is_file());
        assert!(bdir.join("region/r.0.0.mca").is_file());
        assert!(s.backup_world("../world", &root.join("backups")).is_err());

        let sf_src = root.join("Slimefun-Universal.jar");
        let suite_src = root.join("DrakesCore.jar");
        std::fs::write(&sf_src, b"sf-bytes").unwrap();
        std::fs::write(&suite_src, b"suite-bytes").unwrap();
        let installed = s.install_slimefun_preset(&sf_src, &[suite_src]).unwrap();
        assert_eq!(installed, vec!["Slimefun.jar", "DrakesCore.jar"]);
        assert_eq!(s.plugins().len(), 2);
    }
}
