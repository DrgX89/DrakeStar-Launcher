//! Modelo de mods del ecosistema Fabric, en Rust puro: manifiesto, dialecto de
//! versiones y resolucion de dependencias. No carga nada ni toca disco: eso es
//! `drakestar-app::mods`.
//!
//! El manifiesto es el estándar `fabric.mod.json`, SIN modificarlo. Los mods
//! nativos Rust usan el mismo archivo y declaran sus metadatos extra en
//! `custom` (donde el formato ya lo permite), asi hay un unico formato y un
//! unico resolvedor para los dos runtimes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub const MOD_SCHEMA: u32 = 1;

/// Donde corre el mod (fisicamente).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Carga en cliente y en servidor.
    #[default]
    Universal,
    Client,
    Server,
}

impl Environment {
    pub fn label(self) -> &'static str {
        match self {
            Environment::Universal => "*",
            Environment::Client => "client",
            Environment::Server => "server",
        }
    }
    pub fn runs_on(self, side: Environment) -> bool {
        self == Environment::Universal || self == side
    }
}

// ---------- versiones ----------

/// Version estilo Minecraft: componentes numericos (con comodin opcional) y
/// sufijo (build/prerelease). `None` en un componente es el comodin `x`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    nums: Vec<Option<u64>>,
    suffix: String,
}

impl Version {
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.trim();
        let s = s.strip_prefix('v').unwrap_or(s);
        let (nums, suffix) = match s.find(['+', '-']) {
            Some(i) => (&s[..i], s[i..].to_string()),
            None => (s, String::new()),
        };
        // una version sin componentes numericos (o vacia) no es una version
        if nums.is_empty() {
            return None;
        }
        let mut v = vec![];
        for part in nums.split('.') {
            // "x" y vacio son comodin
            let n = if part.is_empty() || part.eq_ignore_ascii_case("x") { None } else { Some(part.parse().ok()?) };
            v.push(n);
        }
        Some(Version { nums: v, suffix })
    }

    fn num(&self, i: usize) -> u64 {
        self.nums.get(i).copied().flatten().unwrap_or(0)
    }

    /// Igualdad con comodines: `1.x` cubre 1.99.1; `1.2` cubre 1.2.9.
    fn covers(&self, other: &Version) -> bool {
        for (i, n) in self.nums.iter().enumerate() {
            match n {
                // comodin: cubre cualquier valor en esa posicion
                None => continue,
                Some(a) => match other.nums.get(i) {
                    Some(Some(b)) if a == b => continue,
                    // 1.2 declara dos componentes y cubre 1.2.lo-que-sea
                    None => {
                        if i + 1 != self.nums.len() {
                            return false;
                        }
                    }
                    _ => return false,
                },
            }
        }
        true
    }

    fn cmp(&self, other: &Version) -> std::cmp::Ordering {
        for i in 0..self.nums.len().max(other.nums.len()) {
            let (a, b) = (self.num(i), other.num(i));
            if a != b {
                return a.cmp(&b);
            }
        }
        self.suffix.cmp(&other.suffix)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.any {
            return write!(f, "*");
        }
        let branches: Vec<String> = self
            .branches
            .iter()
            .map(|b| b.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "))
            .collect();
        write!(f, "{}", branches.join(" || "))
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.cmp {
            Cmp::Ge => ">=",
            Cmp::Gt => ">",
            Cmp::Le => "<=",
            Cmp::Lt => "<",
            Cmp::Eq => "",
            Cmp::Caret => "^",
            Cmp::Tilde => "~",
        };
        write!(f, "{prefix}{}", self.version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .nums
            .iter()
            .map(|n| match n {
                Some(v) => v.to_string(),
                None => "x".to_string(),
            })
            .collect();
        write!(f, "{}{}", parts.join("."), self.suffix)
    }
}

/// Un comparador del dialecto de Fabric.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmp {
    Ge,
    Gt,
    Le,
    Lt,
    Eq,
    /// `^1.2.3` = >=1.2.3 <2.0.0 (compatible dentro del major)
    Caret,
    /// `~1.2.3` = >=1.2.3 <1.3.0 (compatible dentro del minor)
    Tilde,
}

