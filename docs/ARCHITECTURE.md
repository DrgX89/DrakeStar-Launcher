# Arquitectura

## Decisión de stack (recomendada)

| Capa | Elección | Por qué |
|---|---|---|
| Núcleo y servicios | **Rust** (`drakestar-core`, crates de I/O) | descargas, hashes, procesos, perfiles: Rust aporta seguridad y un binario único |
| Shell/UI | **Tauri 2 + Svelte/TypeScript** | UI moderna y ligera, SVG nativo para la marca; no arrastra un Chromium propio como Electron |
| Java | **procesos externos** (JRE gestionado, 21/25) | los mods Java necesitan JVM; se administra, no se incrusta |
| Servidor | JAR de DrakesPurpur como subproceso | aislamiento; un crash no tumba el launcher |
| Cliente Rust | binario gestionado (sin decidir si Horizon-fork o propio) | decisión abierta, ver abajo |

*Alternativa descartada por ahora:* UI nativa 100 % Rust (egui/iced): más control, pero peor para una marca visual rica. Se reevalúa si Tauri molesta.

> El stack recomendado **está implementado** (alfa): Tauri 2 + Svelte 5/TypeScript.

## Capas

```
UI (Svelte)  ──invoke / evento proc-event──▶  src-tauri (glue, sin logica)
                                                   │
                                            drakestar-app  (sin Tauri; todo testeable)
                                   store · process · server · instances · release · ping · java
                                      │                       │
                                drakestar-core          drakestar-net
                          perfiles, canales, manifiesto,   descarga https + SHA-256
                          plan, compat (verdad)
```

`drakestar-core` es puro y determinista. Nada depende hacia arriba. El gestor de procesos (planeado antes como `drakestar-proc`) vive en `drakestar-app::process` y emite eventos a un `EventSink`; `src-tauri` los reenvía a la UI como `proc-event`.

**Una sola fuente de verdad de compatibilidad:** la UI no duplica reglas. Pide `compat_report` (que serializa `drakestar_core::compat`) y valida cada perfil en vivo con `validate_profile` (`Profile::validate`). El backend es la autoridad: un perfil `drakes-rust` con loader o modpack se rechaza al guardar e importar. (El *mock* del modo navegador sí duplica reglas a mano; por eso se marca en pantalla.)

## Estructura del repositorio

```
crates/drakestar-core   modelo + logica pura                      (implementado)
crates/drakestar-cli    validacion y planes                       (implementado)
crates/drakestar-net    descarga + SHA-256                        (implementado)
crates/drakestar-app    perfiles, procesos, servidor, instancias  (implementado)
src-tauri/              comandos IPC (Tauri 2)                    (implementado, alfa)
ui/                     Svelte + TS + Vite, routes.json           (implementado, alfa)
schemas/                JSON Schema de perfil y manifiesto
assets/brand/           SVG de marca (generados)
tools/                  gen_brand.py, screenshots.cjs
docs/  examples/  .github/workflows/ci.yml
```

## Integraciones

- **DrakesPurpur:** `launch::local_server_plan` (usado por `Service::server_start`) produce `java -Xms/-Xmx --enable-native-access=ALL-UNNAMED -jar drakespurpur.jar --nogui` en `profiles/<id>/server`. Java requerido: 25 para MC 26.x. El preset Slimefun (plugins a copiar) sigue por definir y debe salir de artefactos verificados; hoy los plugins se copian a mano a `plugins/`. El servidor se detiene enviando `stop` por stdin y, pasados 30 s, con kill. No se ha probado contra un JAR real de DrakesPurpur.
- **Cliente Rust:** decisión abierta. Opciones: (a) *fork* de Horizon (MIT; hoy 26.3/protocolo 777, con adaptador a 26.2 solo en el cliente), (b) cliente propio. Criterio: demostrar conexión a un DrakesPurpur 26.2 y comportamiento de inventarios de Slimefun antes de prometer nada.
- **Clientes Java:** descarga de versión + loader, JRE adecuado, argumentos JVM; los modpacks se resuelven en el launcher y se lanzan con JVM externa.

## Seguridad

Artefactos solo con SHA-256 válido (`sha256_wellformed`), manifiestos versionados por esquema, sin ejecutar nada descargado sin verificar, sin tocar servidores remotos. Firmado de manifiestos: pendiente.
