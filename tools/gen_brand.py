#!/usr/bin/env python3
"""Genera todos los SVG de marca de DrakeStar. Solo vectores, sin rasters ni recursos externos."""
import os, math
R = os.path.join(os.path.dirname(__file__), "..", "assets", "brand")
BG, BG2 = "#0B0612", "#160A26"
PUR, PUR2, GOLD, GOLD2, RED, TXT, MUT = "#8B3DFF", "#B77CFF", "#F5B942", "#FFE08A", "#FF2E4D", "#F4ECFF", "#9A8AB8"
FONT = "'Orbitron','Rajdhani','Segoe UI',Verdana,sans-serif"

DEFS = f"""<defs>
<linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{GOLD2}"/><stop offset=".45" stop-color="{GOLD}"/><stop offset="1" stop-color="{RED}"/></linearGradient>
<linearGradient id="p" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{PUR2}"/><stop offset="1" stop-color="{PUR}"/></linearGradient>
<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{BG2}"/><stop offset="1" stop-color="{BG}"/></linearGradient>
<filter id="glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="3" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
</defs>"""

def hexpts(cx, cy, r, rot=-90):
    return " ".join(f"{cx + r*math.cos(math.radians(rot+60*i)):.2f},{cy + r*math.sin(math.radians(rot+60*i)):.2f}" for i in range(6))

def star(cx, cy, R_, r_):
    # estrella de 4 puntas (JackStar)
    pts = []
    for i in range(8):
        rad = R_ if i % 2 == 0 else r_
        a = math.radians(-90 + 45*i)
        pts.append(f"{cx+rad*math.cos(a):.2f},{cy+rad*math.sin(a):.2f}")
    return " ".join(pts)

def iso(cx=100, cy=100, s=1.0, glow=True):
    """Isotipo: hexagono (cristal/Rust-forge), cuernos de dragon (rojo) y estrella JackStar (oro)."""
    f = ' filter="url(#glow)"' if glow else ""
    r = 80*s
    horns = (f'<path d="M{cx-52*s},{cy-38*s} L{cx-78*s},{cy-92*s} L{cx-30*s},{cy-62*s} Z" fill="{RED}"/>'
             f'<path d="M{cx+52*s},{cy-38*s} L{cx+78*s},{cy-92*s} L{cx+30*s},{cy-62*s} Z" fill="{RED}"/>')
    return (f'<g{f}>{horns}<polygon points="{hexpts(cx,cy,r)}" fill="url(#bg)" stroke="url(#p)" stroke-width="{6*s}" stroke-linejoin="round"/>'
            f'<polygon points="{hexpts(cx,cy,r*0.72)}" fill="none" stroke="{PUR}" stroke-opacity=".45" stroke-width="{2*s}"/>'
            f'<polygon points="{star(cx,cy,44*s,11*s)}" fill="url(#g)"/>'
            f'<circle cx="{cx}" cy="{cy}" r="{5*s}" fill="{BG}"/></g>')

def svg(w, h, body, title, vb=None):
    vb = vb or f"0 0 {w} {h}"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="{vb}" role="img" aria-label="{title}">'
            f'<title>{title}</title>{DEFS}{body}</svg>\n')

def w(path, content):
    full = os.path.join(R, path); os.makedirs(os.path.dirname(full), exist_ok=True)
    open(full, "w", encoding="utf-8").write(content)

def wordmark(x, y, size, anchor="start"):
    return (f'<text x="{x}" y="{y}" font-family="{FONT}" font-weight="800" font-size="{size}" letter-spacing="{size*0.08:.1f}" text-anchor="{anchor}" fill="{TXT}">DRAKE'
            f'<tspan fill="url(#g)">STAR</tspan></text>')

