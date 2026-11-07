use std::path::{Path, PathBuf};

/// Disposicion en disco bajo el directorio de datos de la app:
/// ```text
/// <root>/settings.json
/// <root>/manifest.json            (manifiesto de releases importado)
/// <root>/exports/<id>.json
/// <root>/profiles/<id>/profile.json
/// <root>/profiles/<id>/server/    (local-server: cwd del JAR)
/// <root>/profiles/<id>/instance/  (clientes: directorio de juego)
/// <root>/profiles/<id>/logs/      (logs del launcher para el proceso)
/// ```
/// Los ids de perfil estan validados por `drakestar-core` (`[a-z0-9_-]`), por lo
/// que no pueden escapar del directorio `profiles/`.
#[derive(Debug, Clone)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }
    pub fn profile_dir(&self, id: &str) -> PathBuf {
        self.profiles_dir().join(id)
    }
    pub fn profile_json(&self, id: &str) -> PathBuf {
        self.profile_dir(id).join("profile.json")
    }
    pub fn server_dir(&self, id: &str) -> PathBuf {
        self.profile_dir(id).join("server")
    }
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.profile_dir(id).join("instance")
    }
    pub fn logs_dir(&self, id: &str) -> PathBuf {
        self.profile_dir(id).join("logs")
    }
    pub fn exports_dir(&self) -> PathBuf {
        self.root.join("exports")
    }
    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }
}

/// Escritura atomica: archivo temporal + rename.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// Un nombre de archivo simple: sin separadores, sin `..`, no vacio.
pub fn safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && !name.contains(['/', '\\', '\0'])
        && name != "."
        && name != ".."
}
