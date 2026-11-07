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

> Esto es una **recomendación**; el shell no está implementado.

## Capas

```
UI (Tauri/Svelte)  ──IPC──▶  drakestar-app (comandos)
                                 │
              ┌──────────────────┼────────────────────┐
         drakestar-core     drakestar-net (futuro)   drakestar-proc (futuro)
         perfiles, canales  descarga + SHA-256       procesos, logs, consola
         manifiesto, plan   caché, rollback          Java / servidor / cliente
         compat (verdad)
```

`drakestar-core` es puro y determinista (hoy: 12 tests). Nada depende hacia arriba.

## Estructura del repositorio

```
crates/drakestar-core   modelo + lógica pura            (implementado)
crates/drakestar-cli    validación y planes             (implementado)
crates/drakestar-net    descargas/verificación          (pendiente)
crates/drakestar-proc   gestor de procesos y logs       (pendiente)
crates/drakestar-app    comandos para la UI             (pendiente)
ui/                     rutas (routes.json) y shell     (rutas especificadas)
schemas/                JSON Schema de perfil y manifiesto
assets/brand/           SVG de marca (generados)
tools/gen_brand.py      generador de la marca
docs/  examples/  .github/workflows/ci.yml
```

## Integraciones

- **DrakesPurpur:** `launch::local_server_plan` produce `java -Xms/-Xmx --enable-native-access=ALL-UNNAMED -jar drakespurpur.jar --nogui` en `profiles/<id>/server`. Java requerido: 25 para MC 26.x. El preset Slimefun (plugins a copiar, `server.properties`) está por definir y debe salir de artefactos verificados.
- **Cliente Rust:** decisión abierta. Opciones: (a) *fork* de Horizon (MIT; hoy 26.3/protocolo 777, con adaptador a 26.2 solo en el cliente), (b) cliente propio. Criterio: demostrar conexión a un DrakesPurpur 26.2 y comportamiento de inventarios de Slimefun antes de prometer nada.
- **Clientes Java:** descarga de versión + loader, JRE adecuado, argumentos JVM; los modpacks se resuelven en el launcher y se lanzan con JVM externa.

## Seguridad

Artefactos solo con SHA-256 válido (`sha256_wellformed`), manifiestos versionados por esquema, sin ejecutar nada descargado sin verificar, sin tocar servidores remotos. Firmado de manifiestos: pendiente.
