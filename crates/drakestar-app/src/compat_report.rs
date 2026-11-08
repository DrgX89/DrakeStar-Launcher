//! Vista serializable de `drakestar_core::compat` para la UI. No anade
//! afirmaciones nuevas: solo `caveat` (texto) cuando algo es verdadero a nivel
//! de diseno pero no esta verificado con software real.

use drakestar_core::compat::{java_mods, server_plugins_visible, slimefun_gui, Support};
use drakestar_core::profile::{Loader, Mode};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub support: Support,
    pub caveat: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderCell {
    pub loader: Loader,
    pub cell: Cell,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeCompat {
    pub mode: Mode,
    pub label: &'static str,
    pub loaders: Vec<LoaderCell>,
    /// Loaders que el perfil de este modo puede elegir (los que no son `No`).
    pub allowed_loaders: Vec<Loader>,
    pub modpacks_allowed: bool,
    pub server_plugins: Cell,
    pub slimefun_gui: Cell,
    /// ¿Puede el launcher lanzar este modo hoy?
    pub launch_implemented: bool,
    pub launch_note: &'static str,
}

fn cell(support: Support, caveat: Option<&str>) -> Cell {
    Cell { support, caveat: caveat.map(String::from) }
}

pub fn label(mode: Mode) -> &'static str {
    match mode {
        Mode::DrakesRust => "DrakesRust",
        Mode::DrakesJava => "DrakesJava",
        Mode::LocalServer => "Local Forge",
        Mode::Online => "Online",
    }
}

pub fn mode_compat(mode: Mode) -> ModeCompat {
    let loaders: Vec<LoaderCell> = [Loader::Vanilla, Loader::Fabric, Loader::Forge, Loader::Neoforge]
        .into_iter()
        .map(|l| {
            let s = java_mods(mode, l);
            let caveat = match (mode, s) {
                (Mode::DrakesJava, Support::Planned) => Some("previsto, no implementado"),
                (Mode::DrakesRust, Support::No) => Some("otro runtime: un cliente Rust no carga mods Java"),
                _ => None,
            };
            LoaderCell { loader: l, cell: cell(s, caveat) }
        })
        .collect();
    let allowed_loaders = loaders.iter().filter(|l| l.cell.support != Support::No).map(|l| l.loader).collect();
    let sp = server_plugins_visible(mode);
    let server_plugins = match mode {
        Mode::DrakesRust => cell(
            sp,
            Some("los plugins viven en el servidor; que un cliente Rust conecte a DrakesPurpur 26.2 NO esta verificado"),
        ),
        _ => cell(sp, None),
    };
    let sg = slimefun_gui(mode);
    let slimefun = match mode {
        Mode::DrakesRust => cell(sg, Some("sin verificar: requiere protocolo de inventarios completo en el cliente")),
        Mode::LocalServer | Mode::Online => cell(sg, Some("no aplica: depende del cliente con el que te conectes")),
        _ => cell(sg, None),
    };
    let (launch_implemented, launch_note) = match mode {
        Mode::LocalServer => (true, "arranca el JAR con el plan de drakestar-core (requiere Java y un JAR en la carpeta del servidor)"),
        Mode::DrakesRust => (true, "lanza el cliente nativo DrakesRust conectado a mc.drakescraft.cl:25565"),
        Mode::DrakesJava => (false, "lanzamiento de cliente Java pendiente (JRE gestionado, loaders y modpacks: v0.4)"),
        Mode::Online => (false, "lanzar el cliente depende de DrakesJava/DrakesRust; aqui solo hay perfiles y ping TCP"),
    };
    ModeCompat {
        mode,
        label: label(mode),
        loaders,
        allowed_loaders,
        modpacks_allowed: mode == Mode::DrakesJava,
        server_plugins,
        slimefun_gui: slimefun,
        launch_implemented,
        launch_note,
    }
}

pub fn report() -> Vec<ModeCompat> {
    [Mode::DrakesRust, Mode::DrakesJava, Mode::LocalServer, Mode::Online].into_iter().map(mode_compat).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rust_accepts_no_loaders_or_modpacks_and_marks_unverified() {
        let m = mode_compat(Mode::DrakesRust);
        assert_eq!(m.allowed_loaders, vec![Loader::Vanilla]);
        assert!(!m.modpacks_allowed);
        assert_eq!(m.slimefun_gui.support, Support::Planned);
        assert!(m.slimefun_gui.caveat.as_deref().unwrap().contains("sin verificar"));
        assert!(m.server_plugins.caveat.is_some());
        assert!(m.launch_implemented);
    }
    #[test]
    fn java_allows_loaders_as_planned_only() {
        let m = mode_compat(Mode::DrakesJava);
        assert_eq!(m.allowed_loaders.len(), 4);
        assert!(m.loaders.iter().filter(|l| l.loader != Loader::Vanilla).all(|l| l.cell.support == Support::Planned));
        assert!(m.modpacks_allowed);
    }
    #[test]
    fn allowed_loaders_agree_with_profile_validation() {
        use crate::store::tests::sample;
        for mode in [Mode::DrakesRust, Mode::DrakesJava, Mode::LocalServer, Mode::Online] {
            let m = mode_compat(mode);
            for l in [Loader::Fabric, Loader::Forge, Loader::Neoforge] {
                let mut p = sample("x", mode);
                p.loader = l;
                assert_eq!(p.validate().is_ok(), m.allowed_loaders.contains(&l), "{mode:?} {l:?}");
            }
        }
    }
    #[test]
    fn local_forge_and_drakes_rust_launch_are_implemented() {
        let n = report().iter().filter(|m| m.launch_implemented).count();
        assert_eq!(n, 2);
    }
}