# --- isotipo y logos ---
w("isotype.svg", svg(200, 200, iso(), "DrakeStar isotipo"))
w("logo.svg", svg(720, 200, iso(100, 100, 1.0) + wordmark(210, 112, 76)
  + f'<text x="214" y="150" font-family="{FONT}" font-size="20" letter-spacing="6" fill="{MUT}">JAVA × RUST · FORGED FOR SLIMEFUN</text>', "DrakeStar logo"))
w("logo-mono.svg", svg(200, 200, f'<polygon points="{hexpts(100,100,80)}" fill="none" stroke="currentColor" stroke-width="6"/><polygon points="{star(100,100,44,11)}" fill="currentColor"/>', "DrakeStar isotipo monocromo"))

# --- banner ---
grid = "".join(f'<path d="M0,{y} L1280,{y}" stroke="{PUR}" stroke-opacity=".12"/>' for y in range(40, 360, 40))
grid += "".join(f'<path d="M{x},0 L{x},360" stroke="{PUR}" stroke-opacity=".08"/>' for x in range(0, 1280, 80))
banner = (f'<rect width="1280" height="360" fill="url(#bg)"/>{grid}'
  f'<path d="M0,330 L360,330 L400,296 L880,296 L920,330 L1280,330" fill="none" stroke="{RED}" stroke-width="2" stroke-opacity=".7"/>'
  f'<path d="M0,342 L340,342 L384,308 L896,308 L940,342 L1280,342" fill="none" stroke="{GOLD}" stroke-width="1" stroke-opacity=".5"/>'
  + iso(220, 170, 1.35) + wordmark(370, 180, 112)
  + f'<text x="376" y="228" font-family="{FONT}" font-size="26" letter-spacing="9" fill="{GOLD}">JAVA × RUST · LAUNCHER</text>'
  + f'<text x="376" y="262" font-family="{FONT}" font-size="17" letter-spacing="4" fill="{MUT}">DRAKESRUST · DRAKESJAVA · LOCAL FORGE · DRAKESCRAFT</text>')
w("banner.svg", svg(1280, 360, banner, "DrakeStar banner"))

# --- splash ---
splash = (f'<rect width="960" height="540" fill="url(#bg)"/>{grid.replace("1280","960")}'
  + iso(480, 230, 1.6) + wordmark(480, 420, 72, "middle")
  + f'<rect x="300" y="456" width="360" height="6" rx="3" fill="{PUR}" fill-opacity=".25"/>'
  + f'<rect x="300" y="456" width="140" height="6" rx="3" fill="url(#g)"><animate attributeName="x" values="300;520;300" dur="2.4s" repeatCount="indefinite"/></rect>'
  + f'<text x="480" y="500" font-family="{FONT}" font-size="14" letter-spacing="6" text-anchor="middle" fill="{MUT}">INICIANDO NUCLEO…</text>')
w("splash.svg", svg(960, 540, splash, "DrakeStar splash"))

# --- iconos de modo y generales (64x64, trazo) ---
def icon(name, inner, color=GOLD):
    w(f"icons/{name}.svg", svg(64, 64, f'<g fill="none" stroke="{color}" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round">{inner}</g>', name))
