//! El **intermediario**: el ABI estable entre los mods y el motor.
//!
//! Vive en el cargador, no en el cliente: el motor solo publica una tabla de
//! hooks, y de adaptar modos al motor se encarga esta capa. Un mod escrito
//! contra el ABI 1 sigue cargando cuando el motor cambia, igual que
//! *intermediary* de Fabric permite que un mod compile una vez y corra en
//! cualquier entorno.
//!
//! Cubre los dos runtimes del loader: los mods nativos (cdylib, compilados
//! contra un ABI concreto) y los mods Java (classpath, que se verifican por
//! manifiesto: entrypoints, hooks y dependencias).

use crate::mods::ModManifest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Version del ABI que expone el motor. Major = contratos que rompen;
/// minor = hooks y campos nuevos (hacia atras compatibles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AbiVersion {
    pub major: u32,
    pub minor: u32,
}

impl AbiVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }
    /// `>=1.2` del dialecto de Fabric, para declarar el ABI pedido.
    pub fn parse(s: &str) -> Option<AbiVersion> {
        let mut nums = s.split('.').map(|p| p.parse::<u32>().ok());
        let major = nums.next()??;
        let minor = nums.next().flatten().unwrap_or(0);
        Some(AbiVersion { major, minor })
    }
    /// El mismo major siempre es compatible; el minor del motor debe alcanzar
    /// al que pide el mod.
    pub fn accepts(self, mod_abi: AbiVersion) -> Compat {
        if mod_abi.major != self.major {
            return if mod_abi.major > self.major { Compat::ModNewer } else { Compat::ModOlder };
        }
        if self.minor < mod_abi.minor {
            Compat::ModNewer
        } else {
            Compat::Ok
        }
    }
}

impl fmt::Display for AbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Resultado de comparar el ABI de un mod con el del motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compat {
    /// Carga sin problemas.
    Ok,
    /// El mod es mas viejo: carga, pero puede faltarle algo.
    ModOlder,
    /// El mod pide un ABI mas nuevo que el motor: no carga.
    ModNewer,
}

impl Compat {
    pub fn ok(self) -> bool {
        matches!(self, Compat::Ok | Compat::ModOlder)
    }
    pub fn label(self) -> &'static str {
        match self {
            Compat::Ok => "compatible",
            Compat::ModOlder => "compatible (ABI mas viejo)",
            Compat::ModNewer => "incompatible (necesita ABI mas nuevo)",
        }
    }
}

/// ABI que expone esta version del cargador. Va con el ABI C de
/// `drakestar_core::native`: son el mismo contrato.
pub const ABI: AbiVersion = AbiVersion { major: 2, minor: 0 };

/// Lado del juego donde corre un hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Client,
    Server,
    Both,
}

impl Side {
    pub fn matches(self, side: crate::mods::Environment) -> bool {
        use crate::mods::Environment as E;
        matches!(
            (self, side),
            (Side::Both, _) | (Side::Client, E::Client) | (Side::Server, E::Server)
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Side::Client => "cliente",
            Side::Server => "servidor",
            Side::Both => "ambos",
        }
    }
}

/// Identificador estable de un hook: `dominio:nombre` (p. ej. `client:tick`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HookId(pub String);

impl HookId {
    pub fn new(s: impl Into<String>) -> Option<HookId> {
        let s = s.into();
        // dominio:nombre, los dos en minusculas y guiones
        let (d, n) = s.split_once(':')?;
        let ok = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if ok(d) && ok(n) {
            Some(HookId(s))
        } else {
            None
        }
    }
    pub fn domain(&self) -> &str {
        self.0.split(':').next().unwrap_or("")
    }
    pub fn name(&self) -> &str {
        self.0.split_once(':').map(|(_, n)| n).unwrap_or("")
    }
}

