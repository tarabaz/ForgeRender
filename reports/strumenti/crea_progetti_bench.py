"""Crea i progetti di benchmark derivati dalla demo del cocktail (stessa scena, camera fissa,
senza profondità di campo, seed fisso). I file nascono accanto alla demo, così i percorsi
relativi dei modelli funzionano. I file sono nel repository e nello zip portable (benchmark
pronti senza Python); lo script serve a rigenerarli quando cambia la demo, e la CI controlla
che siano allineati.

Uso, dalla radice del repository:
    python reports/strumenti/crea_progetti_bench.py

Crea in samples/scene/cocktail-ghiaccio-secco/:
    bench_intera.frproject   camera "Demo" senza DOF                 (report majorant 3D)
    bench_vicina.frproject   camera ravvicinata sulla nebbia, 22°     (report majorant 3D)
    bench_ad.frproject       camera "Demo" senza DOF, seed dalla CLI  (report campionamento adattivo)
"""
import json
from pathlib import Path

DIR = Path("samples/scene/cocktail-ghiaccio-secco")
src = json.loads((DIR / "cocktail-ghiaccio-secco.frproject").read_text(encoding="utf-8"))


def variant(name, position=None, target=None, vfov=None, seed=None):
    d = json.loads(json.dumps(src))
    cam = d["render_cameras"][0]["camera"]
    if position:
        cam["position"], cam["target"], cam["vfov_deg"] = position, target, vfov
    cam["lens"]["dof"] = False
    d["camera"] = cam
    if seed is not None:
        d["render"]["seed"] = seed
    (DIR / f"bench_{name}.frproject").write_text(json.dumps(d), encoding="utf-8")
    print("creato", DIR / f"bench_{name}.frproject")


variant("intera", seed=12345)
variant("vicina", [0.03, 0.15, 0.42], [0.0, 0.1, 0.0], 22, seed=12345)
variant("ad")