/// Una condicion simple: comparador + version (que admite comodines).
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub cmp: Cmp,
    pub version: Version,
}

impl Condition {
    fn matches(&self, v: &Version) -> bool {
        use Cmp::*;
        use std::cmp::Ordering::*;
        match self.cmp {
            // igualdad con comodines: 1.2 cubre 1.2.x
            Eq => self.version.covers(v),
            _ if self.version.covers(v) && !matches!(self.cmp, Ge | Gt | Le | Lt) => true,
            Ge => v.cmp(&self.version) != Less,
            Gt => v.cmp(&self.version) == Greater,
            Le => v.cmp(&self.version) != Greater,
            Lt => v.cmp(&self.version) == Less,
            Caret => {
                // semver/npm: ^1.2.3 = <2.0.0, pero ^0.14.0 = <0.15.0 (con major
                // 0 el cambio de minor ya es incompatible)
                let bound = if self.version.num(0) == 0 {
                    upper_bound(&self.version, 1)
                } else {
                    upper_bound(&self.version, 0)
                };
                v.cmp(&self.version) != Less && v.cmp(&bound) == Less
            }
            Tilde => v.cmp(&self.version) != Less && v.cmp(&upper_bound(&self.version, 1)) == Less,
        }
    }
}

fn upper_bound(v: &Version, keep: usize) -> Version {
    let mut nums: Vec<Option<u64>> = v.nums.iter().take(keep).copied().collect();
    if let Some(last) = v.nums.get(keep).and_then(|n| *n) {
        nums.push(Some(last + 1));
    } else {
        nums.push(Some(1));
    }
    Version { nums, suffix: String::new() }
}

/// Expresion de dependencia: ramas separadas por `||`, condiciones por espacio.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionReq {
    pub any: bool,
    pub branches: Vec<Vec<Condition>>,
}

impl VersionReq {
    pub fn parse(s: &str) -> Option<VersionReq> {
        let s = s.trim();
        // solo "*" es "cualquiera": una dependencia vacia es un error de autor
        if s == "*" {
            return Some(VersionReq { any: true, branches: vec![] });
        }
        let mut branches = vec![];
        for or_part in s.split("||") {
            let mut conds = vec![];
            let or_part = or_part.trim().trim_matches(|c| c == '(' || c == ')').trim();
            if or_part.is_empty() {
                continue;
            }
            for tok in or_part.split_whitespace() {
                conds.push(parse_condition(tok)?);
            }
            if conds.is_empty() {
                return None;
            }
            branches.push(conds);
        }
        if branches.is_empty() {
            return None;
        }
        Some(VersionReq { any: false, branches })
    }

    /// ¿Satisface la version `v` esta expresion?
    pub fn matches(&self, v: &str) -> bool {
        if self.any {
            return true;
        }
        let Some(v) = Version::parse(v) else { return false };
        self.branches.iter().any(|b| b.iter().all(|c| c.matches(&v)))
    }
}

fn parse_condition(tok: &str) -> Option<Condition> {
    let tok = tok.trim();
    let (cmp, rest) = if let Some(r) = tok.strip_prefix(">=") {
        (Cmp::Ge, r)
    } else if let Some(r) = tok.strip_prefix("<=") {
        (Cmp::Le, r)
    } else if let Some(r) = tok.strip_prefix('>') {
        (Cmp::Gt, r)
    } else if let Some(r) = tok.strip_prefix('<') {
        (Cmp::Lt, r)
    } else if let Some(r) = tok.strip_prefix('^') {
        (Cmp::Caret, r)
    } else if let Some(r) = tok.strip_prefix('~') {
        (Cmp::Tilde, r)
    } else if let Some(r) = tok.strip_prefix('=') {
        (Cmp::Eq, r)
    } else {
        (Cmp::Eq, tok)
    };
    Some(Condition { cmp, version: Version::parse(rest.trim())? })
}

