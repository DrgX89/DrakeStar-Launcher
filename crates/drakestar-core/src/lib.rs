//! DrakeStar core: modelo de datos y logica pura (sin I/O de red ni UI).
//!
//! Todo lo que esta aqui es determinista y testeable. El shell de UI y el
//! gestor de procesos dependen de este crate, nunca al reves.

pub mod channel;
pub mod compat;
pub mod fabric;
pub mod inter;
pub mod launch;
pub mod mods;
pub mod native;
pub mod profile;
pub mod release;

pub use channel::Channel;
pub use profile::{Mode, Profile};