icon("mode-drakesrust", '<path d="M32 6 54 19v26L32 58 10 45V19Z"/><path d="M22 40V24h10a6 6 0 0 1 0 12H22m10 0 8 4"/>')            # hexagono + R
icon("mode-drakesjava", '<path d="M24 14c0 8 8 8 8 16s-8 8-8 8"/><path d="M34 18c0 6 6 6 6 12s-6 6-6 6"/><path d="M14 46h32c0 6-6 10-16 10s-16-4-16-10Z"/><path d="M46 44c6 0 6-8 0-8"/>', PUR2)
icon("mode-localserver", '<rect x="10" y="10" width="44" height="16" rx="3"/><rect x="10" y="34" width="44" height="16" rx="3"/><path d="M18 18h.1M18 42h.1M42 18h8M42 42h8"/><path d="M32 26v8"/>', GOLD2)
icon("mode-online", '<circle cx="32" cy="32" r="22"/><path d="M10 32h44M32 10c-10 12-10 32 0 44M32 10c10 12 10 32 0 44"/>', RED)
icon("slimefun", '<path d="M32 8 52 20v24L32 56 12 44V20Z"/><path d="M32 20v24M20 26l12 6 12-6"/>')
icon("settings", '<circle cx="32" cy="32" r="8"/><path d="M32 8v8M32 48v8M8 32h8M48 32h8M15 15l6 6M43 43l6 6M49 15l-6 6M21 43l-6 6"/>', PUR2)
icon("play", '<path d="M20 12 52 32 20 52Z"/>', GOLD2)
icon("stop", '<rect x="14" y="14" width="36" height="36" rx="4"/>', RED)
icon("console", '<rect x="6" y="10" width="52" height="44" rx="4"/><path d="M16 24l10 8-10 8M32 42h14"/>', PUR2)
icon("plugin", '<path d="M24 8v12M40 8v12M16 20h32v14a16 16 0 0 1-32 0Z"/><path d="M32 50v8"/>')
icon("world", '<circle cx="32" cy="32" r="22"/><path d="M14 26c8-4 12 4 20 0s10-2 16 2M14 40c8 4 14-4 22 0"/>', GOLD2)
icon("update", '<path d="M52 24A22 22 0 1 0 54 36"/><path d="M52 8v16H36"/>', PUR2)
icon("profile", '<circle cx="32" cy="22" r="10"/><path d="M12 56c2-14 10-20 20-20s18 6 20 20"/>')
icon("modpack", '<path d="M32 6 56 18v28L32 58 8 46V18Z"/><path d="M8 18l24 12 24-12M32 30v28"/>', PUR2)

# --- estados (pastillas) ---
def pill(name, label, color):
    wd = 40 + 11*len(label)
    w(f"ui/status-{name}.svg", svg(wd, 32, f'<rect x="1" y="1" width="{wd-2}" height="30" rx="15" fill="{color}" fill-opacity=".14" stroke="{color}"/><circle cx="16" cy="16" r="5" fill="{color}"/><text x="30" y="21" font-family="{FONT}" font-size="13" letter-spacing="2" font-weight="700" fill="{color}">{label}</text>', f"estado {label}"))
pill("online", "ONLINE", "#3DF5A0"); pill("offline", "OFFLINE", "#7A6C94"); pill("starting", "STARTING", GOLD)
pill("updating", "UPDATING", PUR2); pill("error", "ERROR", RED); pill("experimental", "EXPERIMENTAL", RED)
# canales
pill("channel-stable", "STABLE", "#3DF5A0"); pill("channel-unstable", "UNSTABLE", GOLD); pill("channel-nightly", "NIGHTLY", RED)

# --- botones y tabs ---
def button(name, label, primary):
    wd = 60 + 13*len(label)
    fill = 'url(#g)' if primary else 'none'
    tc = BG if primary else GOLD
    w(f"ui/button-{name}.svg", svg(wd, 52, f'<polygon points="12,1 {wd-1},1 {wd-1},40 {wd-13},51 1,51 1,12" fill="{fill}" stroke="{GOLD}" stroke-width="2"/><text x="{wd/2}" y="33" text-anchor="middle" font-family="{FONT}" font-size="17" font-weight="800" letter-spacing="3" fill="{tc}">{label}</text>', f"boton {label}"))
button("play-primary", "JUGAR", True); button("secondary", "AJUSTES", False); button("danger", "DETENER", False)
def tab(name, label, active):
    wd = 40 + 12*len(label)
    stroke = GOLD if active else PUR
    fill = f'<rect x="0" y="0" width="{wd}" height="44" fill="{PUR}" fill-opacity=".18"/>' if active else ''
    w(f"ui/tab-{name}-{'active' if active else 'idle'}.svg", svg(wd, 44, f'{fill}<path d="M0,43 L{wd},43" stroke="{stroke}" stroke-width="3"/><text x="{wd/2}" y="28" text-anchor="middle" font-family="{FONT}" font-size="14" font-weight="700" letter-spacing="3" fill="{GOLD if active else MUT}">{label}</text>', f"tab {label}"))
