# Cambios

## 0.7.0 — Feature-Complete / Feature-Freeze Producción (2026-11-18)

Estado oficial: **Feature-Complete (Producción & Estabilidad)**. A partir de esta versión el launcher entra en modo mantenimiento y estabilidad.

**Nuevo**
- **Modo Dual (Java + DrakesRust / Horizon 26.3)**:
  - **Motor Java optimizado**: flags de Aikar (G1GC tuneado) inyectados automáticamente en el lanzamiento del cliente Java (PR #13).
  - **Motor alterno DrakesRust (Horizon 26.3)**: integración nativa (, , , ) para lanzar el cliente escrito en Rust desde la misma interfaz (PR #12).
- **Monolito Local («Slimefun en solitario / LAN»)**: descarga e inicializa el **monolito DrakesPurpur 26.2** en segundo plano (~22 s en  o LAN, Java 25, 9 suites incluidas). Al cerrar el juego o la ventana del launcher (), el proceso del servidor local se termina limpiamente () sin procesos zombis ni fugas de puertos.
- **Cliente Rust**: lectura de componentes de chat y visualización clara del motivo de kick (PR #11).
- **CI/CD & Seguridad**:  sincronizado a  para compilación reproducible con  (PR #15) y firma automática minisign para Windows (/), Linux (//) y macOS universal ().
- **Validación**: 38/38 tests unitarios de  pasando en verde con .

## 0.6.0 — launcher rehecho (2026-11-12)

Foco: que se pueda **jugar**. Es un launcher de Minecraft, no de servidores.

**Nuevo**
- Botón **JUGAR** que prepara y abre Minecraft de 1.20 a 26.2: Java gestionado por versión (Temurin 17/21/25), librerías y assets verificados, Fabric con Fabric API, mods de rendimiento y DrakeStar Fused (alfa).
- **Cuentas**: login de Microsoft por código de dispositivo (abre el navegador solo; el código se muestra y se copia con un clic) y cuentas sin premium; varias de cada tipo por PC; los tokens se guardan aparte.
- **Interfaz nueva y simple**: Jugar, Cuentas, Instancias, Ajustes y Registros; bienvenida con aviso de privacidad; español e inglés.
- **Registros y reportes**: logs del launcher (14 días) y del juego; si el juego se cierra con error se crea y envía un reporte solo para depurar, con rutas, correos, IPs y claves tachados (desactivable). Recolector en `web.drakescraft.cl/api/launcher/report`.
- **Actualización automática firmada** (minisign) desde las releases de GitHub.
- Modo **sin conexión** con lo ya instalado; primer arranque sin la pantalla del narrador.
- Instaladores: Windows `.exe` y `.msi`, Linux `.deb`/`.rpm`/`.AppImage`, macOS `.dmg`.

**Arreglado**
- «Slimefun solitario/LAN» solo levantaba un servidor y no abría Minecraft; ajustes sin destino; GUI que abrumaba; login sin gestión de cuentas.
- Cerrar el juego desde el launcher ya no cuenta como fallo.

**Cambia**
- Las 0.5.x no tienen actualizador: instalar esta una vez a mano. El diseño anterior (perfiles, servidor local, cliente Rust) ya no está en la interfaz; sus crates siguen en el repo hasta decidir su destino.

**Pendiente**: client ID propio de Azure aprobado por Mojang (hoy usa uno temporal), host Bukkit real en Fused, integración con Horizon. Ver [docs/LAUNCHER.md](docs/LAUNCHER.md) y [docs/ROADMAP.md](docs/ROADMAP.md).

## 0.5.2 y anteriores

Ver [los releases](https://github.com/SlimefunNewHorizons/DrakeStar-Launcher/releases).
