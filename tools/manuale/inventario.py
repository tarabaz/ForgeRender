"""Inventario dei controlli dell'interfaccia (fase 1 del piano docs/PIANO_INTERFACCIA_E_MANUALE.md).

Legge i sorgenti di crates/forge_app/src/ui/*.rs (e l'elenco comandi in commands.rs) e scrive:
  manuale/inventario.json   un elemento per controllo: file, riga, funzione, tipo, testo, id proposto
  manuale/INVENTARIO.md     riepilogo per pannello, da rivedere a mano

È un'analisi del testo, non del codice compilato: trova i controlli scritti con un testo fisso
(tr("…"), tk("…") o una stringa). I controlli creati in un ciclo con testi variabili compaiono
con il testo "(variabile)" e vanno documentati per gruppo.

Uso: python3 tools/manuale/inventario.py
"""
import json, os, re, unicodedata

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
UI = os.path.join(ROOT, "crates", "forge_app", "src", "ui")
OUT = os.path.join(ROOT, "manuale")

# Tipo di controllo → espressione che lo riconosce in una riga.
KINDS = [
    ("menu", r"\.menu_button\(|menu_button\("),
    ("pulsante", r"\.button\(|egui::Button::(new|selectable)\(|big_button\(|tool_button\(|cmd_button\(|choice\("),
    ("casella", r"\.checkbox\("),
    ("scelta", r"\.selectable_value\(|\.selectable_label\(|\.radio_value\("),
    ("cursore", r"egui::Slider::new\("),
    ("campo numerico", r"egui::DragValue::new\("),
    ("elenco a tendina", r"egui::ComboBox::"),
    ("colore", r"color_edit_button|color_picker"),
    ("campo di testo", r"egui::TextEdit::|\.text_edit_singleline\("),
    ("comando", r"(item|tab_item|item_with)\(app, ui, (Cmd|InspectorTab)::"),
]
TEXT = re.compile(r'\b(?:tr|tk|tf)\("((?:[^"\\]|\\.)*)"|RichText::new\("((?:[^"\\]|\\.)*)"')
FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)")
PANEL = {
    "inspector.rs": "Avanzata",
    "simple.rs": "Semplice",
    "menubar.rs": "Barra dei menu",
    "quickbar.rs": "Toolbar",
    "tree.rs": "Pannello Scena",
    "transform.rs": "Avanzata · Oggetto",
    "central.rs": "Viewport",
    "gizmo.rs": "Viewport · strumenti",
    "navigator.rs": "Viewport · navigatore",
    "status.rs": "Barra di stato",
    "settings.rs": "Impostazioni",
    "library.rs": "Libreria",
    "splash.rs": "Benvenuto",
    "mod.rs": "Finestre di dialogo",
}
TABS = {
    "materials": "Materiali", "mapping_controls": "Materiali", "mtlx_values": "Materiali",
    "relief_controls": "Materiali", "texture_slot": "Materiali", "library_save_controls": "Materiali",
    "lights": "Luci", "environment": "Ambiente", "hdri_library": "Ambiente", "pick_hdri": "Ambiente",
    "effects": "Effetti", "camera": "Camera", "lens_controls": "Camera", "render_cameras": "Camera",
    "render": "Render", "denoiser": "Render",
}


def commands():
    """Cmd::Nome → (id, testo) dall'elenco unico dei comandi."""
    src = open(os.path.join(ROOT, "crates", "forge_app", "src", "commands.rs"), encoding="utf-8").read()
    rx = re.compile(r'Cmd::(\w+)(?:\([^)]*\))? => s\("([^"]+)", tk\("((?:[^"\\]|\\.)*)"\)')
    return {m.group(1): (m.group(2), m.group(3)) for m in rx.finditer(src)}


CMDS = None


def slug(s):
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode().lower()
    return re.sub(r"[^a-z0-9]+", "_", s).strip("_")[:40] or "x"


def scan(path):
    name = os.path.basename(path)
    lines = open(path, encoding="utf-8").read().split("\n")
    fn = "?"
    out = []
    for i, line in enumerate(lines):
        m = FN.match(line)
        if m:
            fn = m.group(1)
        if line.strip().startswith("//"):
            continue
        for kind, rx in KINDS:
            if re.search(rx, line):
                # Testo sulla stessa riga o nelle tre successive (chiamate spezzate da rustfmt).
                text = None
                for j in range(i, min(i + 4, len(lines))):
                    t = TEXT.search(lines[j])
                    if t:
                        text = t.group(1) or t.group(2)
                        break
                cmd = re.search(r"Cmd::(\w+)", line)
                if cmd and cmd.group(1) in CMDS and not text:
                    cid, text = CMDS[cmd.group(1)]
                panel = PANEL.get(name, name)
                if name == "inspector.rs" and fn in TABS:
                    panel = f"Avanzata · {TABS[fn]}"
                out.append({
                    "file": f"crates/forge_app/src/ui/{name}",
                    "riga": i + 1,
                    "funzione": fn,
                    "pannello": panel,
                    "tipo": kind,
                    "testo": text or "(variabile)",
                    "id": f"{slug(panel)}.{slug(text or fn)}",
                })
                break
    return out


def main():
    global CMDS
    CMDS = commands()
    items = []
    for f in sorted(os.listdir(UI)):
        if f.endswith(".rs"):
            items += scan(os.path.join(UI, f))
    # Id unici: a parità di testo nello stesso pannello si aggiunge un numero.
    seen = {}
    for it in items:
        n = seen.get(it["id"], 0)
        seen[it["id"]] = n + 1
        if n:
            it["id"] = f'{it["id"]}_{n + 1}'
    os.makedirs(OUT, exist_ok=True)
    json.dump(items, open(os.path.join(OUT, "inventario.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)

    by_panel = {}
    for it in items:
        by_panel.setdefault(it["pannello"], []).append(it)
    md = ["# Inventario dei controlli dell'interfaccia", "",
          "Generato da `tools/manuale/inventario.py` dai sorgenti: non modificare a mano.", "",
          f"**{len(items)} controlli** in {len(by_panel)} zone dell'interfaccia.", "",
          "| Zona | Controlli | Senza testo fisso |", "|---|---|---|"]
    for p in sorted(by_panel):
        v = by_panel[p]
        md.append(f"| {p} | {len(v)} | {sum(1 for x in v if x['testo'] == '(variabile)')} |")
    for p in sorted(by_panel):
        md += ["", f"## {p}", "", "| Tipo | Testo | Funzione | Riga | Id proposto |", "|---|---|---|---|---|"]
        for it in by_panel[p]:
            t = it["testo"].replace("|", "\\|").replace("\\n", " ")
            if len(t) > 70:
                t = t[:67] + "…"
            md.append(f"| {it['tipo']} | {t} | `{it['funzione']}` | {it['riga']} | `{it['id']}` |")
    open(os.path.join(OUT, "INVENTARIO.md"), "w", encoding="utf-8").write("\n".join(md) + "\n")
    print(f"{len(items)} controlli → manuale/inventario.json, manuale/INVENTARIO.md")


if __name__ == "__main__":
    main()
