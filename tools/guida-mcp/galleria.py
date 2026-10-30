"""Render d'esempio per la guida MCP, fatti con i comandi di Claude (ponte forgerender-mcp) e
sempre con il denoiser: aspetti di prepare_studio, viste della camera, una luce prima e dopo,
materiali creati.

Uso (Linux, Xvfb, denoiser in target/debug/oidn):
    Xvfb :95 -screen 0 1600x960x24 &
    python3 tools/guida-mcp/galleria.py            # tutto
    python3 tools/guida-mcp/galleria.py viste      # solo un gruppo: aspetti, viste, luce, materiali
Le immagini vanno in docs/img/mcp/esempi/.
"""
import json, os, subprocess, sys, time
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.environ.get("GUIDA_OUT", "/tmp/guida-mcp") + "/galleria"
DEST = os.path.join(ROOT, "docs", "img", "mcp", "esempi")
os.makedirs(OUT, exist_ok=True)
os.makedirs(DEST, exist_ok=True)
CONTACT = OUT + "/contatto.json"
env = dict(os.environ, LD_LIBRARY_PATH=os.path.join(ROOT, "target/debug/oidn"), DISPLAY=os.environ.get("DISPLAY", ":95"), FORGE_MCP_FILE=CONTACT,
           FORGE_SETTINGS_FILE=OUT + "/prefs.json", FORGE_LIBRARY_DIR=OUT + "/lib")
# Modalità Avanzata: nessun arresto a soglia di rumore (le zone scure arrivano ai campioni pieni).
json.dump({"show_splash": False, "ui_mode": "Advanced"}, open(OUT + "/prefs.json", "w"))
if os.path.exists(CONTACT):
    os.remove(CONTACT)
app = subprocess.Popen([os.path.join(ROOT, "target/debug/forgerender"), "--controllo-ia"], env=env,
                       stdout=subprocess.DEVNULL, stderr=open(OUT + "/app.log", "w"))
for _ in range(120):
    if os.path.exists(CONTACT):
        break
    time.sleep(0.5)
bridge = subprocess.Popen([os.path.join(ROOT, "target/debug/forgerender-mcp")], env=env,
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
n = 0


def call(name, args=None):
    global n
    n += 1
    msg = {"jsonrpc": "2.0", "id": n, "method": "tools/call", "params": {"name": name, "arguments": args or {}}}
    bridge.stdin.write(json.dumps(msg) + "\n")
    bridge.stdin.flush()
    r = json.loads(bridge.stdout.readline())["result"]
    text = r["content"][0]["text"]
    if r["isError"]:
        sys.exit(f"{name}: {text}")
    return text


def render(name, w, h, spp):
    call("set_render_settings", {"width": w, "height": h, "samples": spp, "device": "cpu", "denoise": True})
    path = f"{OUT}/{name}.png"
    if os.path.exists(path):
        os.remove(path)
    t = time.time()
    text = call("render", {"output": path, "max_wait_s": 600, "preview_px": 0})
    while "render finito" not in text:
        text = call("render_status", {"max_wait_s": 600, "preview_px": 0})
    print(f"{name}: {time.time() - t:.0f} s")
    return path


def save(src, dst, width=None, crop=None):
    im = Image.open(src).convert("RGB")
    if crop:  # ritaglio centrale (w, h): toglie i bordi della scena visti di sbieco
        x, y = (im.width - crop[0]) // 2, (im.height - crop[1]) // 2
        im = im.crop((x, y, x + crop[0], y + crop[1]))
    if width and im.width > width:
        im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    im.save(os.path.join(DEST, dst), quality=88, optimize=True)


bridge.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}}) + "\n")
bridge.stdin.flush()
bridge.stdout.readline()
info = json.loads(call("set_render_settings", {"denoise": True}))
if not info.get("denoiser_installato"):
    sys.exit("denoiser non installato: niente immagini sgranate nella guida")
W, H, SPP = 800, 450, 512
ZOOM = 0.72  # inquadratura più stretta di quella automatica: il prodotto riempie la foto


# Viste ruotate: più strette, altrimenti a 45° si vede oltre il bordo del fondale.
VIEW_ZOOM = {"tre_quarti": 0.47, "isometrica": 0.40}
# Aspetto per vista: nell'isometrica la piega del fondale curvo attraversa la foto, la tela no.
VIEW_LOOK = {"isometrica": "studio"}
# Solo alcune viste: VISTE=isometrica python3 galleria.py viste
ONLY_VIEWS = [v for v in os.environ.get("VISTE", "").split(",") if v]
# Gruppi da rifare (argomenti dello script); nessuno = tutti.
GROUPS = set(sys.argv[1:]) or {"aspetti", "viste", "luce", "materiali"}


def studio(look="studio", view=None):
    call("new_scene", {"kind": "studio"})
    call("prepare_studio", {"look": look})
    cam = {"zoom": VIEW_ZOOM.get(view, ZOOM)}
    if view:
        cam["view"] = view
    call("set_camera", cam)

# Aspetti di prepare_studio.
for look in ("studio", "chiaro", "scuro", "scenografico") if "aspetti" in GROUPS else ():
    studio(look)
    save(render(f"aspetto_{look}", W, H, SPP), f"aspetto_{look}.jpg")

# Viste della camera.
# Sfondo chiaro: fondale bianco su ambiente bianco, i bordi visti di sbieco non si notano.
for view in ("frontale", "tre_quarti", "isometrica") if "viste" in GROUPS else ():
    if ONLY_VIEWS and view not in ONLY_VIEWS:
        continue
    studio(VIEW_LOOK.get(view, "chiaro"), view=view)
    if view == "isometrica":
        # 10 % più grande e ritagliata: il filo della tela nell'angolo resta fuori.
        save(render(f"vista_{view}", W * 11 // 10, H * 11 // 10, SPP), f"vista_{view}.jpg", crop=(W, H))
    else:
        save(render(f"vista_{view}", W, H, SPP), f"vista_{view}.jpg")

# Luce calda a destra: prima e dopo.
if "luce" in GROUPS:
    studio()
    save(render("luce_prima", W, H, SPP), "luce_prima.jpg")
    call("add_light", {"type": "area", "side": "destra", "kelvin": 3000, "strength": 2})
    save(render("luce_dopo", W, H, SPP), "luce_dopo.jpg")

# Materiali creati da Claude.
if "materiali" in GROUPS:
    studio()
    call("create_material", {"name": "Metallo satinato", "color": "#c8c8cc", "metallic": 1, "roughness": 0.35, "targets": ["Sfera acciaio"]})
    call("create_material", {"name": "Plastica lucida blu", "color": "#1f4fbf", "metallic": 0, "roughness": 0.12, "targets": ["Box plastica"]})
    call("create_material", {"name": "Bronzo spazzolato", "color": "#b07a3c", "metallic": 1, "roughness": 0.45, "targets": ["Toro rame"]})
    save(render("materiali", W, H, SPP), "materiali.jpg")

bridge.stdin.close()
bridge.wait()
app.terminate()
app.wait()
if os.path.exists(CONTACT):
    os.remove(CONTACT)
print("fatto:", DEST)