impl fmt::Display for HookId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Definicion de un hook en el catalogo.
#[derive(Debug, Clone)]
pub struct HookDef {
    pub id: HookId,
    pub side: Side,
    /// ABI minimo que lo expone.
    pub since: AbiVersion,
    /// ABI en el que desaparecio (si alguna vez lo hace).
    pub removed_in: Option<AbiVersion>,
    pub desc: &'static str,
}

/// El catalogo de hooks del motor. Crece por versiones: agregar nunca rompe,
/// quitar si (por eso `removed_in`).
#[derive(Debug, Clone)]
pub struct Catalog {
    hooks: Vec<HookDef>,
}

impl Catalog {
    pub fn new() -> Self {
        Self { hooks: vec![] }
    }

    /// Catalogo del ABI 1.0: ciclo de vida, tick, y los puntos que el
    /// ecosistema (Slimefun, plugins, mods) realmente necesita.
    pub fn abi_1() -> Catalog {
        let h = |id: &str, side: Side, since: (u32, u32), desc: &'static str| HookDef {
            id: HookId(id.to_string()),
            side,
            since: AbiVersion::new(since.0, since.1),
            removed_in: None,
            desc,
        };
        Catalog {
            hooks: vec![
                // ciclo de vida
                h("mod:load", Side::Both, (1, 0), "el mod fue cargado (classpath/nativo listo)"),
                h("mod:init", Side::Both, (1, 0), "inicializacion: entrypoints main"),
                h("mod:ready", Side::Both, (1, 0), "todo cargado; los mods ya se ven entre si"),
                h("mod:shutdown", Side::Both, (1, 0), "antes de descargar"),
                // cliente
                h("client:init", Side::Client, (1, 0), "arranque del cliente"),
                h("client:ready", Side::Client, (1, 0), "cliente listo, ventana abierta"),
                h("client:tick", Side::Client, (1, 0), "un tick del cliente (20/s)"),
                h("client:render", Side::Client, (1, 0), "por frame, antes de dibujar"),
                h("client:key", Side::Client, (1, 0), "tecla pulsada"),
                h("client:join", Side::Client, (1, 0), "conectado a un servidor"),
                h("client:disconnect", Side::Client, (1, 0), "desconectado"),
                h("client:packet", Side::Client, (1, 0), "paquete recibido del servidor"),
                // servidor
                h("server:init", Side::Server, (1, 0), "arranque del servidor"),
                h("server:ready", Side::Server, (1, 0), "servidor aceptando conexiones"),
                h("server:tick", Side::Server, (1, 0), "un tick del servidor"),
                h("server:command", Side::Server, (1, 0), "comando de consola o de jugador"),
                h("server:player_join", Side::Server, (1, 0), "entra un jugador"),
                h("server:player_leave", Side::Server, (1, 0), "sale un jugador"),
                h("server:packet", Side::Server, (1, 0), "paquete recibido de un cliente"),
                h("chat", Side::Both, (1, 0), "mensaje de chat enviado o recibido"),
            ],
        }
    }

    pub fn add(&mut self, def: HookDef) -> Result<(), InterError> {
        if self.find(&def.id).is_some() {
            return Err(InterError::DuplicateHook(def.id.clone()));
        }
        self.hooks.push(def);
        Ok(())
    }

    pub fn find(&self, id: &HookId) -> Option<&HookDef> {
        self.hooks.iter().find(|h| h.id == *id)
    }

    pub fn hooks(&self) -> &[HookDef] {
        &self.hooks
    }

    /// Hooks que existen en una version concreta del ABI.
    pub fn at(&self, abi: AbiVersion) -> impl Iterator<Item = &HookDef> {
        self.hooks
            .iter()
            .filter(move |h| h.since <= abi && h.removed_in.is_none_or(|r| abi < r))
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Lo que un mod declara para el intermediario (en `custom` de su manifiesto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// ABI contra el que fue compilado. Los mods Java no lo llevan (su
    /// "contrato" es el classpath): se verifica por manifiesto.
    #[serde(default)]
    pub abi: Option<String>,
    /// entrypoint -> hooks que ese entrypoint engancha.
    #[serde(default)]
    pub hooks: BTreeMap<String, Vec<String>>,
}

/// Como se ejecuta el codigo del mod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    /// Mod Java: classpath + JVM.
    Java,
    /// Mod nativo: cdylib cargado contra el ABI.
    Native,
}

