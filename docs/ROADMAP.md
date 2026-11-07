# Roadmap

**v0.1 (hecho):** núcleo con tests, CLI, esquemas, ejemplos, marca vectorial, CI, documentación.

**v0.2 — Local Forge usable (hecho en backend, falta prueba end-to-end contra JAR completo de DrakesPurpur):**
- hecho y probado: `drakestar-net` (descarga https + SHA-256, probada contra un servidor HTTP local), gestor de procesos (arrancar/parar con `stop` y escalada a kill, stdout/stderr en streaming, stdin, logs en disco; tests para Unix `sh` y Windows `cmd /C`, además de JAR de juguete en Java 25 real), EULA, `server.properties`, listado y activar/desactivar plugins, instalación de preset Slimefun (`install_slimefun_preset`), listado de mundos y copias de seguridad (`backup_world`).
- pendiente: prueba automática en CI contra un **JAR completo de DrakesPurpur**; caché de descargas compartida y rollback; manifiesto oficial publicado.

**v0.3 — Shell gráfico (hecho en alcance alfa):** Tauri 2 + Svelte con todas las rutas de `ui/routes.json`, tema DrakeStar, perfiles CRUD + import/export, instancias, consola en vivo, `preserveSymlinks` en Vite para compilación multiplataforma en Windows/junctions y Linux. Verificado en Linux (ventana real bajo Xvfb). Pendiente: bundles/instaladores por plataforma, selector nativo de archivos para import/export (hoy: pegar/elegir archivo en la UI y carpeta `exports/`), pruebas automáticas de UI contra el backend real (hoy contra el mock), revisión de accesibilidad con lector de pantalla.

**v0.4 — DrakesJava:** instalación de versiones/loaders, JRE gestionado, lanzamiento del cliente, modpacks, memoria. *Hoy solo perfiles y validación.*

**v0.5 — Online:** DrakesCraft, accesos rápidos, estado del servidor. *Hoy: ping TCP y protocolo de estado de Minecraft (`mc_status_ping`, Server List Ping `0x00` con MOTD, versión, protocolo y jugadores) con tests unitarios, más gestión de perfiles.*

**v0.6 — Actualizaciones:** manifiestos firmados, canales por componente, rollback. *Hoy: resolución por canal desde un manifiesto importado a mano.*

**v1.0 — DrakesRust:** solo cuando exista un cliente Rust que demuestre conexión a DrakesPurpur 26.2 y comportamiento correcto de GUIs de Slimefun. Hasta entonces el modo se marca *experimental* y **no se puede lanzar**.

Pendiente transversal: instaladores, tests con jugadores reales, Geyser/Bedrock (fuera de alcance), firma de manifiestos, internacionalización (la UI está solo en español).
