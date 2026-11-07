# Publicación y preservación

## Auditoría previa a publicar (realizada sobre el commit `872728a`)

| Comprobación | Resultado |
|---|---|
| Secretos (tokens, claves privadas, contraseñas, `.env`) | ninguno encontrado (búsqueda por patrones en todo el árbol) |
| Rutas locales del entorno de trabajo en el código | ninguna |
| Archivos generados versionados (`node_modules`, `target`, `ui/dist`, `ui/public/brand`) | ninguno; excluidos por `.gitignore` |
| Archivos grandes | el mayor es una captura PNG de ~244 KB; historial total ~4 MB |
| Licencia del proyecto | MIT (`LICENSE`) |
| Licencias de dependencias | Rust: MIT / Apache-2.0 / Unicode-3.0 / Zlib / BSD / ISC / MPL-2.0 (5 paquetes) / CDLA-Permissive-2.0; npm: MIT, Apache-2.0, ISC, BSD-3-Clause. Todas permisivas o de copyleft débil a nivel de archivo (MPL-2.0); sin GPL/AGPL |
| Código o marca de Horizon copiados | ninguno (`git ls-files` no contiene archivos de Horizon); solo se cita como inspiración |
| Autoría del historial | 7 commits con autor `Claude <noreply@anthropic.com>` y trailers `Co-Authored-By` / `Claude-Session`. Si prefieres que figure otra autoría, reescribe el historial **antes** del primer push (`git rebase --root --exec 'git commit --amend --reset-author --no-edit'` con tu `user.name`/`user.email`) |
| Capturas | 3 son de la app Tauri real (Xvfb), 10 del modo navegador con mock, etiquetadas |

## Publicar en el repositorio de la organización

```bash
# 1. Crear el repositorio vacío (sin README/licencia/.gitignore):
#    https://github.com/organizations/SlimefunNewHorizons/repositories/new  ->  DrakeStar-Launcher
# 2. Restaurar desde el bundle y subir:
git clone DrakeStar-Launcher.bundle DrakeStar-Launcher
cd DrakeStar-Launcher
git remote set-url origin https://github.com/SlimefunNewHorizons/DrakeStar-Launcher.git
git push -u origin main
```

Verificar el bundle antes de usarlo: `git bundle verify DrakeStar-Launcher.bundle`.

## Preservación
El bundle contiene todo el historial (`main`). Guárdalo fuera de la sesión (la sesión y su contenedor son efímeros).
