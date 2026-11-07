//! Matriz de compatibilidad HONESTA. Una sola fuente de verdad: la UI y el CLI la leen de aqui.

use crate::profile::{Loader, Mode};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Support {
    Yes,
    No,
    /// Existe en el diseno pero no esta implementado/verificado todavia.
    Planned,
}

/// ¿Puede el modo `mode` cargar mods/loaders Java?
pub fn java_mods(mode: Mode, loader: Loader) -> Support {
    match (mode, loader) {
        (_, Loader::Vanilla) => Support::Yes,
        (Mode::DrakesJava, _) => Support::Planned,
        _ => Support::No,
    }
}

/// ¿Pueden conectar a un servidor DrakesPurpur con plugins (Slimefun)?
/// Los plugins son de servidor: cualquier cliente compatible con el protocolo los ve.
pub fn server_plugins_visible(mode: Mode) -> Support {
    match mode {
        Mode::DrakesRust | Mode::DrakesJava | Mode::Online => Support::Yes,
        Mode::LocalServer => Support::Yes,
    }
}

/// Las GUI/items de Slimefun se sirven desde el servidor con recursos vanilla;
/// no requieren mods de cliente. Un cliente Rust debe implementar el protocolo
/// de inventarios completo para usarlas: hasta verificarlo, queda como Planned.
pub fn slimefun_gui(mode: Mode) -> Support {
    match mode {
        Mode::DrakesJava => Support::Yes,
        Mode::DrakesRust => Support::Planned,
        _ => Support::No,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rust_client_never_claims_java_mods() {
        for l in [Loader::Fabric, Loader::Forge, Loader::Neoforge] {
            assert_eq!(java_mods(Mode::DrakesRust, l), Support::No);
        }
    }
    #[test]
    fn rust_slimefun_gui_is_not_overclaimed() {
        assert_eq!(slimefun_gui(Mode::DrakesRust), Support::Planned);
    }
}
