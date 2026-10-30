"""Client MCP minimo: avvia forgerender --mcp e invia una sequenza di comandi."""
import json, subprocess, sys, time, base64, os
exe, out_dir = sys.argv[1], sys.argv[2]
env = dict(os.environ)
p = subprocess.Popen(["xvfb-run", "-a", "-s", "-screen 0 1600x900x24", "sh", "-c", exe + " --mcp " + " ".join(sys.argv[4:]) + " 2>" + out_dir + "/stderr.log"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, env=env)
nid = 0
def rpc(method, params=None, notify=False):
    global nid
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None: msg["params"] = params
    if not notify:
        nid += 1; msg["id"] = nid
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
    if notify: return None
    line = p.stdout.readline()
    if not line: raise SystemExit("nessuna risposta (stdout chiuso)")
    return json.loads(line)
def tool(name, args=None):
    t = time.time()
    r = rpc("tools/call", {"name": name, "arguments": args or {}})
    res = r.get("result", r)
    texts = [c["text"] for c in res.get("content", []) if c["type"] == "text"]
    imgs = [c for c in res.get("content", []) if c["type"] == "image"]
    print(f"== {name} ({time.time()-t:.1f}s) err={res.get('isError')} img={len(imgs)}")
    print((texts[0] if texts else json.dumps(r))[:900])
    for k, im in enumerate(imgs):
        open(f"{out_dir}/{name}_{k}.png", "wb").write(base64.b64decode(im["data"]))
    return res
r = rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}})
print("init:", r["result"]["serverInfo"], r["result"]["protocolVersion"])
rpc("notifications/initialized", notify=True)
tl = rpc("tools/list")["result"]["tools"]
print("strumenti:", len(tl), [t["name"] for t in tl])
steps = json.load(open(sys.argv[3]))
for name, args in steps:
    tool(name, args)
time.sleep(4)
p.stdin.close()
try: p.wait(timeout=20)
except subprocess.TimeoutExpired: p.kill(); print("terminato a forza")
print("exit", p.returncode)
