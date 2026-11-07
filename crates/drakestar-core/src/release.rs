use crate::channel::Channel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    /// "drakes-rust-client", "drakespurpur", "launcher"...
    pub component: String,
    /// "linux-x86_64", "windows-x86_64", "macos-aarch64" o "any".
    pub platform: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub channel: Channel,
    /// Versiones de Minecraft/protocolo con las que el componente fue verificado.
    pub minecraft: Vec<String>,
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub releases: Vec<Release>,
}

fn version_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

impl Manifest {
    pub fn from_json(s: &str) -> Result<Manifest, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Release mas reciente que un perfil de canal `profile` puede usar para `component`/`platform`/`minecraft`.
    pub fn resolve(
        &self,
        profile: Channel,
        component: &str,
        platform: &str,
        minecraft: &str,
    ) -> Option<(&Release, &Artifact)> {
        self.releases
            .iter()
            .filter(|r| profile.accepts(r.channel) && r.minecraft.iter().any(|m| m == minecraft))
            .filter_map(|r| {
                r.artifacts
                    .iter()
                    .find(|a| {
                        a.component == component && (a.platform == platform || a.platform == "any")
                    })
                    .map(|a| (r, a))
            })
            .max_by(|(a, _), (b, _)| version_key(&a.version).cmp(&version_key(&b.version)))
    }
}

/// Un artefacto solo es instalable si trae un SHA-256 hexadecimal de 64 caracteres.
pub fn sha256_wellformed(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art(c: &str) -> Artifact {
        Artifact {
            component: c.into(),
            platform: "any".into(),
            url: "https://x/y".into(),
            sha256: "0".repeat(64),
            size: 1,
        }
    }
    fn rel(v: &str, ch: Channel) -> Release {
        Release {
            version: v.into(),
            channel: ch,
            minecraft: vec!["26.2".into()],
            artifacts: vec![art("drakes-rust-client")],
            notes: String::new(),
        }
    }

    #[test]
    fn stable_profile_ignores_newer_nightly() {
        let m = Manifest {
            schema: 1,
            releases: vec![
                rel("1.0.0", Channel::Stable),
                rel("1.1.0-nightly", Channel::Nightly),
            ],
        };
        let (r, _) = m
            .resolve(
                Channel::Stable,
                "drakes-rust-client",
                "linux-x86_64",
                "26.2",
            )
            .unwrap();
        assert_eq!(r.version, "1.0.0");
        let (r, _) = m
            .resolve(
                Channel::Nightly,
                "drakes-rust-client",
                "linux-x86_64",
                "26.2",
            )
            .unwrap();
        assert_eq!(r.version, "1.1.0-nightly");
    }

    #[test]
    fn minecraft_version_must_match() {
        let m = Manifest {
            schema: 1,
            releases: vec![rel("1.0.0", Channel::Stable)],
        };
        assert!(m
            .resolve(
                Channel::Stable,
                "drakes-rust-client",
                "linux-x86_64",
                "26.3"
            )
            .is_none());
    }

    #[test]
    fn sha_check() {
        assert!(sha256_wellformed(&"a".repeat(64)));
        assert!(!sha256_wellformed("xyz"));
    }
}