impl Runtime {
    pub fn label(self) -> &'static str {
        match self {
            Runtime::Java => "Java (JVM)",
            Runtime::Native => "Rust nativo",
        }
    }
}

/// Plan de carga de un mod: que entrypoints llamar y en que hooks enganchan.
#[derive(Debug, Clone)]
pub struct ModPlan {
    pub id: String,
    pub runtime: Runtime,
    pub abi: AbiVersion,
    pub abi_note: String,
    pub entrypoints: Vec<EntryPlan>,
}

#[derive(Debug, Clone)]
pub struct EntryPlan {
    /// nombre del entrypoint (`main`, `client`, `server`...).
    pub name: String,
    /// clases/funciones que invocar, en orden.
    pub targets: Vec<String>,
    /// hooks que este entrypoint engancha.
    pub hooks: Vec<HookId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterError {
    /// `custom.drakes` no es un objeto valido.
    BadContract(String),
    /// El ABI pedido no es una version.
    BadAbi(String),
    /// El mod pide un ABI mas nuevo que el motor.
    AbiTooNew { have: AbiVersion, want: AbiVersion },
    /// Un hook del manifiesto no existe en el catalogo.
    UnknownHook { mod_id: String, hook: HookId },
    /// Un hook no corre en el lado que declaro el mod.
    WrongSide { mod_id: String, hook: HookId },
    /// El mismo hook declarado dos veces.
    DuplicateHook(HookId),
}

impl fmt::Display for InterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadContract(s) => write!(f, "contrato del mod invalido: {s}"),
            Self::BadAbi(s) => write!(f, "ABI invalido '{s}' (usa 1 o 1.2)"),
            Self::AbiTooNew { have, want } => {
                write!(f, "el mod fue compilado contra el ABI {want} y el cargador expone {have}")
            }
            Self::UnknownHook { mod_id, hook } => write!(f, "'{mod_id}' engancha el hook '{hook}', que no existe"),
            Self::WrongSide { mod_id, hook } => {
                write!(f, "'{mod_id}' engancha '{hook}', que no corre en el lado que declaro")
            }
            Self::DuplicateHook(id) => write!(f, "el hook '{id}' ya estaba en el catalogo"),
        }
    }
}
impl std::error::Error for InterError {}

/// Decide el runtime por el manifiesto: nativo si declara ABI, Java si no.
pub fn runtime_of(_m: &ModManifest, contract: Option<&Contract>) -> Runtime {
    match contract {
        Some(c) if c.abi.is_some() => Runtime::Native,
        _ => Runtime::Java,
    }
}

