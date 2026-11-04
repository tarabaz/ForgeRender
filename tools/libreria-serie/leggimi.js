// Scrive il LEGGIMI delle librerie (compresa e aggiuntiva) con fonte e licenza di ogni materiale.
const fs = require('fs');
const [tabella, sostituti, outInc, outExt] = process.argv.slice(2);
const t = JSON.parse(fs.readFileSync(tabella, 'utf8'));
const sost = {};
for (const l of fs.readFileSync(sostituti, 'utf8').split(/\r?\n/)) {
  const p = l.trim().split(' ');
  if (p.length < 4 || p[0].startsWith('#')) continue;
  sost[p[2].replace(/_/g, ' ') + '/' + p[3]] = p[0] === 'ph'
    ? `[Poly Haven ${p[1]}](https://polyhaven.com/a/${p[1]})`
    : `[ambientCG ${p[1]}](https://ambientcg.com/a/${p[1]})`;
}
// Materiali Poly Haven arrivati dagli zip originali con un altro nome.
const ph = { 'lib-Brown-Floor-Tilles-2K': 'brown_floor_tiles', 'lib-Denim-Fabric-02-2K': 'denmin_fabric_02', 'Raso': 'crepe_satin', 'liuta': 'hessian_230', 'liuta-fine': 'hessian_380' };
function source(src) {
  if (src.startsWith('nuovo:')) return 'ForgeRender (parametri, senza mappe)';
  if (src.startsWith('sost:')) return sost[src.slice(5)] || '?';
  const stem = src.split('/')[1];
  let id = ph[stem];
  if (!id && stem.startsWith('lib-')) id = stem.slice(4).replace(/-2K$/, '').replace(/-/g, '_').toLowerCase();
  if (id) return `[Poly Haven ${id}](https://polyhaven.com/a/${id})`;
  return 'ForgeRender (parametri, senza mappe)';
}
function write(out, included, title, intro) {
  const rows = t.materiali.filter(m => m[4] === included).map(m => {
    const cat = t.categorie[m[3]];
    return `| ${m[3]} ${cat} | \`${m[1]}\` | ${m[2]} | ${source(m[0])} |`;
  });
  const txt = `# ${title}\n\n${intro}\n\n| Categoria | Id | Nome | Fonte |\n|---|---|---|---|\n${rows.join('\n')}\n`;
  fs.writeFileSync(out, txt);
  console.log(out, rows.length);
}
const comune = `Una cartella per categoria (\`01 Metalli\`, \`02 Anodizzati\`…) e dentro una cartella per
materiale con \`materiale.json\` e le mappe, lo stesso formato degli zip di "Esporta zip…".
Si rifanno con l'esempio \`libreria_serie\` di \`forge_render_core\` seguendo
\`tools/libreria-serie/tabella.json\` (vedi \`tools/libreria-serie/LEGGIMI.md\`).

Licenze: Poly Haven e ambientCG sono CC0 (pubblico dominio, liberi anche per uso
commerciale); i materiali senza mappe sono parametri di ForgeRender. Nessun materiale
BlenderKit: la loro licenza non ne permette la ridistribuzione.`;
write(outInc, true, 'Materiali compresi nel programma',
  `Libreria di serie di ForgeRender: tutti i materiali senza mappe e al massimo cinque con mappe
per categoria, con le mappe ridotte a 1024 px (il programma resta leggero). La cartella \`_vecchi\`
contiene le mappe dei predefiniti fino alla 0.37, solo per ritrovarle nei progetti salvati.

${comune}`);
write(outExt, false, 'Pacchetto aggiuntivo dei materiali',
  `Materiali con mappe a 2K non compresi nel programma, da scaricare a parte. Si installano
copiando questa cartella come \`materiali-aggiuntivi\` accanto a \`forgerender.exe\` (oppure in
\`assets/materiali-aggiuntivi\` nella cartella del codice, o indicandola con la variabile
\`FORGE_EXTRA_MATERIALS_DIR\`): al riavvio i materiali compaiono nella libreria di serie.

${comune}`);
