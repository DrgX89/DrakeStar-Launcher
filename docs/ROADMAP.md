# Roadmap

**v0.1 (hecho):** núcleo con tests, CLI, esquemas, ejemplos, marca vectorial, CI, documentación.

**v0.2 — Local Forge usable (hecho en backend, falta prueba end-to-end contra JAR completo de DrakesPurpur):**
- hecho y probado: `drakestar-net` (descarga https + SHA-256, probada contra un servidor HTTP local), gestor de procesos (arrancar/parar con `stop` y escalada a kill, stdout/stderr en streaming, stdin, logs en disco; tests para Unix `sh` y Windows `cmd /C`, además de JAR de juguete en Java 25 real), EULA, `server.properties`, listado y activar/desactivar plugins, instalación de preset Slimefun (`install_slimefun_preset`), listado de mundos y copias de seguridad (`backup_world`).
- pendiente: prueba automática en CI contra un **JAR completo de DrakesPurpur**; caché de descargas compartida y rollback; manifiesto oficial publicado.

**v0.3 — Shell gráfico (hecho en alcance alfa):** Tauri 2 + Svelte con todas las rutas de `ui/routes.json`, tema DrakeStar, perfiles CRUD + import/export, instancias, consola en vivo, `preserveSymlinks` en Vite para compilación multiplataforma en Windows/junctions y Linux. Verificado en Linux (ventana real bajo Xvfb). Pendiente: bundles/instaladores por plataforma, selector nativo de archivos para import/export (hoy: pegar/elegir archivo en la UI y carpeta `exports/`), pruebas automáticas de UI contra el backend real (hoy contra el mock), revisión de accesibilidad con lector de pantalla.

**v0.4 — Sistema de mods y cliente nativo (en curso):** el launcher pasa de lanzar el juego a **decidir que mods entran**, y a cargar codigo nativo ademas de Java.
- **hecho y probado**: instalacion y lanzamiento del cliente Java con **Fabric** (Mojang + Fabric verificados por hash; ejecutado con 52 mods reales: Sodium, Distant Horizons, fabric-api y sus 48 submodulos). **Sistema de mods propio**: manifiesto `fabric.mod.json` sin modificar, dialecto semver de Fabric, resolucion de dependencias, **cargador** que descubre mods (incluidos sus jar-in-jar), verifica SHA-256 contra `mods.lock.json` y deshabilita lo que no carga. **Intermediario** con ABI versionado y catalogo de hooks. **Adaptador nativo** (`cdylib`) y **motor de hooks** que ejecuta un mod Rust de punta a punta. **Cliente Rust nativo** (`drakestar-rust-client`) que el launcher instala por canal, verifica y lanza, probado contra un mini-servidor de login. **Analisis pre-arranque**: dependencias incumplidas, conflictos, ids duplicados y colisiones de clases objetivo de Mixin (lo que hace crashear Fabric en caliente).
- **pendiente**: JRE gestionado por el launcher (hoy usa el del sistema), assets/sonidos, modpacks, Forge/NeoForge, Windows/macOS, motor Rust que publique hooks reales del juego, y publicar el binario del cliente Rust como release.

**v0.5 — Online:** DrakesCraft, accesos rápidos, estado del servidor. *Hoy: ping TCP y protocolo de estado de Minecraft (`mc_status_ping`, Server List Ping `0x00` con MOTD, versión, protocolo y jugadores) con tests unitarios, más gestión de perfiles.*

**v0.6 — Actualizaciones:** manifiestos firmados, canales por componente, rollback. *Hoy: resolución por canal desde un manifiesto importado a mano.*

**v1.0 — DrakesRust:** solo cuando exista un cliente Rust que demuestre conexión a DrakesPurpur 26.2 y comportamiento correcto de GUIs de Slimefun. Hasta entonces el modo se marca *experimental* y **no se puede lanzar**.

**Analisis pre-arranque:** el cargador detecta dependencias incumplidas (con el dialecto semver de Fabric), conflictos declarados, ids duplicados y **colisiones de clases objetivo de Mixin** — que es lo que hace crashear Fabric en caliente. Se suma al `modpack` delta-sync: uno decide que se descarga, el otro que se carga.

Pendiente transversal: instaladores, tests con jugadores reales, Geyser/Bedrock (fuera de alcance), firma de manifiestos, internacionalización (la UI está solo en español).
