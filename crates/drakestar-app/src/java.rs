use drakestar_core::launch::required_java;
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct JavaInfo {
    pub bin: String,
    pub found: bool,
    pub version: Option<String>,
    pub major: Option<u32>,
    pub error: Option<String>,
}

/// Ejecuta `<bin> -version` (la salida va a stderr) y extrae la version.
pub fn detect(bin: &str) -> JavaInfo {
    match Command::new(bin).arg("-version").output() {
        Ok(o) => {
            let text = String::from_utf8_lossy(if o.stderr.is_empty() { &o.stdout } else { &o.stderr }).to_string();
            let version = parse_version(&text);
            let major = version.as_deref().and_then(major_of);
            JavaInfo { bin: bin.into(), found: o.status.success() && version.is_some(), version, major, error: None }
        }
        Err(e) => JavaInfo { bin: bin.into(), found: false, version: None, major: None, error: Some(e.to_string()) },
    }
}

/// Busca `version "X"` en la primera linea que lo contenga.
pub fn parse_version(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let i = l.find("version \"")? + 9;
        let rest = &l[i..];
        Some(rest[..rest.find('"')?].to_string())
    })
}

/// "25.0.4.1" -> 25 ; "1.8.0_392" -> 8 ; "21" -> 21
pub fn major_of(v: &str) -> Option<u32> {
    let mut it = v.split(['.', '_', '-', '+']);
    let first: u32 = it.next()?.parse().ok()?;
    if first == 1 {
        it.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Aviso si el Java detectado no alcanza el requerido para la version de Minecraft.
pub fn compatibility_warning(info: &JavaInfo, minecraft: &str, profile_major: Option<u32>) -> Option<String> {
    let need = profile_major.unwrap_or_else(|| required_java(minecraft));
    if !info.found {
        return Some(format!("No se encontro Java ('{}'). Se requiere Java {need}.", info.bin));
    }
    match info.major {
        Some(m) if m < need => Some(format!("Java {m} detectado; Minecraft {minecraft} requiere Java {need}.")),
        None => Some("No se pudo determinar la version de Java.".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_versions() {
        let t = "Picked up X\nopenjdk version \"25.0.4.1\" 2026-08-18\nOpenJDK Runtime";
        assert_eq!(parse_version(t).as_deref(), Some("25.0.4.1"));
        assert_eq!(major_of("25.0.4.1"), Some(25));
        assert_eq!(major_of("1.8.0_392"), Some(8));
        assert_eq!(major_of("21"), Some(21));
        assert_eq!(major_of("17-ea"), Some(17));
        assert_eq!(parse_version("nada"), None);
    }
    #[test]
    fn warns_when_too_old_or_missing() {
        let mk = |found, major| JavaInfo { bin: "java".into(), found, version: None, major, error: None };
        assert!(compatibility_warning(&mk(true, Some(21)), "26.2", None).unwrap().contains("requiere Java 25"));
        assert!(compatibility_warning(&mk(true, Some(25)), "26.2", None).is_none());
        assert!(compatibility_warning(&mk(false, None), "26.2", None).is_some());
    }
    #[test]
    fn missing_binary_reports_error() {
        let i = detect("no-existe-java-drakestar");
        assert!(!i.found && i.error.is_some());
    }
}
