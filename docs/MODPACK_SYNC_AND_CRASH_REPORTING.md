# Sincronización Autoritativa de Mods y Reporte de Crashes (Próxima Actualización)

Especificación de diseño para distribuir el modpack oficial de DrakesCraft con **1 clic** desde el launcher, usando el servidor como única fuente de verdad (análogo al sistema de recarga de texturas en **Odysseia** / `pack.drakescraft.cl`).

---

## 1. Principio: Servidor como Única Fuente de Verdad (*Server-Authoritative Mod Sync*)

El cliente **nunca** decide versiones por su cuenta ni actualiza mods desde fuentes externas (Modrinth/CurseForge) si no coinciden con el servidor:

- **Pinning estricto de versión:** Si el servidor corre `LuckPerms v1` aunque exista `v2` publicada afuera, el cliente descarga y conserva exactamente `LuckPerms v1` desde el espejo configurado del servidor (Oracle / Star / Cloudflare CDN).
- **Cero reempaquetado manual del cliente:** Si en el servidor se agrega, parchea o **elimina** un mod, basta con regenerar el manifiesto en el servidor; al siguiente clic en **JUGAR**, el jugador queda sincronizado automáticamente antes de lanzar Minecraft.
- **Infraestructura objetivo aislada:** El servidor de mods correrá en un nodo dedicado (p. ej. **Oracle Cloud Always Free**: 4 OCPUs ARM + 24 GB RAM) completamente separado de producción en Dallas (`DrakesPurpur`).

---

## 2. Flujo al presionar JUGAR

```text
[Clic en JUGAR]
   │
   ▼
GET https://<host-modpack>/mods-manifest.json   (timeout corto + caché ETag/SHA-256)
   │
   ├── Sin cambios (hash de manifiesto idéntico) ──► Lanzar juego directo
   │
   └── Actualización detectada ("Actualizando mods desde el servidor...")
          1. Verificar loader y versión de Minecraft declarados en el manifiesto
          2. Descargar JARs nuevos o con SHA-256 distinto a .tmp y validar SHA-256
          3. Mover atómicamente a mods/ del perfil gestionado
          4. Eliminar de mods/ cualquier JAR ausente en el manifiesto
             (los mods opcionales locales del usuario se aíslan aparte si se habilitan)
          5. Actualizar estado local y lanzar Minecraft
```

### Contrato mínimo de `mods-manifest.json`

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
  ]
}
```

- **Seguridad:** Todo archivo se valida contra su `sha256` antes de entrar en `mods/`. Sin `sha256` válido, no se reemplaza ni se ejecuta.
- **Resiliencia:** Si el servidor de manifiesto no responde temporalmente pero el perfil local ya está completo y el usuario elige modo offline/reintento, el launcher informa el estado sin corromper `mods/`.

---

## 3. Telemetría de Crashes: Cliente → Star (SAORI) → GitHub

Extensión del sistema de registros y reportes de DrakeStar para separar fallos del launcher vs. fallos del juego o de un mod específico:

1. **Detección y clasificación al cerrar:**
   - **Cierre normal (`exit 0` o salida iniciada por el usuario):** No genera reporte.
   - **Crash del Launcher:** Excepción o pánico en Tauri/Rust/UI.
   - **Crash de Minecraft / Mods:** Código de salida distinto de `0`, aparición de `crash-reports/crash-*.txt` nuevo o `hs_err_pid*.log`. El parser extrae:
     - `pack_id` + `revision` del modpack sincronizado,
     - lista de JARs/hashes cargados,
     - mod sospechoso o mixin culpable si aparece en el stacktrace,
     - últimas líneas de `latest.log` / `crash-report` **sanitizadas** (rutas de usuario, correos, IPs y tokens tachados; respetando *Ajustes → Privacidad*).
2. **Ingesta en Star VPS (SAORI):**
   - El launcher envía el payload JSON firmado/limitado por rate-limit al endpoint recolector en **Star**.
   - **SAORI** agrupa crashes duplicados por firma (hash del stacktrace + `revision`) y crea o actualiza automáticamente un **Issue en GitHub** (en `DrakeStar-Launcher` si es fallo del launcher, o en el repositorio del modpack si es fallo de mods/configuración) con etiqueta de severidad y conteo de jugadores afectados.
