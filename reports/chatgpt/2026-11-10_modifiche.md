# Modifiche richieste — 10/11/2026

## Barra superiore della finestra: comparsa intermittente della barra standard di Windows

### Riscontro di Valerio

Durante l'uso di ForgeRender, occasionalmente la barra superiore della finestra, normalmente nera/coerente con l'interfaccia del programma, non viene visualizzata correttamente.

In quei momenti compare al suo posto (o sopra di essa) la barra/titolo standard di Windows, con un aspetto diverso da quello di ForgeRender e con i controlli della finestra riduci a icona, massimizza/ripristina e chiudi che risultano nascosti o coperti.

Il comportamento osservato:
- può capitare durante il normale utilizzo;
- muovendo leggermente la finestra, la barra nera di ForgeRender ricompare;
- il caso si nota soprattutto dopo aver ridotto ForgeRender a icona e averlo poi ripristinato: la barra torna nera e coerente con il programma.

### Obiettivo

La barra superiore deve essere stabile e ridisegnata correttamente in ogni stato della finestra. Non deve comparire la decorazione standard di Windows al posto della barra prevista dall'applicazione, né devono sparire o essere coperti i controlli di sistema.

### Indagine richiesta a Claude Code — prima di modificare

1. Ricostruire come è gestita oggi la barra del titolo e la decorazione della finestra:
   - eframe/egui e ViewportBuilder;
   - decorazioni native Windows (decorations, title bar, DWM/non-client area), se usate;
   - eventuale title bar personalizzata disegnata dall'interfaccia;
   - gestione dei pulsanti minimizza, massimizza/ripristino e chiudi e delle relative hit area.
2. Verificare se la finestra usa decorazioni native, una barra personalizzata o una soluzione ibrida. Non assumere che la barra sia un semplice pannello egui.
3. Analizzare il ciclo di vita della finestra su Windows: minimizzazione, ripristino, ridimensionamento, spostamento, cambio DPI e ricreazione/perdita del rendering surface.
4. Verificare se al ripristino vengono persi o non aggiornati lo stato di repaint, la dimensione/posizione della barra, il tema della title bar o le impostazioni DWM.
5. Cercare eventuali race condition o problemi di invalidazione/redraw tra winit, eframe e il rendering GPU (WGPU), distinguendo un problema della barra non-client di Windows da uno della UI personalizzata.
6. Riprodurre il difetto o predisporre una verifica mirata. Se non è riproducibile nell'ambiente di Claude, indicare chiaramente cosa è stato verificato e cosa resta da provare sul PC di Valerio.

### Criteri di accettazione

- La barra superiore resta nera e coerente con ForgeRender durante l'uso.
- Minimizzare e ripristinare la finestra non fa comparire la barra standard di Windows.
- Ridurre, massimizzare/ripristinare e chiudere restano sempre visibili e funzionanti.
- Spostare o ridimensionare la finestra non è necessario per far ricomparire la barra.
- Il comportamento è stabile anche dopo più cicli consecutivi di minimizzazione/ripristino.
- Nessuna regressione su trascinamento della finestra, doppio clic sulla barra per massimizzare, ridimensionamento dai bordi, DPI scaling e modalità Semplice/Avanzata.

### Vincoli

- Non cambiare subito lo stile della finestra né sostituire la barra con quella standard come workaround.
- Prima individuare la causa concreta e riferire il percorso attuale di gestione della finestra.
- Evitare workaround basati solo su repaint periodici o timer senza aver dimostrato che il problema sia un repaint mancante.
- Se la correzione richiede modifiche alla gestione delle decorazioni Windows, preservare correttamente hit testing, trascinamento, snap layout e pulsanti di sistema.
- Lavorare su sviluppo; non modificare direttamente main.
- Nel riscontro indicare file modificati, causa individuata, test e prove manuali realmente eseguiti, limiti e commit.

### Stato iniziale

- **Fatto osservato:** comportamento intermittente riferito da Valerio; la barra nera ricompare dopo un leggero spostamento della finestra e in particolare dopo minimizzazione/ripristino.
- **Ipotesi da verificare:** mancato ridisegno/aggiornamento della barra personalizzata oppure interazione errata tra decorazioni native Windows e barra egui. Non è ancora nota la causa.
- **Test eseguiti da ChatGPT:** nessuno.
- **Implementazione:** non richiesta/eseguita in questa nota; serve prima l'indagine di Claude Code.
