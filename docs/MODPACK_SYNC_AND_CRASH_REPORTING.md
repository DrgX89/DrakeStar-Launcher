# Sincronización Autoritativa de Mods, Auditoría de Cliente y Privacidad

Especificación técnica y de gobernanza para implementar en **DrakeStar-Launcher** la sincronización de modpacks controlada por el servidor, el control transparente de mods adicionales del jugador, el triage automático de crashes y las reglas de seguridad/privacidad.

> **Estado:** Especificación aprobada para desarrollo (`v0.7+`). Todo agente o colaborador que implemente estas funciones debe seguir este documento y [AGENTS.md](../AGENTS.md).

---

## 1. Principio: Servidor como Única Fuente de Verdad (*Server-Authoritative Mod Sync*)

Análogo al sistema de recarga de texturas de **Odysseia** (`pack.drakescraft.cl`), el servidor (nodo dedicado futuro en **Oracle** o backend en **Star** detrás de Cloudflare) es la única fuente de verdad del modpack:

- **Pinning estricto de versión:** Si el servidor corre `LuckPerms v1` aunque exista `v2` en Modrinth/CurseForge, el cliente descarga y conserva exactamente `LuckPerms v1` desde el espejo configurado del servidor.
- **Cero reempaquetado manual del modpack:** Agregar, parchear o **eliminar** un mod en el servidor solo requiere actualizar `mods-manifest.json`. Al siguiente clic en **JUGAR**, el cliente queda sincronizado antes de abrir Minecraft.
- **Separación de mods gestionados vs. mods propios del jugador:**
  - El launcher mantiene un registro local de archivos gestionados (`.drakestar-managed.json` o carpeta separada para extras).
  - Si un `.jar` estaba en el manifiesto del servidor y el servidor lo retiró, el launcher **lo elimina**.
  - Si un `.jar` fue agregado manualmente por el jugador (p. ej. cosméticos o rendimiento) y no estaba en el manifiesto gestionado, el launcher **no lo borra**: lo clasifica como **mod adicional del jugador** y aplica la política de la Sección 3.

---

## 2. Flujo de Sincronización al presionar JUGAR

```text
[Clic en JUGAR]
   │
   ▼
GET https://<host-modpack>/mods-manifest.json   (ETag / SHA-256 + timeout acotado)
   │
   ├── 1. Sincronizar mods gestionados por el servidor:
   │      • Descargar JARs nuevos/modificados a .tmp y verificar SHA-256
   │      • Mover atómicamente a mods/ y actualizar .drakestar-managed.json
   │      • Eliminar de mods/ los JARs gestionados que salieron del manifiesto
   │
   ├── 2. Inspeccionar mods adicionales del jugador (si existen):
   │      • Calcular SHA-256 + leer metadata (mod_id, version) del JAR
   │      • Evaluar contra policy.allowed / policy.blocked del manifiesto
   │
   └── 3. Advertir / Bloquear / Lanzar
```

### Contrato de `mods-manifest.json`

```json
{
  "schema": 1,
  "pack_id": "drakescraft-mods",
  "revision": "2026.11.26-01",
  "minecraft": "1.21.1",
  "loader": { "name": "neoforge", "version": "21.1.x" },
  "mods": [
    {
      "id": "create",
      "file": "create-1.21.1-x.y.z.jar",
      "sha256": "<hex-64>",
      "size": 12345678,
      "url": "https://cdn.drakescraft.cl/modpack/mods/create-1.21.1-x.y.z.jar",
      "side": "both"
    }
  ],
  "policy": {
    "allowed_client_sha256": ["<hash-iris>", "<hash-sodium>"],
    "blocked_ids": ["baritone", "meteor-client", "wurst", "aristois"],
    "blocked_sha256": ["<hash-xray>"]
  }
}
```

---

## 3. Control de Mods Adicionales del Jugador (Sin Fricción ni Spam)

Al detectar mods agregados por el usuario fuera del pack del servidor:

| Clasificación | Comportamiento en DrakeStar-Launcher |
| :--- | :--- |
| **Permitido** (`allowed_client_sha256`) | Entra directo sin advertencias molestas (cosméticos, shaders, rendimiento). |
| **Prohibido** (`blocked_ids` / `blocked_sha256`) | **Bloquea el botón JUGAR** hacia el servidor oficial e indica exactamente qué `.jar` retirar. |
| **Desconocido** (no listado aún) | Muestra advertencia clara antes de lanzar: *"Sincronización completada, pero detectamos mod(s) propio(s). El servidor no garantiza compatibilidad ni soporte (p. ej., funciones de Litematica pueden requerir soporte en servidor). Recomendamos usar solo cosméticos o rendimiento."* |

### Auditoría determinista en el servidor (Star / Oracle — Sin usar IA por defecto)

Cuando el usuario lanza hacia el **servidor oficial** (nunca en modo local/single-player) y aceptó los términos de juego, el launcher envía únicamente:
`username`, `account_type` (`premium` | `offline`), `pack_revision` y la lista de **mods adicionales** (`file`, `mod_id`, `version`, `sha256`).

