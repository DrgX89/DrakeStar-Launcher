use serde::{Deserialize, Serialize};

/// Canal de lanzamiento. Cada perfil declara exactamente uno.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Unstable,
    Nightly,
}

impl Channel {
    /// Riesgo creciente: stable < unstable < nightly.
    pub fn risk(self) -> u8 {
        self as u8
    }

    /// Un perfil en `self` acepta releases publicadas en `release` si la
    /// release es igual o mas estable que el canal del perfil.
    pub fn accepts(self, release: Channel) -> bool {
        release <= self
    }

    pub fn label(self) -> &'static str {
        match self {
            Channel::Stable => "STABLE",
            Channel::Unstable => "UNSTABLE",
            Channel::Nightly => "NIGHTLY",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nightly_accepts_everything_stable_only_stable() {
        assert!(Channel::Nightly.accepts(Channel::Stable));
        assert!(Channel::Nightly.accepts(Channel::Unstable));
        assert!(Channel::Unstable.accepts(Channel::Stable));
        assert!(!Channel::Stable.accepts(Channel::Unstable));
        assert!(!Channel::Unstable.accepts(Channel::Nightly));
    }
}
