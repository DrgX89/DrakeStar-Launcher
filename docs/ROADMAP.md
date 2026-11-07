# Roadmap

**v0.1 (hecho):** núcleo con tests, CLI, esquemas, ejemplos, marca vectorial, CI, documentación.

**v0.2 — Local Forge usable (mayormente hecho, falta validación real):**
- hecho y probado: `drakestar-net` (descarga https + SHA-256, probada contra un servidor HTTP local), gestor de procesos (arrancar/parar con `stop` y escalada a kill, stdout/stderr en streaming, stdin, logs en disco; probado con scripts `sh` y con un JAR de juguete en Java 25 real), EULA, `server.properties`, listado y activar/desactivar plugins, listado de mundos.
- pendiente: prueba real contra un **JAR de DrakesPurpur**; preset Slimefun reproducible (copiar plugins verificados); crear/importar/respaldar mundos; caché de descargas y rollback; manifiesto oficial publicado.

**v0.3 — Shell gráfico (hecho en alcance alfa):** Tauri 2 + Svelte con todas las rutas de `ui/routes.json`, tema DrakeStar, perfiles CRUD + import/export, instancias, consola en vivo. Verificado en Linux (ventana real bajo Xvfb). Pendiente: bundles/instaladores por plataforma, pruebas en Windows/macOS (el gestor de procesos y las rutas no se han probado allí; los tests de procesos son solo Unix), selector nativo de archivos para import/export (hoy: pegar/elegir archivo en la UI y carpeta `exports/`), pruebas automáticas de UI contra el backend real (hoy solo contra el mock), revisión de accesibilidad con lector de pantalla.

**v0.4 — DrakesJava:** instalación de versiones/loaders, JRE gestionado, lanzamiento del cliente, modpacks, memoria. *Hoy solo perfiles y validación.*

**v0.5 — Online:** DrakesCraft, accesos rápidos, estado del servidor. *Hoy: ping TCP (no el protocolo de estado de Minecraft) y perfiles.*

**v0.6 — Actualizaciones:** manifiestos firmados, canales por componente, rollback. *Hoy: resolución por canal desde un manifiesto importado a mano.*

**v1.0 — DrakesRust:** solo cuando exista un cliente Rust que demuestre conexión a DrakesPurpur 26.2 y comportamiento correcto de GUIs de Slimefun. Hasta entonces el modo se marca *experimental* y **no se puede lanzar**.

Pendiente transversal: instaladores, tests con jugadores reales, Geyser/Bedrock (fuera de alcance), firma de manifiestos, internacionalización (la UI está solo en español).
