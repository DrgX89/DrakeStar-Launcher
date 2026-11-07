<p align="center"><img src="assets/brand/banner.svg" alt="DrakeStar — Java × Rust Launcher" width="100%"></p>

<p align="center">
  <img src="assets/brand/ui/status-experimental.svg" alt="experimental" height="26">
  <img src="assets/brand/ui/status-channel-nightly.svg" alt="canal nightly" height="26">
  <a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-8B3DFF"></a>
</p>

# DrakeStar — el launcher híbrido Java × Rust del ecosistema Drakes

**DrakeStar** (Drakes + JackStar) es el launcher hermano de [DrakesPurpur Hybrid Engine](https://github.com/SlimefunNewHorizons/DrakesPurpur): un solo producto para **jugar, probar y administrar** todo el ecosistema Drakes/Slimefun — cliente Rust, cliente Java, servidor local y servidor online — con identidad visual propia.

> **Estado: v0.1 — esqueleto.** Existe el núcleo (perfiles, canales, manifiesto de releases, planificación de lanzamiento, matriz de compatibilidad) con tests, el CLI de validación, los esquemas y la marca vectorial. **Aún no existe** el shell gráfico, el descargador, el gestor de procesos ni el cliente Rust. Ver [ROADMAP](docs/ROADMAP.md). Las capturas de este README son **mockups**, no software en ejecución.

## Modos

| Modo | Icono | Qué hace | Estado |
|---|---|---|---|
| **DrakesRust** | <img src="assets/brand/icons/mode-drakesrust.svg" width="28"> | Lanza un cliente Rust (Horizon-like). Perfiles propios, LOD/alto rendimiento. **No carga mods Java.** | diseño (cliente por definir) |
| **DrakesJava** | <img src="assets/brand/icons/mode-drakesjava.svg" width="28"> | Perfiles Java vanilla/Fabric/Forge/NeoForge, modpacks, JVM y memoria, import/export. | diseño + modelo de datos |
| **Local Forge** | <img src="assets/brand/icons/mode-localserver.svg" width="28"> | Levanta un DrakesPurpur local con preset Slimefun; logs, consola, plugins, mundos. | plan de lanzamiento implementado y probado; ejecución pendiente |
| **Online** | <img src="assets/brand/icons/mode-online.svg" width="28"> | Perfiles de conexión a DrakesCraft y accesos rápidos, estado del servidor. | modelo de datos |

<p align="center"><img src="assets/brand/mockups/main-window.svg" alt="Mockup de la ventana principal (no es una captura real)" width="80%"></p>

## Compatibilidades reales

Una sola fuente de verdad: [`compat.rs`](crates/drakestar-core/src/compat.rs) (la UI y el CLI leen de ahí; `drakestar compat`).

| | Mods Fabric/Forge/NeoForge | Plugins de servidor (Slimefun…) | GUIs/ítems de Slimefun |
|---|---|---|---|
| Cliente **DrakesJava** | previsto (no implementado) | sí (son del servidor) | sí, con cliente vanilla |
| Cliente **DrakesRust** | **no, nunca** (otro runtime) | sí, a nivel de servidor | **sin verificar** (requiere protocolo de inventarios completo en el cliente) |

- Los **plugins** (Slimefun, StarSuites) corren en el **servidor**; el cliente solo necesita hablar el protocolo.
- Los **mods/modpacks Java** solo existen en DrakesJava. Un perfil `drakes-rust` con loader o modpack **se rechaza** (test incluido).
- Horizon (MIT) está en protocolo 26.3 y DrakesPurpur en 26.2; la relación está en el repositorio de DrakesPurpur (`docs/hybrid/CLIENTE_HORIZON.md`). Hasta probarlo con un cliente real, **no se afirma** que un cliente Horizon/Rust conecte a DrakesPurpur 26.2.

## Canales

| Canal | Pill | Quién lo usa |
|---|---|---|
| `stable` | <img src="assets/brand/ui/status-channel-stable.svg" height="22"> | por defecto; solo releases verificadas |
| `unstable` | <img src="assets/brand/ui/status-channel-unstable.svg" height="22"> | builds de la rama de integración |
| `nightly` | <img src="assets/brand/ui/status-channel-nightly.svg" height="22"> | experimental, puede romper |

Cada **perfil declara un canal**; un perfil `stable` jamás resuelve una release `nightly` (test incluido). Detalle: [docs/RELEASES.md](docs/RELEASES.md).

## Relación con DrakesPurpur Hybrid

Local Forge ejecuta el JAR de DrakesPurpur (Java 25; el motor híbrido extrae su biblioteca nativa Rust del propio JAR). El launcher no duplica lógica del motor: lo descarga por canal (SHA-256), lo configura con un preset y lo supervisa. Los resultados medidos del motor están en el repo de DrakesPurpur, no aquí.

## Uso del núcleo (hoy)

```bash
cargo test --workspace
cargo run -p drakestar-cli -- profile-check examples/profile-local-dev.json
cargo run -p drakestar-cli -- plan examples/profile-local-dev.json   # muestra el comando, no lo ejecuta
cargo run -p drakestar-cli -- compat
```

## Marca

Cyberpunk dorado · morado · rojo. Todo vectorial y regenerable (`python3 tools/gen_brand.py`): logo, isotipo, banner, splash, 14 iconos, pastillas de estado/canal, botones, tabs y mockup. Guía: [docs/BRANDING.md](docs/BRANDING.md).

## Documentación

[ARQUITECTURA](docs/ARCHITECTURE.md) · [PERFILES](docs/PROFILES.md) · [RELEASES](docs/RELEASES.md) · [BRANDING](docs/BRANDING.md) · [ROADMAP](docs/ROADMAP.md) · rutas de UI: [`ui/routes.json`](ui/routes.json)

## Créditos y licencia

MIT. Inspirado conceptualmente en [Horizon](https://github.com/horizon-rust/horizon) (MIT); este repo no copia su código ni su marca. Si en el futuro se reutiliza código de Horizon, se conservará su aviso de licencia.