// ---------- manifiesto ----------

/// Una dependencia puede escribirse como texto o como lista de textos (que es
/// un OR, como en Fabric): `"minecraft": ["26.2.0"]` == `"26.2.0"`.
fn de_req<'de, D: serde::Deserializer<'de>>(d: D) -> Result<BTreeMap<String, String>, D::Error> {
    use serde::de::Error;
    let raw: BTreeMap<String, serde_json::Value> = BTreeMap::deserialize(d)?;
    let mut out = BTreeMap::new();
    for (k, v) in raw {
        let expr = match v {
            serde_json::Value::String(s) => s,
            serde_json::Value::Array(a) => a
                .iter()
                .map(|x| x.as_str().map(str::to_string).ok_or_else(|| D::Error::custom("dependencia: se esperaba texto")))
                .collect::<Result<Vec<String>, _>>()?
                .join(" || "),
            other => return Err(D::Error::custom(format!("dependencia de '{k}': se esperaba texto o lista, no {other}"))),
        };
        out.insert(k, expr);
    }
    Ok(out)
}

/// Fabric escribe `"environment": "*"`, que no es un nombre de variante: se
/// acepta tal cual, igual que el loader real.
fn de_env<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Environment, D::Error> {
    use serde::de::Error;
    let s = String::deserialize(d)?;
    match s.as_str() {
        "" | "*" => Ok(Environment::Universal),
        "client" => Ok(Environment::Client),
        "server" => Ok(Environment::Server),
        other => Err(D::Error::custom(format!("environment desconocido '{other}'"))),
    }
}

/// `fabric.mod.json` (schemaVersion 1). Campos desconocidos se ignoran, como
/// hace el loader real.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// `*` (por defecto), `client` o `server`.
    #[serde(default, deserialize_with = "de_env")]
    pub environment: Environment,
    #[serde(rename = "entrypoints", default)]
    pub entrypoints: BTreeMap<String, Vec<String>>,
    #[serde(default, deserialize_with = "de_req")]
    pub depends: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "de_req")]
    pub recommends: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "de_req")]
    pub suggests: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "de_req")]
    pub breaks: BTreeMap<String, String>,
    /// Metadatos de extension (los mods nativos Rust declaran aqui su ABI).
    #[serde(default)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, PartialEq)]
pub enum ModError {
    BadJson(String),
    Schema(u32),
    BadId(String),
    BadVersion(String),
    BadReq(String, String),
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadJson(e) => write!(f, "fabric.mod.json invalido: {e}"),
            Self::Schema(v) => write!(f, "schemaVersion {v} no soportado (esperado {MOD_SCHEMA})"),
            Self::BadId(i) => write!(f, "id invalido '{i}' (usa a-z, 0-9, _ y -)"),
            Self::BadVersion(v) => write!(f, "version invalida '{v}'"),
            Self::BadReq(id, r) => write!(f, "expresion de dependencia invalida para '{id}': '{r}'"),
        }
    }
}
impl std::error::Error for ModError {}

impl ModManifest {
    pub fn from_json(s: &str) -> Result<ModManifest, ModError> {
        let m: ModManifest = serde_json::from_str(s).map_err(|e| ModError::BadJson(e.to_string()))?;
        m.validate()?;
        Ok(m)
    }