En el servidor receptor (**Star** hoy detrás de Cloudflare Tunnel, o nodo aislado futuro en **Oracle**), la resolución funciona en **2 etapas con cero spam**:

1. **Etapa 1 — 100% determinista (sin IA):**
   - Consulta caché local por `sha256`.
   - Si es nuevo, consulta automáticamente **Modrinth API** (`GET /v2/version_file/{sha256}?algorithm=sha256`) y **CurseForge API** (por fingerprint/hash) para identificar proyecto oficial, autor, categoría y URL de origen.
   - Guarda el resultado cacheado por `sha256` en la base del servidor.
2. **Etapa 2 — Búsqueda web con IA (solo fallback opcional y deduplicado):**
   - Únicamente si el `sha256` no existe ni en Modrinth ni en CurseForge (p. ej. mod casero o compilado manualmente), SAORI puede lanzar **una única búsqueda web asistida por IA por cada `sha256` nuevo** para recabar contexto.
   - **Cero spam de alertas:** No se envían correos ni alertas por cada conexión. Todo queda en un **log/digest consolidado** agrupado por `sha256` y solo escala si hay coincidencia sospechosa o sanción/infracción asociada a la cuenta.

---

## 4. Pipeline de Logs de Errores y Triage de Crashes

Extiende el sistema actual de registros de DrakeStar:

- **Separación explícita:** clasifica si el fallo ocurrió en el **launcher** (Tauri/Rust/UI), en el motor nativo **Horizon** (Rust) o en **Minecraft / Mods** (`exit != 0`, `crash-reports/`, `hs_err_pid*.log`).
- **Atribución de cliente modificado:** si había mods propios del jugador cargados al momento del crash, el reporte incluye `client_modified: true` y la lista de esos mods para no confundir fallos propios del usuario con bugs del modpack.
- **Destino y finalidad:** los logs sanitizados (sin rutas personales, correos, IPs ni tokens) se envían a **Star (SAORI)** exclusivamente para diagnosticar y corregir errores del launcher/modpack, agrupando duplicados por firma antes de abrir o actualizar un Issue en GitHub.

---

## 5. Seguridad y Endurecimiento (*Proteger Star / Futuro Oracle*)

Toda implementación cliente/servidor de estas rutas debe cumplir sin excepciones:

1. **Cero secretos en el cliente:** El binario/código de DrakeStar-Launcher jamás incluye llaves de BD, tokens de GitHub, API keys de CurseForge/Modrinth privadas ni tokens admin de Star.
2. **Validación estricta de entrada:** El endpoint receptor en Star valida esquema JSON estricto, tamaño máximo de payload (p. ej. `<= 64 KB`), cantidad máxima de items, y regex estricta para `sha256` (`^[a-f0-9]{64}$`), `mod_id` y `username`.
3. **Rate limiting & WAF:** Límites por IP y por usuario en Cloudflare WAF + servidor receptor para impedir inundación de logs o denegación de servicio.
4. **Aislamiento contra inyección:** Todo campo recibido del cliente (`file`, `mod_id`, líneas de crash log) se trata como **datos no confiables**: prohibida cualquier interpolación en shell/SQL, escape estricto en vistas/issues y sanitización contra *prompt injection* antes de cualquier análisis opcional con IA.
5. **Aislamiento de producción:** Ni el recolector de crashes ni el sync de modpacks tienen acceso de escritura a producción Dallas (`DrakesPurpur`).

---

## 6. Transparencia, Privacidad y Aviso Legal (Launcher + GitHub + `drakescraft.cl`)

Las pantallas del launcher, este repositorio y las páginas legales existentes de **`drakescraft.cl`** (`/terms.html`, `/terms-en.html`, `/privacidad.html`, `/privacidad-en.html`, `/reglas.html`) deben mantener coherencia total sobre estos puntos:

1. **Proyecto sin ánimo de lucro y código abierto:** DrakeStar-Launcher se distribuye sin fines de lucro bajo licencia **MIT**; cualquier jugador puede auditar exactamente qué datos salen del cliente revisando el código fuente en GitHub.
2. **Transparencia exacta de datos enviados:**
   - **Reportes de error/crash** (desactivables en *Ajustes → Privacidad*): versión del launcher/Minecraft, sistema operativo, estado de cliente modificado y fragmentos sanitizados de logs, usados **únicamente** para corregir fallos del launcher y mejorar la estabilidad.
   - **Auditoría de mods adicionales al entrar al servidor oficial** (no aplica en modo local/single-player): `username` (premium o no premium, solo para identificar la cuenta dentro del servidor) + nombres/versiones/SHA-256 de los mods externos añadidos por el jugador.
3. **No afiliación con Mojang / Microsoft:**
   - *DrakeStar-Launcher y DrakesCraft Labs son proyectos comunitarios independientes, sin ánimo de lucro. **NO son un producto oficial de Minecraft** ni están aprobados por o asociados con **Mojang AB** o **Microsoft Corporation**.*
   - Aplica tanto al flujo tradicional de Minecraft Java (Vanilla/Fabric/NeoForge) como al cliente experimental en Rust (**Horizon**).
