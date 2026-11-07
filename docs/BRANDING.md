# Marca DrakeStar

**Nombre:** Drake(s) + (Jack)Star → *DRAKESTAR*. Isotipo: hexágono (cristal/forja), **cuernos de dragón** rojos y **estrella de cuatro puntas** dorada (la "star" de JackStar).

| Token | Hex | Uso |
|---|---|---|
| Void | `#0B0612` / `#160A26` | fondos |
| Purple | `#8B3DFF` / `#B77CFF` | estructura, enlaces, DrakesJava |
| Gold | `#F5B942` / `#FFE08A` | acento principal, DrakesRust, CTA |
| Red | `#FF2E4D` | peligro, nightly, cuernos |
| Text / Muted | `#F4ECFF` / `#9A8AB8` | texto |

Tipografía: Orbitron/Rajdhani con *fallback* a sans del sistema. Los textos de los SVG son `<text>` (no trazados): el aspecto exacto depende de la fuente disponible; convertir a trazados para impresión es una tarea pendiente.

**Assets** (`assets/brand/`, generados por `tools/gen_brand.py`; sin rasters ni recursos externos): `logo`, `logo-mono`, `isotype`, `banner`, `splash` (con animación SMIL de carga), 14 iconos (`icons/`), pastillas de estado y canal, 3 botones, 8 tabs (`ui/`), `mockups/main-window.svg` (mockup, no captura).

Usos: no deformar el isotipo; mínimo 24 px; sobre fondo claro usar `logo-mono.svg`. Horizon aporta inspiración técnica, **no** su marca.

**En la UI** (`ui/src/app.css`): los tokens son variables CSS `--void`, `--void-2`, `--purple`, `--purple-2`, `--gold`, `--gold-2`, `--red`, `--text`, `--muted` con los mismos hex de la tabla. Los SVG de `assets/brand` no se redibujan: `npm run sync-brand` (se ejecuta antes de `dev`/`build`) los copia a `ui/public/brand` (ignorado por git) y la UI los usa como `<img>` (logo, iconos de modo, pastillas de canal y de estado, tabs de modo). Los textos de esos SVG van dentro de la imagen, por lo que se usan con `alt` descriptivo.
