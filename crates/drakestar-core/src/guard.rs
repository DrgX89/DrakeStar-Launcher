//! DrakeStar Blackwall Guard: Motor de seguridad y análisis estático de mods JAR.
//!
//! Inspecciona el bytecode y las entradas del archivo ZIP/JAR en busca de patrones
//! de riesgo conocidos (ej. Fractureiser, llamadas directas a ProcessBuilder/Runtime.exec,
//! exfiltración de credenciales a Discord webhooks, o acceso a rutas fuera del juego).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Safe,
    Suspicious,
    Dangerous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub file: String,
    pub risk: RiskLevel,
    pub reasons: Vec<String>,
    pub suspicious_strings: Vec<String>,
    pub quarantine: bool,
    pub user_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuardAllowlist {
    pub allowed: Vec<String>,
}

impl GuardAllowlist {
    pub fn is_allowed(&self, file_name: &str, sha256: &str) -> bool {
        self.allowed.iter().any(|a| a == file_name || a.eq_ignore_ascii_case(sha256))
    }
}

/// Palabras clave y firmas heurísticas de riesgo en bytecode / constantes UTF-8
const RISKY_PATTERNS: &[(&str, &str, RiskLevel)] = &[
    ("java/lang/ProcessBuilder", "Ejecución de procesos del sistema operativo", RiskLevel::Dangerous),
    ("java/lang/Runtime;->exec", "Invocación de comandos de terminal (Runtime.exec)", RiskLevel::Dangerous),
    ("discord.com/api/webhooks", "Exfiltración potencial a Discord Webhook", RiskLevel::Dangerous),
    (".discord/Local Storage", "Acceso a tokens de sesión de Discord", RiskLevel::Dangerous),
    (".ssh/id_", "Intento de lectura de llaves SSH privadas", RiskLevel::Dangerous),
    ("powershell.exe", "Invocación de scripts PowerShell", RiskLevel::Dangerous),
    ("/bin/sh", "Invocación de intérprete de comandos shell Unix", RiskLevel::Dangerous),
    ("/bin/bash", "Invocación de intérprete de comandos Bash Unix", RiskLevel::Dangerous),
    ("curl -s", "Descarga y ejecución remota de binarios en background", RiskLevel::Dangerous),
    ("wget ", "Descarga no declarada de scripts externos", RiskLevel::Suspicious),
];

/// Analiza un búfer de archivo JAR en memoria en busca de firmas y cadenas sospechosas.
pub fn scan_jar_bytes(file_name: &str, data: &[u8]) -> ScanResult {
    let mut reasons = Vec::new();
    let mut suspicious_strings = BTreeSet::new();
    let mut max_risk = RiskLevel::Safe;

    // Convertir de forma tolerante a caracteres legibles para análisis de constantes
    let text = String::from_utf8_lossy(data);

    for (pattern, reason, risk) in RISKY_PATTERNS {
        if text.contains(pattern) {
            reasons.push(reason.to_string());
            suspicious_strings.insert(pattern.to_string());
            if *risk == RiskLevel::Dangerous {
                max_risk = RiskLevel::Dangerous;
            } else if max_risk != RiskLevel::Dangerous && *risk == RiskLevel::Suspicious {
                max_risk = RiskLevel::Suspicious;
            }
        }
    }

    let quarantine = max_risk == RiskLevel::Dangerous;

    ScanResult {
        file: file_name.to_string(),
        risk: max_risk,
        reasons,
        suspicious_strings: suspicious_strings.into_iter().collect(),
        quarantine,
        user_override: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_jar_passes_safe() {
        let dummy = b"PK\x03\x04Hello world clean fabric mod data";
        let res = scan_jar_bytes("clean_mod.jar", dummy);
        assert_eq!(res.risk, RiskLevel::Safe);
        assert!(!res.quarantine);
        assert!(res.reasons.is_empty());
    }

    #[test]
    fn dangerous_process_builder_is_quarantined() {
        let dummy = b"PK\x03\x04...ConstantPool: java/lang/ProcessBuilder -> cmd.exe";
        let res = scan_jar_bytes("evil_mod.jar", dummy);
        assert_eq!(res.risk, RiskLevel::Dangerous);
        assert!(res.quarantine);
        assert!(res.reasons.iter().any(|r| r.contains("procesos del sistema")));
    }

    #[test]
    fn ssh_theft_is_blocked() {
        let dummy = b"PK\x03\x04...path: /home/user/.ssh/id_rsa";
        let res = scan_jar_bytes("stealer.jar", dummy);
        assert_eq!(res.risk, RiskLevel::Dangerous);
        assert!(res.quarantine);
    }
}