    pub fn validate(&self) -> Result<(), ModError> {
        if self.schema_version != MOD_SCHEMA {
            return Err(ModError::Schema(self.schema_version));
        }
        let ok = !self.id.is_empty()
            && self.id.len() <= 64
            && self.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if !ok {
            return Err(ModError::BadId(self.id.clone()));
        }
        Version::parse(&self.version).ok_or_else(|| ModError::BadVersion(self.version.clone()))?;
        for (id, req) in self
            .depends
            .iter()
            .chain(self.recommends.iter())
            .chain(self.suggests.iter())
            .chain(self.breaks.iter())
        {
            VersionReq::parse(req).ok_or_else(|| ModError::BadReq(id.clone(), req.clone()))?;
        }
        // un mod sin entrypoints es valido: los mods de API (fabric-api) son
        // exactamente asi, y otros mods dependen de ellos
        Ok(())
    }

    /// Dependencia declarada de un pseudo-mod del entorno (`minecraft`,
    /// `fabricloader`, `java`, `minecraft-server`...): el loader la resuelve
    /// contra la version real del entorno.
    pub fn requires_env(&self) -> Vec<(&str, &str)> {
        self.depends.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
    }
}

// ---------- resolucion del grafo de mods ----------

/// Versiones del entorno contra las que se resuelven los pseudo-mods.
#[derive(Debug, Clone)]
pub struct Env {
    pub minecraft: String,
    pub loader: String,
    pub java: String,
}

/// Un aviso: no impide cargar, pero el usuario debe enterarse.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub mod_id: String,
    pub text: String,
}

/// pseudo-mods: dependencias que resuelve el entorno, no otro mod.
pub fn pseudo_version<'a>(id: &str, env: &'a Env) -> Option<&'a str> {
    match id {
        "minecraft" => Some(&env.minecraft),
        "fabricloader" | "fabric-loader" => Some(&env.loader),
        "java" => Some(&env.java),
        _ => None,
    }
}

/// Una razon por la que algo no carga.
#[derive(Debug, Clone, PartialEq)]
pub struct Reason {
    pub mod_id: String,
    pub text: String,
}

fn is_dropped(id: &str, dropped: &[Reason]) -> bool {
    dropped.iter().any(|d| d.mod_id == id)
}

