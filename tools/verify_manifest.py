#!/usr/bin/env python3
"""Descarga cada artefacto de manifest.json y compara tamano y SHA-256.
Uso: tools/verify_manifest.py [manifest.json] [--fix]  (--fix reescribe size/sha256 reales)
Sale con 1 si algun artefacto no coincide, responde mal o tiene hash de ceros."""
import hashlib, json, sys, urllib.request
path = next((a for a in sys.argv[1:] if not a.startswith("--")), "manifest.json")
fix = "--fix" in sys.argv
m = json.load(open(path, encoding="utf8"))
bad = 0
for r in m["releases"]:
    for a in r["artifacts"]:
        tag = f'{r["version"]} {a["component"]}'
        try:
            h, n = hashlib.sha256(), 0
            with urllib.request.urlopen(urllib.request.Request(a["url"], headers={"User-Agent": "drakestar-verify"}), timeout=120) as resp:
                while chunk := resp.read(1 << 20):
                    h.update(chunk); n += len(chunk)
        except Exception as e:
            print(f"FALLA {tag}: {e}"); bad += 1; continue
        ok = h.hexdigest() == a["sha256"] and n == a["size"]
        print(("OK    " if ok else "DIFIERE ") + f"{tag} size={n} sha={h.hexdigest()[:12]}")
        if not ok:
            bad += 1
            if fix: a["sha256"], a["size"] = h.hexdigest(), n
if fix:
    json.dump(m, open(path, "w", encoding="utf8"), indent=2, ensure_ascii=False); open(path, "a").write("\n")
sys.exit(1 if bad and not fix else 0)
