#!/usr/bin/env python3
"""Comprueba el flujo de actualizacion real de una release publicada, sin instalar nada.

Descarga latest.json (el endpoint que consulta la app instalada), y para cada plataforma baja el
artefacto y verifica su firma minisign contra la pubkey de src-tauri/tauri.conf.json, igual que el
plugin updater de Tauri. Sale 1 si alguna plataforma falla.

Uso: python3 tools/verify_updater_release.py [--json latest.json] [--only windows-x86_64-msi,...]
Requiere: pip install pynacl
"""
import argparse, base64, hashlib, json, os, sys, urllib.request
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError

ROOT = os.path.join(os.path.dirname(__file__), "..")


def b64lines(text):
    """Un .pub/.sig de minisign: comentario 'untrusted', base64, ['trusted comment', base64]."""
    return [l for l in text.splitlines() if l.strip()]


def parse_pubkey(b64):
    lines = b64lines(base64.b64decode(b64).decode())
    raw = base64.b64decode(lines[1])
    assert raw[:2] == b"Ed", "pubkey no es Ed25519"
    return raw[2:10], VerifyKey(raw[10:42])


def verify(data, sig_b64, keyid, vk):
    lines = b64lines(base64.b64decode(sig_b64).decode())
    raw = base64.b64decode(lines[1])
    alg, sig_keyid, sig = raw[:2], raw[2:10], raw[10:74]
    if sig_keyid != keyid:
        raise ValueError(f"key id {sig_keyid.hex()} != pubkey {keyid.hex()}")
    msg = hashlib.blake2b(data).digest() if alg == b"ED" else data
    vk.verify(msg, sig)
    trusted = lines[2].split(":", 1)[1].strip()
    gsig = base64.b64decode(lines[3])
    vk.verify(sig + trusted.encode(), gsig)  # firma global sobre el trusted comment
    return trusted


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "drakestar-verify"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return r.read()


def main():
    conf = json.load(open(os.path.join(ROOT, "src-tauri/tauri.conf.json")))
    up = conf["plugins"]["updater"]
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", help="latest.json local (por defecto, el endpoint de tauri.conf.json)")
    ap.add_argument("--only", help="plataformas separadas por coma")
    a = ap.parse_args()
    latest = json.load(open(a.json)) if a.json else json.loads(fetch(up["endpoints"][0]))
    keyid, vk = parse_pubkey(up["pubkey"])
    print(f"latest.json version={latest['version']} keyid={keyid[::-1].hex().upper()}")
    only = set(a.only.split(",")) if a.only else None
    cache, fails = {}, 0
    for plat, info in sorted(latest["platforms"].items()):
        if only and plat not in only:
            continue
        url = info["url"]
        try:
            if url not in cache:
                cache[url] = fetch(url)
            data = cache[url]
            trusted = verify(data, info["signature"], keyid, vk)
            print(f"ok   {plat:24} {len(data):>10}B sha256={hashlib.sha256(data).hexdigest()[:16]} [{trusted}]")
        except (BadSignatureError, ValueError, OSError, IndexError) as e:
            fails += 1
            print(f"FAIL {plat:24} {url}: {type(e).__name__} {e}")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
