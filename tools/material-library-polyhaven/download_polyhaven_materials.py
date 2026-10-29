#!/usr/bin/env python3
"""Build a local, organized library of Poly Haven PBR materials.

Uses only Python's standard library. No HDRIs or 3D models are downloaded.
"""
from __future__ import annotations

import argparse
import csv
import json
import os
import re
import shutil
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path
from typing import Any

API = "https://api.polyhaven.com"
USER_AGENT = "ForgeRender-PolyHaven-Material-Library/1.0 (personal asset downloader)"
RESOLUTIONS = ("1k", "2k", "4k", "8k", "16k")
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".exr", ".tif", ".tiff", ".webp"}


def request(url: str, timeout: int = 60) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=timeout) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError) as exc:
            if attempt == 3:
                raise RuntimeError(f"Download fallito: {url}\n{exc}") from exc
            time.sleep(1.5 * (attempt + 1))
    raise RuntimeError(f"Download fallito: {url}")


def api_json(path: str) -> Any:
    return json.loads(request(f"{API}/{path}").decode("utf-8"))


def safe_name(value: str) -> str:
    value = re.sub(r'[<>:"/\\|?*\x00-\x1f]', "_", value).strip(" .")
    return value or "material"


def choose_resolution(available: list[str], requested: str) -> str | None:
    if requested in available:
        return requested
    valid = [r for r in available if r in RESOLUTIONS]
    if not valid:
        return None
    # Prefer the closest resolution, choosing the lower one on a tie.
    target = RESOLUTIONS.index(requested)
    return min(valid, key=lambda r: (abs(RESOLUTIONS.index(r) - target),
                                     RESOLUTIONS.index(r) > target))


def choose_format(map_name: str, formats: dict[str, Any]) -> tuple[str, dict[str, Any]] | None:
    available = [fmt for fmt, info in formats.items()
                 if isinstance(info, dict) and info.get("url")]
    if not available:
        return None
    name = map_name.lower()
    if any(k in name for k in ("diffuse", "albedo", "basecolor", "color", "preview")):
        priority = ("jpg", "png", "exr", "webp", "tif", "tiff")
    else:
        # Preserve floating-point data when Poly Haven offers EXR.
        priority = ("exr", "png", "tif", "tiff", "jpg", "webp")
    fmt = next((f for f in priority if f in available), available[0])
    return fmt, formats[fmt]