for n, l in [("rust", "DRAKESRUST"), ("java", "DRAKESJAVA"), ("server", "LOCAL FORGE"), ("online", "ONLINE")]:
    tab(n, l, True); tab(n, l, False)

# --- mockup de la ventana principal ---
side = ""
for i, (lab, c) in enumerate([("DRAKESRUST", GOLD), ("DRAKESJAVA", PUR2), ("LOCAL FORGE", GOLD2), ("ONLINE", RED), ("PERFILES", MUT), ("AJUSTES", MUT)]):
    y = 120 + i*52
    act = f'<rect x="0" y="{y-24}" width="220" height="40" fill="{PUR}" fill-opacity=".22"/><rect x="0" y="{y-24}" width="4" height="40" fill="{GOLD}"/>' if i == 0 else ""
    side += f'{act}<circle cx="32" cy="{y-4}" r="6" fill="{c}"/><text x="52" y="{y}" font-family="{FONT}" font-size="14" letter-spacing="3" font-weight="700" fill="{TXT if i==0 else MUT}">{lab}</text>'
card = lambda x, y, t, s, col: (f'<rect x="{x}" y="{y}" width="300" height="120" rx="6" fill="{BG2}" stroke="{col}" stroke-opacity=".6"/>'
    f'<text x="{x+18}" y="{y+36}" font-family="{FONT}" font-size="16" font-weight="800" letter-spacing="2" fill="{TXT}">{t}</text>'
    f'<text x="{x+18}" y="{y+62}" font-family="{FONT}" font-size="12" letter-spacing="2" fill="{MUT}">{s}</text>')
mock = (f'<rect width="1280" height="720" fill="url(#bg)"/><rect width="220" height="720" fill="#0E0818"/><line x1="220" y1="0" x2="220" y2="720" stroke="{PUR}" stroke-opacity=".5"/>'
  + iso(54, 50, 0.38, False) + f'<text x="92" y="58" font-family="{FONT}" font-weight="800" font-size="16" letter-spacing="2" fill="{TXT}">DRAKE<tspan fill="url(#g)">STAR</tspan></text>' + side
  + f'<text x="260" y="70" font-family="{FONT}" font-size="30" font-weight="800" letter-spacing="4" fill="{TXT}">DRAKESRUST</text>'
  + f'<text x="260" y="98" font-family="{FONT}" font-size="13" letter-spacing="4" fill="{MUT}">CLIENTE RUST · EXPERIMENTAL · MC 26.2</text>'
  + card(260, 140, "PERFIL: DRAKESCRAFT", "canal NIGHTLY · 8 chunks LOD", RED) + card(580, 140, "PERFIL: LOCAL DEV", "canal UNSTABLE · slimefun", GOLD)
  + card(900, 140, "ESTADO", "cliente Rust: no instalado", PUR)
  + f'<rect x="260" y="290" width="940" height="300" rx="6" fill="#07030C" stroke="{PUR}" stroke-opacity=".5"/>'
  + f'<text x="280" y="324" font-family="monospace" font-size="13" fill="{GOLD}">[drakestar] canal=nightly manifiesto verificado (SHA-256)</text>'
  + f'<text x="280" y="348" font-family="monospace" font-size="13" fill="{MUT}">[drakestar] mock de interfaz — no es una captura de software en ejecucion</text>'
  + f'<polygon points="260,620 470,620 470,668 450,688 260,688" fill="url(#g)"/><text x="365" y="661" text-anchor="middle" font-family="{FONT}" font-size="20" font-weight="800" letter-spacing="4" fill="{BG}">JUGAR</text>')
w("mockups/main-window.svg", svg(1280, 720, mock, "Mockup de la ventana principal (no es una captura real)"))
print("ok")