/// Arma el plan de carga de un mod contra el catalogo. No depende de nada mas:
/// el cargador solo tiene que ejecutarlo.
pub fn plan(
    m: &ModManifest,
    catalog: &Catalog,
    side: crate::mods::Environment,
) -> Result<ModPlan, InterError> {
    let contract = contract_of(m)?;
    // ABI: el que declara el mod, o el del catalogo si es Java (no compila
    // contra nada: se verifica por manifiesto)
    let abi = match contract.as_ref().and_then(|c| c.abi.clone()) {
        Some(s) => AbiVersion::parse(&s).ok_or(InterError::BadAbi(s))?,
        None => ABI,
    };
    if !ABI.accepts(abi).ok() {
        return Err(InterError::AbiTooNew { have: ABI, want: abi });
    }
    let abi_note = ABI.accepts(abi).label().to_string();
    let runtime = runtime_of(m, contract.as_ref());
    // entrypoints del manifiesto -> hooks del contrato
    let mut entrypoints = vec![];
    let declared = contract.as_ref().map(|c| c.hooks.clone()).unwrap_or_default();
    for (name, targets) in &m.entrypoints {
        if targets.is_empty() {
            continue;
        }
        let hooks_declared: Vec<String> = declared
            .get(name)
            .cloned()
            .unwrap_or_else(|| default_hook(name).map(|h| vec![h.0]).unwrap_or_default());
        let hooks: Vec<HookId> = hooks_declared
            .iter()
            .map(|h| HookId::new(h).ok_or_else(|| InterError::UnknownHook { mod_id: m.id.clone(), hook: HookId(h.clone()) }))
            .collect::<Result<_, _>>()?;
        for h in &hooks {
            let def = catalog.find(h).ok_or_else(|| InterError::UnknownHook { mod_id: m.id.clone(), hook: h.clone() })?;
            // el hook tiene que existir en el ABI que pide el mod...
            if def.since > abi {
                return Err(InterError::UnknownHook { mod_id: m.id.clone(), hook: h.clone() });
            }
            // ...y correr en un lado donde corre el mod: un mod universal puede
            // enganchar de los dos lados (es lo normal: entrypoint server);
            // uno client-only no puede enganchar hooks de servidor
            if m.environment != crate::mods::Environment::Universal && !def.side.matches(m.environment) {
                return Err(InterError::WrongSide { mod_id: m.id.clone(), hook: h.clone() });
            }
        }
        entrypoints.push(EntryPlan { name: name.clone(), targets: targets.clone(), hooks });
    }
    // sin entrypoints = mod librería (fabric-api): no se ejecuta nada, pero carga

    let _ = side; // el lado solo informa; no veta hooks (ver arriba)
    Ok(ModPlan { id: m.id.clone(), runtime, abi, abi_note, entrypoints })
}

/// Lee `custom.drakes` del manifiesto. Lo que no este ahi es un mod Java sin
/// contrato explicito.
fn contract_of(m: &ModManifest) -> Result<Option<Contract>, InterError> {
    let Some(v) = m.custom.get("drakes") else { return Ok(None) };
    serde_json::from_value::<Contract>(v.clone()).map(Some).map_err(|e| InterError::BadContract(e.to_string()))
}

/// Hook que corresponde a cada entrypoint estandar de Fabric. Un mod que no
/// declara `custom.drakes` los usa igual: es lo que hace el loader real.
pub fn default_hook(entry: &str) -> Option<HookId> {
    let id = match entry {
        "preLaunch" => "mod:load",
        "main" => "mod:init",
        "client" => "client:init",
        "server" => "server:init",
        "modmenu" => return None, // solo UI: no es un hook del juego
        _ => return None,
    };
    HookId::new(id)
}

/// Orden en que se llaman las fases del ciclo de vida, para todos los mods.
pub const PHASES: [&str; 4] = ["mod:load", "mod:init", "mod:ready", "mod:shutdown"];

