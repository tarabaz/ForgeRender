# Lingue dell'interfaccia / Interface languages

Ogni file `Lan_<nome>.json` è una lingua: `code`, `name` e la tabella `strings`
"testo italiano" → "traduzione". I testi del programma sono in italiano, quindi `Lan_ita.json`
ha la tabella vuota. Un testo che manca resta in italiano.

Per aggiungere una lingua copia `Lan_eng.json` come `Lan_spa.json` (per esempio) nella cartella
`lang` accanto a `forgerender.exe`, cambia `code` e `name` e traduci i valori. Lasciare uguali i
segnaposto tra graffe (`{}`, `{n}`, `{:.1}`): si possono spostare nella frase. La lingua si
sceglie in alto a sinistra o in Impostazioni → Lingua; un file nuovo si legge al prossimo avvio.

Each `Lan_<name>.json` file is a language: `code`, `name` and the `strings` table
"Italian text" → "translation". Keep the `{…}` placeholders (their order may change).
To add a language, copy `Lan_eng.json` next to `forgerender.exe` under `lang`, change `code`
and `name` and translate the values.
