// Prepara assets/colori/RAL_Classic.json: codici RAL Classic con nome italiano, nome inglese e
// un valore sRGB INDICATIVO (non ufficiale RAL).
//
// Valori: confronto delle tabelle di Wikipedia in inglese ("List of RAL colours") e in tedesco
// ("RAL-Farbe"), scaricate con
//   curl "https://en.wikipedia.org/w/index.php?title=List_of_RAL_colours&action=raw" -o en.txt
//   curl "https://de.wikipedia.org/w/index.php?title=RAL-Farbe&action=raw" -o de.txt
// Se le due fonti differiscono al massimo di 20 su 255 per canale si usa la media; se differiscono
// di più (colori fluorescenti e blu molto saturi, che sRGB non rappresenta) si tiene il valore
// della tabella tedesca, più vicino ai campioni stampati; con una sola fonte, quella.
// I nomi italiani sono scritti qui (denominazioni d'uso comune in Italia).
//
//   node tools/colori/ral_classic.js <cartella con en.txt e de.txt> assets/colori/RAL_Classic.json
const fs = require('fs');
const [dir, out] = process.argv.slice(2);
const IT = {
  1000: 'Beige verdastro', 1001: 'Beige', 1002: 'Giallo sabbia', 1003: 'Giallo segnale', 1004: 'Giallo oro',
  1005: 'Giallo miele', 1006: 'Giallo polenta', 1007: 'Giallo narciso', 1011: 'Beige brunastro', 1012: 'Giallo limone',
  1013: 'Bianco perla', 1014: 'Avorio', 1015: 'Avorio chiaro', 1016: 'Giallo zolfo', 1017: 'Giallo zafferano',
  1018: 'Giallo zinco', 1019: 'Beige grigiastro', 1020: 'Giallo olivastro', 1021: 'Giallo navone', 1023: 'Giallo traffico',
  1024: 'Giallo ocra', 1026: 'Giallo brillante', 1027: 'Giallo curry', 1028: 'Giallo melone', 1032: 'Giallo ginestra',
  1033: 'Giallo dalia', 1034: 'Giallo pastello', 1035: 'Beige perlato', 1036: 'Oro perlato', 1037: 'Giallo sole',
  1039: 'Beige sabbia', 1040: 'Beige argilla',
  2000: 'Arancio giallastro', 2001: 'Arancio rossastro', 2002: 'Arancio sanguigno', 2003: 'Arancio pastello',
  2004: 'Arancio puro', 2005: 'Arancio brillante', 2007: 'Arancio chiaro brillante', 2008: 'Rosso arancio chiaro',
  2009: 'Arancio traffico', 2010: 'Arancio segnale', 2011: 'Arancio profondo', 2012: 'Arancio salmone',
  2013: 'Arancio perlato', 2017: 'Arancio RAL',
  3000: 'Rosso fuoco', 3001: 'Rosso segnale', 3002: 'Rosso carminio', 3003: 'Rosso rubino', 3004: 'Rosso porpora',
  3005: 'Rosso vino', 3007: 'Rosso nerastro', 3009: 'Rosso ossido', 3011: 'Rosso marrone', 3012: 'Rosso beige',
  3013: 'Rosso pomodoro', 3014: 'Rosa antico', 3015: 'Rosa chiaro', 3016: 'Rosso corallo', 3017: 'Rosato',
  3018: 'Rosso fragola', 3020: 'Rosso traffico', 3022: 'Rosso salmone', 3024: 'Rosso brillante',
  3026: 'Rosso chiaro brillante', 3027: 'Rosso lampone', 3028: 'Rosso puro', 3031: 'Rosso oriente',
  3032: 'Rosso rubino perlato', 3033: 'Rosa perlato',
  4001: 'Lilla rossastro', 4002: 'Viola rossastro', 4003: 'Viola erica', 4004: 'Viola bordeaux', 4005: 'Lilla bluastro',
  4006: 'Porpora traffico', 4007: 'Viola porpora', 4008: 'Viola segnale', 4009: 'Viola pastello', 4010: 'Telemagenta',
  4011: 'Viola perlato', 4012: 'Mora perlato',
  5000: 'Blu violaceo', 5001: 'Blu verdastro', 5002: 'Blu oltremare', 5003: 'Blu zaffiro', 5004: 'Blu nerastro',
  5005: 'Blu segnale', 5007: 'Blu brillante', 5008: 'Blu grigiastro', 5009: 'Blu azzurro', 5010: 'Blu genziana',
  5011: 'Blu acciaio', 5012: 'Blu luce', 5013: 'Blu cobalto', 5014: 'Blu colomba', 5015: 'Blu cielo',
  5017: 'Blu traffico', 5018: 'Blu turchese', 5019: 'Blu capri', 5020: 'Blu oceano', 5021: 'Blu acqua',
  5022: 'Blu notte', 5023: 'Blu distante', 5024: 'Blu pastello', 5025: 'Blu genziana perlato', 5026: 'Blu notte perlato',
  6000: 'Verde patina', 6001: 'Verde smeraldo', 6002: 'Verde foglia', 6003: 'Verde oliva', 6004: 'Verde bluastro',
  6005: 'Verde muschio', 6006: 'Oliva grigiastro', 6007: 'Verde bottiglia', 6008: 'Verde brunastro', 6009: 'Verde abete',
  6010: 'Verde erba', 6011: 'Verde reseda', 6012: 'Verde nerastro', 6013: 'Verde canna', 6014: 'Oliva giallastro',
  6015: 'Oliva nerastro', 6016: 'Verde turchese', 6017: 'Verde maggio', 6018: 'Verde giallastro', 6019: 'Verde biancastro',
  6020: 'Verde cromo', 6021: 'Verde pallido', 6022: 'Oliva brunastro', 6024: 'Verde traffico', 6025: 'Verde felce',
  6026: 'Verde opale', 6027: 'Verde chiaro', 6028: 'Verde pino', 6029: 'Verde menta', 6031: 'Verde bronzo',
  6032: 'Verde segnale', 6033: 'Turchese menta', 6034: 'Turchese pastello', 6035: 'Verde perlato',
  6036: 'Verde opale perlato', 6037: 'Verde puro', 6038: 'Verde brillante', 6039: 'Verde fibra', 6040: 'Oliva chiaro',
  7000: 'Grigio vaio', 7001: 'Grigio argento', 7002: 'Grigio olivastro', 7003: 'Grigio muschio', 7004: 'Grigio segnale',
  7005: 'Grigio topo', 7006: 'Grigio beige', 7008: 'Grigio kaki', 7009: 'Grigio verdastro', 7010: 'Grigio tenda',
  7011: 'Grigio ferro', 7012: 'Grigio basalto', 7013: 'Grigio brunastro', 7015: 'Grigio ardesia',
  7016: 'Grigio antracite', 7021: 'Grigio nerastro', 7022: 'Grigio ombra', 7023: 'Grigio calcestruzzo',
  7024: 'Grigio grafite', 7026: 'Grigio granito', 7030: 'Grigio pietra', 7031: 'Grigio bluastro', 7032: 'Grigio ghiaia',
  7033: 'Grigio cemento', 7034: 'Grigio giallastro', 7035: 'Grigio luce', 7036: 'Grigio platino', 7037: 'Grigio polvere',
  7038: 'Grigio agata', 7039: 'Grigio quarzo', 7040: 'Grigio finestra', 7042: 'Grigio traffico A',
  7043: 'Grigio traffico B', 7044: 'Grigio seta', 7045: 'Telegrigio 1', 7046: 'Telegrigio 2', 7047: 'Telegrigio 4',
  7048: 'Grigio topo perlato', 7050: 'Grigio mimetico',
  8000: 'Marrone verdastro', 8001: 'Marrone ocra', 8002: 'Marrone segnale', 8003: 'Marrone fango', 8004: 'Marrone rame',
  8007: 'Marrone capriolo', 8008: 'Marrone oliva', 8011: 'Marrone noce', 8012: 'Marrone rossiccio',
  8014: 'Marrone seppia', 8015: 'Marrone castagna', 8016: 'Marrone mogano', 8017: 'Marrone cioccolata',
  8019: 'Marrone grigiastro', 8022: 'Marrone nerastro', 8023: 'Marrone arancio', 8024: 'Marrone beige',
  8025: 'Marrone pallido', 8027: 'Marrone cuoio', 8028: 'Marrone terra', 8029: 'Rame perlato', 8031: 'Marrone sabbia',
  9001: 'Bianco crema', 9002: 'Bianco grigiastro', 9003: 'Bianco segnale', 9004: 'Nero segnale', 9005: 'Nero intenso',
  9006: 'Alluminio brillante', 9007: 'Alluminio grigiastro', 9010: 'Bianco puro', 9011: 'Nero grafite',
  9012: 'Bianco camera sterile', 9016: 'Bianco traffico', 9017: 'Nero traffico', 9018: 'Bianco papiro',
  9020: 'Bianco satinato', 9021: 'Nero catrame', 9022: 'Grigio chiaro perlato', 9023: 'Grigio scuro perlato',
};
// Nomi inglesi delle voci che la tabella inglese non ha.
const EN_EXTRA = {
  1039: 'Sand beige', 1040: 'Clay beige', 2017: 'RAL orange', 6003: 'Olive green', 6019: 'Pastel green',
  6031: 'Bronze green', 6039: 'Fibrous green', 6040: 'Light olive', 7050: 'Camouflage grey', 8027: 'Leather brown',
  8031: 'Sand brown', 9020: 'Silky white', 9021: 'Tar black',
};
const clean = s => s.replace(/\[\[([^\]|]*\|)?([^\]]*)\]\]/g, '$2').replace(/<[^>]+>/g, '').trim();
const en = fs.readFileSync(dir + '/en.txt', 'utf8');
const de = fs.readFileSync(dir + '/de.txt', 'utf8');
const E = {}, D = {};
for (const line of en.split(/\r?\n/)) {
  const m = line.match(/^\|RAL (\d{4})\|\|style="text-align:left"\|(.+?)\|\|\{\{[^}]*\}\}\s*\|\|#([0-9A-Fa-f]{6})/);
  if (m) E[m[1]] = { name: clean(m[2]), hex: m[3] };
}
const L = de.split(/\r?\n/);
for (let i = 0; i < L.length; i++) {
  const m = L[i].match(/^! RAL (\d{4})\s*$/);
  if (!m) continue;
  const c = (L[i + 2] || '').match(/[Cc]oltit\|([0-9A-Fa-f]{6})/);
  if (c) D[m[1]] = { hex: c[1] };
}
const rgb = h => [0, 2, 4].map(i => parseInt(h.slice(i, i + 2), 16));
const hex = c => '#' + c.map(v => v.toString(16).padStart(2, '0').toUpperCase()).join('');
const colors = [];
for (const code of Object.keys(IT).sort()) {
  const a = E[code] && rgb(E[code].hex), b = D[code] && rgb(D[code].hex);
  if (!a && !b) throw new Error('nessun valore per RAL ' + code);
  let c;
  if (a && b) {
    const d = Math.max(...a.map((v, k) => Math.abs(v - b[k])));
    c = d <= 20 ? a.map((v, k) => Math.round((v + b[k]) / 2)) : b;
  } else {
    c = a || b;
  }
  colors.push({ code: 'RAL ' + code, name: IT[code], name_en: (E[code] && E[code].name.replace(/\s*\/.*$/, '')) || EN_EXTRA[code] || '', hex: hex(c) });
}
const doc = {
  name: 'RAL Classic (valori indicativi)',
  note: 'Valori sRGB indicativi, non ufficiali RAL: media di fonti pubbliche (Wikipedia en/de). Per i colori esatti usare i campioni RAL o la libreria digitale ufficiale RAL. RAL è un marchio di RAL gGmbH.',
  colors,
};
fs.mkdirSync(require('path').dirname(out), { recursive: true });
fs.writeFileSync(out, JSON.stringify(doc, null, 1) + '\n');
console.log(colors.length, 'colori →', out);