def extract_if_archive(path: Path, destination: Path) -> list[str]:
    if not zipfile.is_zipfile(path):
        return []
    extracted: list[str] = []
    with zipfile.ZipFile(path) as archive:
        for member in archive.infolist():
            # Avoid path traversal from untrusted archive entries.
            target = (destination / member.filename).resolve()
            root = destination.resolve()
            if target != root and root not in target.parents:
                continue
            if member.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(member) as src, target.open("wb") as dst:
                shutil.copyfileobj(src, dst)
            extracted.append(str(target.relative_to(destination)))
    return extracted


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Scarica e organizza tutti i materiali PBR Poly Haven (niente HDRI/modelli).")
    parser.add_argument("--output", type=Path, default=Path("PolyHaven_Materials"),
                        help="cartella libreria (default: ./PolyHaven_Materials)")
    parser.add_argument("--resolution", choices=RESOLUTIONS, default="2k",
                        help="risoluzione desiderata; se manca usa quella più vicina (default: 2k)")
    parser.add_argument("--limit", type=int, default=0,
                        help="limita il numero di materiali (utile per una prova); 0 = tutti")
    parser.add_argument("--ids", nargs="*", default=None,
                        help="scarica solo questi ID Poly Haven, separati da spazi")
    parser.add_argument("--skip-existing", action="store_true",
                        help="salta i file già presenti e non vuoti")
    args = parser.parse_args()

    root: Path = args.output.expanduser().resolve()
    root.mkdir(parents=True, exist_ok=True)
    print("Recupero elenco materiali Poly Haven…")
    assets = api_json("assets?t=textures")
    if not isinstance(assets, dict):
        raise RuntimeError("Risposta inattesa dall'API Poly Haven /assets.")

    ids = sorted(args.ids if args.ids else assets.keys())
    ids = [asset_id for asset_id in ids if asset_id in assets]
    if args.limit > 0:
        ids = ids[:args.limit]
    if not ids:
        print("Nessun materiale trovato. Controlla gli ID o la connessione.", file=sys.stderr)
        return 2

    index_rows: list[dict[str, str]] = []
    errors: list[str] = []
    total = len(ids)

    for number, asset_id in enumerate(ids, 1):
        asset = assets.get(asset_id, {})
        title = asset.get("name") or asset_id
        folder = root / safe_name(asset_id)
        maps_dir = folder / "maps"
        maps_dir.mkdir(parents=True, exist_ok=True)
        print(f"[{number}/{total}] {title} ({asset_id})")
        try:
            files = api_json(f"files/{urllib.parse.quote(asset_id)}")
            if not isinstance(files, dict):
                raise RuntimeError("metadati file in formato inatteso")

            manifest: dict[str, Any] = {
                "id": asset_id,
                "name": title,
                "source": f"https://polyhaven.com/a/{asset_id}",
                "license": "CC0",
                "requested_resolution": args.resolution,
                "maps": [],
                "note": "I file non sono convertiti: formato e precisione originali sono preservati.",
            }

            for map_name, resolution_data in files.items():
                if not isinstance(resolution_data, dict):
                    continue
                resolution = choose_resolution(list(resolution_data.keys()), args.resolution)
                if not resolution:
                    continue
                selected = choose_format(map_name, resolution_data[resolution])
                if not selected:
                    continue
                fmt, details = selected
                url = details["url"]
                filename = Path(urllib.parse.urlparse(url).path).name
                if not filename:
                    filename = f"{safe_name(map_name)}.{fmt}"
                target = maps_dir / safe_name(filename)
                if args.skip_existing and target.exists() and target.stat().st_size > 0:
                    status = "already_present"
                else:
                    data = request(url)
                    target.write_bytes(data)
                    status = "downloaded"
                    if target.suffix.lower() == ".zip":
                        extracted = extract_if_archive(target, maps_dir)
                        manifest.setdefault("extracted_files", []).extend(extracted)
                map_record = {
                    "map": map_name,
                    "resolution": resolution,
                    "format": fmt,
                    "file": str(Path("maps") / target.name),
                    "url": url,
                    "status": status,
                    "bytes": target.stat().st_size if target.exists() else 0,
                }
                manifest["maps"].append(map_record)
                print(f"    {map_name}: {resolution} {fmt} — {status}")

            if not manifest["maps"]:
                raise RuntimeError("nessuna mappa scaricabile trovata nei metadati")
            (folder / "material.json").write_text(
                json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            index_rows.append({
                "id": asset_id,
                "name": title,
                "maps": str(len(manifest["maps"])),
                "folder": asset_id,
                "source": manifest["source"],
                "license": "CC0",
            })
        except Exception as exc:
            errors.append(f"{asset_id}: {exc}")
            print(f"    ERRORE: {exc}", file=sys.stderr)

    index_path = root / "index.csv"
    with index_path.open("w", newline="", encoding="utf-8-sig") as handle:
        writer = csv.DictWriter(handle, fieldnames=["id", "name", "maps", "folder", "source", "license"])
        writer.writeheader()
        writer.writerows(index_rows)

    (root / "README.txt").write_text(
        "Libreria locale di materiali PBR Poly Haven\n"
        "===========================================\n\n"
        "Origine: https://polyhaven.com/ — materiali CC0.\n"
        "Questa libreria contiene texture di materiali, non HDRI o modelli 3D.\n"
        "Ogni cartella contiene material.json (metadati) e maps/ (texture originali).\n"
        "index.csv elenca i materiali completati. I file EXR vengono mantenuti come EXR.\n"
        "I materiali Poly Haven sono CC0; consulta https://polyhaven.com/license per i dettagli.\n"
        "Attribuzione consigliata: Textures from Poly Haven (https://polyhaven.com/).\n",
        encoding="utf-8")
    print(f"\nCompletati: {len(index_rows)} / {total}. Indice: {index_path}")
    if errors:
        (root / "errors.txt").write_text("\n".join(errors) + "\n", encoding="utf-8")
        print(f"Errori: {len(errors)}. Dettagli in {root / 'errors.txt'}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("\nInterrotto. Rilancia il comando con --skip-existing per riprendere.", file=sys.stderr)
        raise SystemExit(130)
