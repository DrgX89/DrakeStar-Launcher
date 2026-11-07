# Perfiles (schema v1)

`profiles/<id>/profile.json`; esquema formal en [`schemas/profile.schema.json`](../schemas/profile.schema.json), validación en `drakestar-core::profile`.

Campos: `schema`, `id` (`[a-z0-9_-]{1,48}`), `name`, `mode` (`drakes-rust|drakes-java|local-server|online`), `channel`, `minecraft`, `loader`, `java{major,min_mem_mb,max_mem_mb,extra_jvm_args}`, `server`, `preset`, `modpack`, `notes`.

## Reglas (aplicadas y testeadas)

- `drakes-rust`: sin loader Java ni modpack.
- `drakes-java`: `modpack` exige loader ≠ vanilla; sin `preset`.
- `local-server`: sin loader ni modpack.
- `online`: `server` obligatorio; sin loader ni modpack (es un perfil de conexión).
- `min_mem_mb ≤ max_mem_mb`; `schema` desconocido se rechaza (no se adivina).

Export/importación: el perfil es un único JSON autocontenido (los modpacks se referencian por id, no se incrustan). Ejemplos en `examples/`.