/// Fase de un hook del ciclo de vida (para saber cuando llamarlo).
pub fn phase_of(hook: &HookId) -> Option<&'static str> {
    PHASES.iter().find(|p| **p == hook.0.as_str()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> ModManifest {
        crate::mods::ModManifest::from_json(json).unwrap()
    }

    fn cat() -> Catalog {
        Catalog::abi_1()
    }

    #[test]
    fn abi_compatibility_rules() {
        let motor = ABI; // 2.0
        assert_eq!(motor, AbiVersion::new(2, 0));
        assert_eq!(motor.accepts(AbiVersion::new(2, 0)), Compat::Ok);
        // compilado contra 2.5 y corre en 2.0: pide hooks que el motor no tiene
        assert_eq!(motor.accepts(AbiVersion::new(2, 5)), Compat::ModNewer);
        assert!(!motor.accepts(AbiVersion::new(2, 5)).ok());
        // compilado contra 2.0 y corre en 2.5: todo lo que usa existe
        let motor25 = AbiVersion::new(2, 5);
        assert_eq!(motor25.accepts(AbiVersion::new(2, 0)), Compat::Ok);
        assert_eq!(motor.accepts(AbiVersion::new(3, 0)), Compat::ModNewer);
        assert!(!motor.accepts(AbiVersion::new(3, 0)).ok(), "ABI 3 rompe: no carga");
        // v1 quedo atras: find/log no recibian userdata
        assert_eq!(motor.accepts(AbiVersion::new(1, 0)), Compat::ModOlder);
        // un mod de ABI 0.x es mas viejo: carga, pero puede faltarle algo
        assert_eq!(motor.accepts(AbiVersion::new(0, 9)), Compat::ModOlder);
        assert!(motor.accepts(AbiVersion::new(0, 9)).ok());
        // versiones del ABI
        assert_eq!(AbiVersion::parse("1"), Some(AbiVersion::new(1, 0)));
        assert_eq!(AbiVersion::parse("1.2"), Some(AbiVersion::new(1, 2)));
        assert_eq!(AbiVersion::parse("v2"), None);
        assert_eq!(AbiVersion::parse("x"), None);
    }

    #[test]
    fn catalog_grows_without_breaking() {
        let c = cat();
        assert!(c.find(&HookId::new("client:tick").unwrap()).is_some());
        assert!(c.find(&HookId::new("mod:init").unwrap()).is_some());
        // hooks desconocidos
        assert!(c.find(&HookId::new("cliente:tick").unwrap()).is_none());
        // todos los hooks del catalogo existen en el ABI actual
        for h in c.hooks() {
            assert!(h.since <= ABI, "{} es del futuro", h.id);
        }
        // agregar uno nuevo no rompe y se ve en su version
        let mut c2 = cat();
        c2.add(HookDef {
            id: HookId::new("client:foo").unwrap(),
            side: Side::Client,
            since: AbiVersion::new(1, 1),
            removed_in: None,
            desc: "nuevo",
        })
        .unwrap();
        assert!(c2.add(HookDef {
            id: HookId::new("client:foo").unwrap(),
            side: Side::Client,
            since: ABI,
            removed_in: None,
            desc: "dup",
        })
        .is_err(), "no se duplica");
        assert_eq!(c2.at(AbiVersion::new(1, 0)).count(), c.hooks().len());
        assert_eq!(c2.at(AbiVersion::new(1, 1)).count(), c.hooks().len() + 1);
    }

    #[test]
    fn java_mod_is_planned_by_manifest() {
        // mod Java: sin ABI, hooks declarados en custom.drakes
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "slimefun", "version": "1.2.0", "environment": "*",
              "entrypoints": { "main": ["net.drakes.slimefun.Main"], "server": ["net.drakes.slimefun.ServerInit"] },
              "depends": { "minecraft": "^26.2" },
              "custom": { "drakes": { "hooks": { "main": ["mod:init"], "server": ["server:init", "server:command"] } } }
            }"#,
        );
        let p = plan(&m, &cat(), crate::mods::Environment::Server).unwrap();
        assert_eq!(p.runtime, Runtime::Java);
        assert_eq!(p.abi, ABI);
        assert_eq!(p.entrypoints.len(), 2);
        let srv = p.entrypoints.iter().find(|e| e.name == "server").unwrap();
        assert_eq!(srv.targets, vec!["net.drakes.slimefun.ServerInit"]);
        assert_eq!(srv.hooks, vec![HookId("server:init".into()), HookId("server:command".into())]);
    }

    #[test]
    fn native_mod_declares_its_abi() {
        // mod nativo: compila contra ABI 1.0 y engancha un hook
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "rustmod", "version": "0.1.0", "environment": "client",
              "entrypoints": { "main": ["drakestar_mod_entry"] },
              "custom": { "drakes": { "abi": "1", "hooks": { "main": ["mod:init", "client:tick"] } } }
            }"#,
        );
        let p = plan(&m, &cat(), crate::mods::Environment::Client).unwrap();
        assert_eq!(p.runtime, Runtime::Native);
        assert_eq!(p.abi, AbiVersion::new(1, 0));
        assert!(p.abi_note.contains("compatible"));
    }

    #[test]
    fn native_mod_of_a_future_abi_does_not_load() {
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "futuro", "version": "0.1.0", "environment": "*",
              "entrypoints": { "main": ["x"] },
              "custom": { "drakes": { "abi": "3", "hooks": { "main": ["mod:init"] } } }
            }"#,
        );
        match plan(&m, &cat(), crate::mods::Environment::Client) {
            Err(InterError::AbiTooNew { have, want }) => {
                assert_eq!(have, ABI);
                assert_eq!(want, AbiVersion::new(3, 0));
            }
            other => panic!("debia fallar por ABI, dio {other:?}"),
        }
    }

    #[test]
    fn unknown_hooks_are_rejected() {
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "m", "version": "1.0.0", "environment": "*",
              "entrypoints": { "main": ["x"] },
              "custom": { "drakes": { "abi": "1", "hooks": { "main": ["mod:init", "hook:que-no-existe"] } } }
            }"#,
        );
        let e = plan(&m, &cat(), crate::mods::Environment::Client).unwrap_err();
        assert!(matches!(e, InterError::UnknownHook { .. }), "{e}");
    }

    #[test]
    fn wrong_side_hook_is_rejected() {
        // mod de cliente que engancha un hook de servidor
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "m", "version": "1.0.0", "environment": "client",
              "entrypoints": { "main": ["x"] },
              "custom": { "drakes": { "abi": "1", "hooks": { "main": ["server:tick"] } } }
            }"#,
        );
        let e = plan(&m, &cat(), crate::mods::Environment::Client).unwrap_err();
        assert!(matches!(e, InterError::WrongSide { .. }), "{e}");
    }

    #[test]
    fn a_mod_of_the_right_side_passes() {
        // universal enganchando hook de servidor: el lado del manifiesto no lo veda
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "m", "version": "1.0.0", "environment": "*",
              "entrypoints": { "main": ["x"] },
              "custom": { "drakes": { "abi": "1", "hooks": { "main": ["server:tick"] } } }
            }"#,
        );
        assert!(plan(&m, &cat(), crate::mods::Environment::Server).is_ok());
    }

    #[test]
    fn bad_contract_is_reported() {
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "m", "version": "1.0.0", "environment": "*",
              "entrypoints": { "main": ["x"] },
              "custom": { "drakes": { "abi": "no-es-abi" } }
            }"#,
        );
        assert!(matches!(plan(&m, &cat(), crate::mods::Environment::Client), Err(InterError::BadAbi(_))));
    }

    #[test]
    fn hooks_from_a_future_abi_minor_do_not_load_yet() {
        // el catalogo declara un hook nuevo en 1.1; un mod que lo pida con ABI 1.0
        // todavia no lo ve
        let mut c = cat();
        c.add(HookDef {
            id: HookId::new("client:nuevo").unwrap(),
            side: Side::Client,
            since: AbiVersion::new(1, 1),
            removed_in: None,
            desc: "del futuro",
        })
        .unwrap();
        let m = manifest(
            r#"{
              "schemaVersion": 1, "id": "m", "version": "1.0.0", "environment": "*",
              "entrypoints": { "main": ["x"] },
              "custom": { "drakes": { "abi": "1", "hooks": { "main": ["client:nuevo"] } } }
            }"#,
        );
        assert!(matches!(
            plan(&m, &c, crate::mods::Environment::Client),
            Err(InterError::UnknownHook { .. })
        ));
    }

    #[test]
    fn lifecycle_phases_are_known() {
        assert_eq!(phase_of(&HookId("mod:init".into())), Some("mod:init"));
        assert_eq!(phase_of(&HookId("client:tick".into())), None);
        assert_eq!(PHASES.len(), 4);
    }

    #[test]
    fn version_req_is_reused_for_abis() {
        // el dialecto de Fabric sirve tambien para pedir ABIs
        let r = crate::mods::VersionReq::parse("^1.0").unwrap();
        assert!(r.matches("1.0.0") && !r.matches("2.0.0"));
    }
}
