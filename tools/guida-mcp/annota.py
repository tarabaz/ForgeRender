"""Immagini della guida MCP dagli scatti di scatta.py: barre con i pulsanti evidenziati,
finestra con i numeri della legenda, registro dei comandi, render d'esempio.

Le posizioni dei pulsanti sono quelle dell'interfaccia attuale: se la barra cambia, aggiornale
in POS (le trovi cercando i pixel arancioni del pulsante "Controllo IA" acceso).
"""
import os
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
G = os.environ.get("GUIDA_OUT", "/tmp/guida-mcp")
OUT = os.path.join(ROOT, "docs", "img", "mcp")
BLUE, WHITE = (40, 150, 255), (255, 255, 255)
FONT = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 20)
POS = {
    "it": dict(btn=(336, 1, 412, 23), lab=(416, 1, 520, 23), light=(34, 712, 236, 734), status=(4, 737, 384, 758), bar_w=960),
    "en": dict(btn=(308, 1, 371, 23), lab=(375, 1, 473, 23), light=(34, 712, 232, 734), status=(4, 737, 336, 758), bar_w=900),
}


def badge(d, x, y, n):
    r = 14
    d.ellipse((x - r, y - r, x + r, y + r), fill=BLUE, outline=WHITE, width=2)
    w = d.textlength(str(n), font=FONT)
    d.text((x - w / 2, y - 12), str(n), fill=WHITE, font=FONT)


def bar(lang, src, out, boxes):
    p = POS[lang]
    im = Image.open(f"{G}/{lang}/{src}.png").convert("RGB").crop((0, 0, p["bar_w"], 26))
    im = im.resize((im.width * 2, im.height * 2), Image.LANCZOS)
    canvas = Image.new("RGB", (im.width, im.height + 8), (24, 24, 24))
    canvas.paste(im, (0, 4))
    d = ImageDraw.Draw(canvas)
    for k in boxes:
        x0, y0, x1, y1 = p[k]
        d.rounded_rectangle((x0 * 2 - 4, y0 * 2, x1 * 2 + 4, y1 * 2 + 8), radius=8, outline=BLUE, width=4)
    canvas.save(f"{OUT}/{lang}/{out}.png", optimize=True)


for lang in ("it", "en"):
    os.makedirs(f"{OUT}/{lang}", exist_ok=True)
    p = POS[lang]
    bar(lang, "01_spento", "barra_spento", ["btn"])
    bar(lang, "02_acceso", "barra_acceso", ["btn"])
    bar(lang, "03_collegato_render", "barra_controllato", ["btn", "lab"])
    im = Image.open(f"{G}/{lang}/03_collegato_render.png").convert("RGB").crop((0, 0, 1600, 812))
    d = ImageDraw.Draw(im)
    for k in ("btn", "lab", "light", "status"):
        x0, y0, x1, y1 = p[k]
        d.rounded_rectangle((x0 - 3, max(y0 - 1, 0), x1 + 3, y1 + 3), radius=6, outline=BLUE, width=3)
    for k, n, dx in (("btn", 1, -30), ("lab", 2, 40)):
        cx = (p[k][0] + p[k][2]) // 2
        d.line((cx, 27, cx + dx, 72), fill=BLUE, width=3)
        badge(d, cx + dx, 86, n)
    badge(d, p["light"][2] + 22, 723, 3)
    badge(d, p["status"][2] + 22, 747, 4)
    d.rounded_rectangle((296, 113, 1232, 641), radius=6, outline=BLUE, width=3)
    badge(d, 1212, 133, 5)
    im.save(f"{OUT}/{lang}/finestra_comandata.jpg", quality=86, optimize=True)
    log = Image.open(f"{G}/{lang}/03_collegato_render.png").convert("RGB").crop((0, 826, 430, 900))
    log.resize((log.width * 2, log.height * 2), Image.LANCZOS).save(f"{OUT}/{lang}/registro.png", optimize=True)
Image.open(f"{G}/it/render.png").convert("RGB").save(f"{OUT}/render_esempio.jpg", quality=88, optimize=True)
print("immagini in", OUT)
