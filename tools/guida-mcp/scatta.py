"""Screenshot di ForgeRender per la guida MCP (docs/MCP.md), in una lingua: it o en.

Uso (Linux, con Xvfb e ImageMagick):
    Xvfb :95 -screen 0 1600x960x24 &
    cargo build -p forge_app -p forge_mcp_bridge
    python3 tools/guida-mcp/scatta.py it && python3 tools/guida-mcp/scatta.py en
    python3 tools/guida-mcp/annota.py
Gli scatti grezzi vanno in $GUIDA_OUT (predefinito /tmp/guida-mcp), le immagini finali in
docs/img/mcp/.
"""
import json, os, subprocess, sys, time
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
L = sys.argv[1]
OUT = os.environ.get("GUIDA_OUT", "/tmp/guida-mcp")
D = OUT + "/" + L
os.makedirs(D, exist_ok=True)
APP = os.path.join(ROOT, "target/debug/forgerender")
BRIDGE = os.path.join(ROOT, "target/debug/forgerender-mcp")
CONTACT = D + "/contatto.json"
env = dict(os.environ, DISPLAY=":95", FORGE_MCP_FILE=CONTACT, FORGE_SETTINGS_FILE=D + "/prefs.json", FORGE_LIBRARY_DIR=D + "/lib")
json.dump({"show_splash": False, "language": L, "ui_mode": "Simple"}, open(D + "/prefs.json", "w"))
def shot(name):
    subprocess.run(["import", "-window", "root", "-display", ":95", D + "/" + name + ".png"], check=True)
    print("scatto", name)
def app(*args):
    p = subprocess.Popen([APP, *args], env=env, stdout=subprocess.DEVNULL, stderr=open(D + "/app.log", "a"))
    return p
# 1. controllo spento
if os.path.exists(CONTACT): os.remove(CONTACT)
p = app(); time.sleep(12); shot("01_spento"); p.terminate(); p.wait(); time.sleep(1)
# 2. controllo acceso, nessuno collegato
p = app("--controllo-ia")
for _ in range(60):
    if os.path.exists(CONTACT): break
    time.sleep(0.5)
time.sleep(8); shot("02_acceso")
# 3. ponte collegato: studio, luce calda, materiale, render
b = subprocess.Popen([BRIDGE], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
n = 0
def call(name, args, wait=True):
    global n
    n += 1
    b.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": "tools/call", "params": {"name": name, "arguments": args}}) + "\n"); b.stdin.flush()
    if wait:
        r = json.loads(b.stdout.readline())
        t = r["result"]["content"][0]["text"]
        print(name, "errore" if r["result"]["isError"] else "ok", t[:120].replace("\n", " "))
        return r
b.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}}) + "\n"); b.stdin.flush(); b.stdout.readline()
call("prepare_studio", {"look": "studio"})
call("add_light", {"type": "area", "side": "destra", "kelvin": 3200, "name": "Luce calda" if L == "it" else "Warm light"})
call("apply_material", {"material": "acciaio-satinato", "targets": ["Sfera acciaio"]})
call("set_render_settings", {"width": 960, "height": 540, "samples": 160, "device": "cpu", "denoise": False})
out = D + "/render.png"
if os.path.exists(out): os.remove(out)
call("render", {"output": out, "max_wait_s": 20}, wait=False)
time.sleep(12); shot("03_collegato_render")
r = json.loads(b.stdout.readline()); print("render:", r["result"]["content"][0]["text"][:100].replace("\n", " "))
while '"esito": "render finito' not in r["result"]["content"][0]["text"]:
    r = call("render_status", {"max_wait_s": 30, "preview_px": 0})
time.sleep(3); shot("04_collegato_finito")
b.stdin.close(); b.wait(); time.sleep(4); shot("05_scollegato")
p.terminate(); p.wait()
if os.path.exists(CONTACT): os.remove(CONTACT)
