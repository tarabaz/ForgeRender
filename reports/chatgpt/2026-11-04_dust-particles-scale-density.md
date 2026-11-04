# Richiesta Valerio → Claude Code — 04/11/2026
## Pulviscolo: quantità molto più basse e particelle fino a 3× più grandi

Questa richiesta integra il problema osservato nella scena demo con sfere e anelli: attivando il pulviscolo, anche con la quantità al minimo, il fascio luminoso sembra interrompersi prima di raggiungere il piano del tavolo. Non assumere ancora che la causa sia questa regolazione: verificare separatamente il trasporto della luce e il volume.

### Obiettivo visivo

Voglio poter simulare particelle sospese nell'aria, visibili quando attraversate dai fasci luminosi e capaci di proiettare **piccole ombre/sagome nei fasci** (effetto pulviscolo cinematografico), senza trasformare il volume in una nebbia compatta e senza oscurare/interrompere il fascio prima che raggiunga le superfici.

### Modifiche richieste

1. **Quantità/densità**
   - Ridurre sensibilmente la quantità minima ottenibile con lo slider attuale e, se necessario, ampliare verso il basso il range.
   - Deve essere possibile avere pochissime particelle isolate e ben distanziate, non una nube continua.
   - Verificare che il valore minimo non produca un fascio eccessivamente opaco né una perdita evidente di illuminazione sul piano colpito.
   - Valutare un default più leggero, ma preservare i progetti esistenti e non cambiare i valori salvati senza migrazione esplicita.

2. **Dimensione delle particelle**
   - Ampliare il range dello slider in modo che la dimensione massima raggiungibile sia **almeno 3 volte quella massima attuale**.
   - Il controllo deve consentire particelle più grandi e riconoscibili, utili per creare piccole sagome/ombre nei fasci di luce.
   - Verificare che l'aumento dimensionale non trasformi le particelle in macchie/volumi innaturali e che la distribuzione resti controllabile.

3. **Interazione quantità × dimensione**
   - Quantità e dimensione devono essere regolabili indipendentemente: poche particelle grandi devono essere possibili senza dover aumentare la densità.
   - Verificare l'effetto combinato su visibilità, scattering, attenuazione e ombre. Se il modello attuale non permette alle particelle di proiettare ombre, chiarirlo e proporre la soluzione fisicamente/visivamente più coerente (ad esempio particelle opache o semi-opache con scattering controllato), senza simulare ombre finte non coerenti.

### Diagnostica obbligatoria sul fascio che non arriva a terra

Riprodurre la scena di riferimento e confrontare:
- pulviscolo disattivato;
- pulviscolo alla quantità minima attuale;
- quantità molto bassa con particelle grandi;
- quantità bassa con particelle piccole;
- scena equivalente senza geometria che possa intercettare il fascio.

Misurare o visualizzare l'attenuazione lungo il fascio e verificare se il problema dipende da densità, dimensione, assorbimento/scattering, limiti del volume o intersezioni con la geometria. Il fascio deve poter raggiungere il piano illuminato anche quando il pulviscolo è presente, salvo attenuazione fisicamente motivata.

### Compatibilità e consegna

- Ispezionare prima il codice effettivo del pulviscolo e i percorsi CPU/GPU.
- Mantenere controlli coerenti tra CPU e GPU.
- Verificare serializzazione e retrocompatibilità dei progetti.
- Aggiornare UI italiana/inglese e documentazione.
- Aggiungere test per range/clamp e per i casi limite; produrre render comparativi della scena.
- Riportare file modificati, commit, comandi e test realmente eseguiti. Distinguere ciò che è misurato da ciò che resta da verificare.

**Stato:** richiesta da valutare e implementare; non è una modifica già realizzata. ChatGPT non ha eseguito build, test o render.