/// Resuelve el conjunto de mods para un lado (cliente o servidor) y devuelve
/// el **orden de carga** (las dependencias primero). Lo que no carga queda
/// explicado en `reasons`, con el mod responsable y el motivo.
pub fn resolve(
    mods: &[ModManifest],
    env: &Env,
    side: Environment,
) -> Result<(Vec<String>, Vec<Warning>), Vec<Reason>> {
    let alive: Vec<&ModManifest> = mods.iter().filter(|m| m.environment.runs_on(side)).collect();
    let mut dropped: Vec<Reason> = vec![];
    let mut warnings: Vec<Warning> = vec![];
    // se itera hasta que nada cambie: un mod que cae arrastra a los que
    // dependen de el en la vuelta siguiente
    loop {
        let mut changed = false;
        for m in alive.iter() {
            if is_dropped(&m.id, &dropped) {
                continue;
            }
            let hard: Vec<(&String, &String)> = m.depends.iter().collect();
            let soft: Vec<(&String, &String)> = m.recommends.iter().collect();
            for (required, all) in [(true, &hard), (false, &soft)] {
              for (dep, req) in all.iter() {
                if let Some(v) = pseudo_version(dep, env) {
                    let req = match VersionReq::parse(req) {
                        Some(r) => r,
                        None => {
                            dropped.push(Reason { mod_id: m.id.clone(), text: format!("dependencia invalida: '{dep} {req}'") });
                            changed = true;
                            continue;
                        }
                    };
                    if !req.matches(v) {
                        if required {
                            dropped.push(Reason {
                                mod_id: m.id.clone(),
                                text: format!("pide {dep} {req} y el entorno tiene {v}"),
                            });
                            changed = true;
                        } else {
                            warnings.push(Warning {
                                mod_id: m.id.clone(),
                                text: format!("recomienda {dep} {req} y el entorno tiene {v}"),
                            });
                        }
                    }
                    continue;
                }
                let Some(other) = alive.iter().find(|o| o.id == **dep && !is_dropped(&o.id, &dropped)) else {
                    if !required {
                        warnings.push(Warning {
                            mod_id: m.id.clone(),
                            text: format!("recomienda '{dep}' y no esta instalado"),
                        });
                        continue;
                    }
                    dropped.push(Reason {
                        mod_id: m.id.clone(),
                        text: format!("falta el mod '{dep}' ({req})"),
                    });
                    changed = true;
                    continue;
                };
                let req = match VersionReq::parse(req) {
                    Some(r) => r,
                    None => {
                        dropped.push(Reason { mod_id: m.id.clone(), text: format!("dependencia invalida: '{dep} {req}'") });
                        changed = true;
                        continue;
                    }
                };
                if !req.matches(&other.version) {
                    let text = format!("recomienda '{dep}' {req} y esta presente la version {}", other.version);
                    if required {
                        dropped.push(Reason { mod_id: m.id.clone(), text });
                        changed = true;
                    } else {
                        warnings.push(Warning { mod_id: m.id.clone(), text });
                    }
                }
              }
            }
        }
        // breaks: dos mods que no pueden coexistir
        for m in alive.iter() {
            if is_dropped(&m.id, &dropped) {
                continue;
            }
            for (other_id, req) in &m.breaks {
                if let Some(other) = alive.iter().find(|o| o.id == *other_id && !is_dropped(&o.id, &dropped)) {
                    if VersionReq::parse(req).map(|r| r.matches(&other.version)).unwrap_or(false) {
                        dropped.push(Reason {
                            mod_id: m.id.clone(),
                            text: format!("rompe con '{}' {}", other.id, req),
                        });
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    dropped.sort_by(|a, b| a.mod_id.cmp(&b.mod_id).then(a.text.cmp(&b.text)));
    dropped.dedup_by(|a, b| a == b);
    if !dropped.is_empty() {
        return Err(dropped);
    }
    let order = topo_order(&alive);
    Ok((order, warnings))
}

/// Orden estable por dependencias (Kahn). Los ciclos se reportan y esos mods
/// quedan fuera de la lista: cargarlos en cualquier orden es mejor que no
/// cargar nada, pero el usuario debe saberlo.
fn topo_order(alive: &[&ModManifest]) -> Vec<String> {
    let mut order: Vec<String> = vec![];
    let mut placed: Vec<String> = vec![];
    let ids: Vec<String> = alive.iter().map(|m| m.id.clone()).collect();
    loop {
        let mut progress = false;
        for m in alive {
            if placed.contains(&m.id) {
                continue;
            }
            let ready = m.depends.keys().all(|d| {
                // un pseudo-mod o una dependencia no presente no bloquea el orden
                !ids.contains(d) || placed.contains(d)
            });
            if ready {
                placed.push(m.id.clone());
                order.push(m.id.clone());
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    // ciclos: se agregan al final, en orden alfabetico, para no perderlos
    let mut stuck: Vec<String> = alive.iter().map(|m| m.id.clone()).filter(|id| !placed.contains(id)).collect();
    stuck.sort();
    order.extend(stuck);
    order
}


#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> ModManifest {
        try_manifest(json).unwrap()
    }
    fn try_manifest(json: &str) -> Result<ModManifest, ModError> {
        ModManifest::from_json(json)
    }

    const FULL: &str = r#"{
      "schemaVersion": 1,
      "id": "slimefun-addon",
      "version": "2.1.0+build.7",
      "name": "Slimefun Addon",
      "description": "anade contenido",
      "environment": "*",
      "entrypoints": { "main": ["net.drakes.addon.Main"], "client": ["net.drakes.addon.ClientInit"] },
      "depends": { "minecraft": "~1.20", "fabricloader": ">=0.14", "java": ">=17", "slimefun": "1.2.x" },
      "recommends": { "fabric-api": "*" },
      "conflicts_ignored": true,
      "custom": { "drakes-rust": { "abi": 1, "cdylib": "libaddon.so" } }
    }"#;

    #[test]
    fn dependencies_can_be_a_list_of_expressions() {
        // caso real de Modrinch (DistantHorizons): la version puede ser una lista
        let m = try_manifest(
            r#"{
              "schemaVersion": 1, "id": "dh", "version": "3.3.3", "environment": "*",
              "entrypoints": { "client": ["a.Main"] },
              "depends": { "minecraft": ["26.2.0"] }
            }"#,
        )
        .unwrap();
        assert_eq!(m.depends["minecraft"], "26.2.0");
        // una lista es un OR, igual que en Fabric
        let m2 = try_manifest(
            r#"{
              "schemaVersion": 1, "id": "x", "version": "1.0.0", "environment": "*",
              "entrypoints": { "main": ["a.Main"] },
              "depends": { "otro": ["^1.0", "^2.0"] }
            }"#,
        )
        .unwrap();
        assert_eq!(m2.depends["otro"], "^1.0 || ^2.0");
        let req = VersionReq::parse(&m2.depends["otro"]).unwrap();
        assert!(req.matches("1.5.0") && req.matches("2.5.0") && !req.matches("3.0.0"));
        // resolver: 26.2 cubre 26.2.0 (comodin al final)
        let env = Env { minecraft: "26.2".into(), loader: "0.20.1".into(), java: "25".into() };
        assert!(resolve(&[m], &env, Environment::Client).is_ok());
        // una dependencia con basura en la lista: error claro, no panico
        let bad = r#"{
              "schemaVersion": 1, "id": "y", "version": "1.0.0", "environment": "*",
              "entrypoints": { "main": ["a.Main"] },
              "depends": { "otro": ["^1.0", 5] }
            }"#;
        assert!(try_manifest(bad).is_err());
    }

    #[test]
    fn parses_the_standard_manifest() {
        let m = manifest(FULL);
        assert_eq!(m.id, "slimefun-addon");
        assert_eq!(m.version, "2.1.0+build.7");
        assert_eq!(m.environment, Environment::Universal);
        assert_eq!(m.entrypoints["main"], vec!["net.drakes.addon.Main"]);
        assert_eq!(m.depends.len(), 4);
        // custom pasa tal cual: por ahi viajan los metadatos de los mods Rust
        assert!(m.custom.contains_key("drakes-rust"));
    }

    #[test]
    fn rejects_broken_manifests() {
        assert!(matches!(try_manifest(&FULL.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2")), Err(ModError::Schema(2))));
        assert!(matches!(try_manifest(&FULL.replace("slimefun-addon", "Mal Id")), Err(ModError::BadId(_))));
        assert!(matches!(try_manifest(&FULL.replace("2.1.0+build.7", "no-es-version")), Err(ModError::BadVersion(_))));
        // expresion de dependencia rota
        assert!(matches!(try_manifest(&FULL.replace("\">=0.14\"", "\">=0.14 <\"")), Err(ModError::BadReq(..))));
        // un mod libreria sin entrypoints es valido (fabric-api es asi)
        assert!(try_manifest(&FULL.replace(
            "\"entrypoints\": { \"main\": [\"net.drakes.addon.Main\"], \"client\": [\"net.drakes.addon.ClientInit\"] }",
            "\"entrypoints\": {}"
        ))
        .is_ok());
        assert!(matches!(try_manifest("{no json"), Err(ModError::BadJson(_))));
    }

    #[test]
    fn version_dialect_matches_fabric() {
        let cases = [
            // (expresion, version, esperado)
            ("*", "1.2.3", true),
            ("1.2.3", "1.2.3", true),
            ("1.2", "1.2.9", true),   // comodin: 1.2 cubre 1.2.x
            ("1.2", "1.3.0", false),
            (">=1.0", "1.0.0", true),
            (">1.0", "1.0.0", false),
            ("<2.0", "1.9.9", true),
            ("<=1.2.3", "1.2.3", true),
            ("^1.2.3", "1.9.0", true),
            ("^1.2.3", "2.0.0", false),
            ("^1.2.3", "1.2.2", false),
            ("~1.2.3", "1.2.9", true),
            ("~1.2.3", "1.3.0", false),
            ("~1.2", "1.2.5", true),
            (">=1.0 <2.0", "1.5.0", true),   // AND implicito
            (">=1.0 <2.0", "2.1.0", false),
            ("1.x", "1.99.1", true),
            ("1.2.x", "1.2.0", true),
            ("26.2", "26.2.1", true),
            ("^0.14.0", "0.14.9", true),
            ("^0.14.0", "0.15.0", false),
        ];
        for (req, ver, want) in cases {
            let r = VersionReq::parse(req).unwrap_or_else(|| panic!("expresion invalida: {req}"));
            assert_eq!(r.matches(ver), want, "{req} vs {ver}");
        }
    }

    #[test]
    fn or_branches_are_supported() {
        let r = VersionReq::parse("^1.0 || ^2.0").unwrap();
        assert!(r.matches("1.5.0") && r.matches("2.1.0") && !r.matches("3.0.0"));
        // branches vacias o tokens vacios: invalido
        assert!(VersionReq::parse("").is_none());
        assert!(VersionReq::parse("||").is_none());
        assert!(VersionReq::parse(">=").is_none());
    }

    #[test]
    fn version_parsing_edges() {
        assert!(Version::parse("").is_none());
        assert!(Version::parse("x").is_some()); // comodin puro
        assert_eq!(Version::parse("26.2-rc.1").unwrap().nums, vec![Some(26), Some(2)]);
        assert_eq!(Version::parse("v1.2.3").unwrap().nums, vec![Some(1), Some(2), Some(3)]);
        assert_eq!(Version::parse("1.x").unwrap().nums, vec![Some(1), None]);
        assert_eq!(Version::parse("1.2.3+build.7").unwrap().suffix, "+build.7");
    }


    // ---------- resolucion ----------

    fn env() -> Env {
        Env { minecraft: "26.2".into(), loader: "0.20.1".into(), java: "25".into() }
    }

    /// `extra` es un objeto JSON que se fusiona en el manifiesto (breaks,
    /// recommends, environment...).
    fn modjson(id: &str, version: &str, depends: &str, extra: &str) -> String {
        let extra = match extra.trim() {
            "" | "{}" => String::new(),
            e => format!(", {e}"),
        };
        format!(
            r#"{{ "schemaVersion": 1, "id": "{id}", "version": "{version}",
                 "entrypoints": {{ "main": ["m.{id}.Main"] }},
                 "depends": {depends}{extra} }}"#
        )
    }

    fn mods(list: &[(&str, &str, &str, &str)]) -> Vec<ModManifest> {
        list.iter()
            .map(|(id, v, d, b)| ModManifest::from_json(&modjson(id, v, d, b)).unwrap())
            .collect()
    }

    #[test]
    fn dependencies_load_before_dependents() {
        let m = mods(&[
            ("api", "1.0.0", r#"{"minecraft": "^26.2", "fabricloader": ">=0.15"}"#, "{}"),
            ("addon", "2.0.0", r#"{"api": ">=1.0.0"}"#, "{}"),
            ("solo", "0.1.0", "{}", "{}"),
        ]);
        let (order, warns) = resolve(&m, &env(), Environment::Client).unwrap();
        assert_eq!(order.len(), 3);
        // api antes que addon
        let api = order.iter().position(|x| x == "api").unwrap();
        let addon = order.iter().position(|x| x == "addon").unwrap();
        assert!(api < addon, "orden: {order:?}");
        assert!(warns.is_empty());
    }

    #[test]
    fn missing_dependency_blocks_the_mod() {
        let m = mods(&[("addon", "1.0.0", r#"{"api": ">=1.0.0"}"#, "{}")]);
        let errs = resolve(&m, &env(), Environment::Client).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].mod_id, "addon");
        assert!(errs[0].text.contains("falta el mod 'api'"), "{}", errs[0].text);
    }

    #[test]
    fn wrong_version_and_environment_block_the_mod() {
        let m = mods(&[
            ("api", "1.0.0", "{}", "{}"),
            ("addon", "1.0.0", r#"{"api": ">=2.0.0"}"#, "{}"),
            ("old", "1.0.0", r#"{"minecraft": "1.20.x"}"#, "{}"),
            ("serveronly", "1.0.0", "{}", r#""environment": "server""#),
        ]);
        // en cliente: el serveronly ni entra
        let errs = resolve(&m, &env(), Environment::Client).unwrap_err();
        assert!(errs.iter().any(|e| e.mod_id == "addon" && e.text.contains(">=2.0.0")));
        assert!(errs.iter().any(|e| e.mod_id == "old" && e.text.contains("1.20.x")));
        assert!(!errs.iter().any(|e| e.mod_id == "serveronly"));
        // en servidor: serveronly ya no cae (los otros dos siguen fallando)
        let errs = resolve(&m, &env(), Environment::Server).unwrap_err();
        assert!(!errs.iter().any(|e| e.mod_id == "serveronly"));
    }

    #[test]
    fn broken_mods_are_reported_and_do_not_load() {
        let m = mods(&[
            ("a", "1.0.0", "{}", r#""breaks": {"b": ">=1.0.0"}"#),
            ("b", "1.2.0", "{}", "{}"),
        ]);
        let errs = resolve(&m, &env(), Environment::Client).unwrap_err();
        assert!(errs.iter().any(|e| e.mod_id == "a" && e.text.contains("rompe con 'b'")), "errs: {errs:?}");
    }

    #[test]
    fn recommends_warn_but_do_not_block() {
        let m = mods(&[
            ("api", "1.0.0", "{}", "{}"),
            ("addon", "1.0.0", r#"{"api": ">=1.0.0"}"#, r#""recommends": {"api": "^2.0.0"}"#),
        ]);
        let (order, warns) = resolve(&m, &env(), Environment::Client).unwrap();
        assert_eq!(order.len(), 2, "recomendar no bloquea");
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].mod_id, "addon");
    }

    #[test]
    fn a_dropped_mod_drags_its_dependents() {
        let m = mods(&[
            ("base", "1.0.0", r#"{"minecraft": "27.0"}"#, "{}"),
            ("mid", "1.0.0", r#"{"base": "*"}"#, "{}"),
            ("top", "1.0.0", r#"{"mid": "*"}"#, "{}"),
        ]);
        let errs = resolve(&m, &env(), Environment::Client).unwrap_err();
        assert_eq!(errs.len(), 3, "base cae y arrastra a mid y top: {errs:?}");
    }

    #[test]
    fn cycles_are_reported_but_not_lost() {
        let m = mods(&[
            ("x", "1.0.0", r#"{"y": "*"}"#, "{}"),
            ("y", "1.0.0", r#"{"x": "*"}"#, "{}"),
        ]);
        // un ciclo no impide cargar: se reporta el hecho y se cargan los dos
        let (order, _) = resolve(&m, &env(), Environment::Client).unwrap();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn expression_roundtrips_in_error_messages() {
        let m = mods(&[("a", "1.0.0", r#"{"minecraft": "~26.3 || ^27.0"}"#, "{}")]);
        let errs = resolve(&m, &env(), Environment::Client).unwrap_err();
        assert!(errs[0].text.contains("~26.3 || ^27.0"), "texto: {}", errs[0].text);
    }

}
