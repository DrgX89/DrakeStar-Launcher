//! Integración con Modrinth API v2: búsqueda e instalación 1-click de mods Fabric.

use crate::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthSearchHit>,
    pub total_hits: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthVersionFile>,
}

/// Busca mods en Modrinth compatibles con Fabric
pub fn search_mods(query: &str, limit: usize) -> AppResult<Vec<ModrinthSearchHit>> {
    let q = urlencode_simple(query);
    let url = format!(
        "https://api.modrinth.com/v2/search?query={q}&facets=[[\"categories:fabric\"],[\"project_type:mod\"]]&limit={limit}"
    );
    let json_text = drakestar_net::fetch_text(&url).map_err(AppError::from)?;
    let res: ModrinthSearchResult = serde_json::from_str(&json_text)
        .map_err(|e| AppError::Invalid(format!("Error decodificando respuesta de Modrinth: {e}")))?;
    Ok(res.hits)
}

/// Obtiene las versiones de un mod en Modrinth
pub fn get_project_versions(project_id_or_slug: &str, mc_version: Option<&str>) -> AppResult<Vec<ModrinthVersion>> {
    let mut url = format!("https://api.modrinth.com/v2/project/{project_id_or_slug}/version?loaders=[\"fabric\"]");
    if let Some(v) = mc_version {
        url.push_str(&format!("&game_versions=[\"{v}\"]"));
    }
    let json_text = drakestar_net::fetch_text(&url).map_err(AppError::from)?;
    let versions: Vec<ModrinthVersion> = serde_json::from_str(&json_text)
        .map_err(|e| AppError::Invalid(format!("Error decodificando versiones de Modrinth: {e}")))?;
    Ok(versions)
}

/// Instala un mod descargando su versión a la carpeta `mods_dir`
pub fn install_mod(mods_dir: &Path, file_url: &str, file_name: &str) -> AppResult<u64> {
    if !mods_dir.is_dir() {
        std::fs::create_dir_all(mods_dir)?;
    }
    let dest = mods_dir.join(file_name);
    drakestar_net::download_direct(file_url, &dest).map_err(AppError::from)
}

fn urlencode_simple(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoding_handles_spaces_and_symbols() {
        assert_eq!(urlencode_simple("sodium fabric"), "sodium+fabric");
        assert_eq!(urlencode_simple("iris&shaders"), "iris%26shaders");
    }
}
