---
id: cam.tiltshift
titolo: Tilt-shift
parole: tilt-shift, tilt shift, miniatura, plastico, fascia nitida, fascia sfocata, fuoco selettivo
ui: panel.camera.tiltshift
correlate: cam.dof, cam.obiettivo
---
# Tilt-shift

**Dove si trova:** **Scena → Camera** → **Tilt-shift**.

**A cosa serve:** una fascia nitida sull'immagine finale con lo sfocato che cresce ai due lati,
come gli obiettivi tilt-shift che fanno sembrare le scene vere dei plastici. È **separato dalla
profondità di campo**: diaframma, focale e messa a fuoco non cambiano, e con tutti e due accesi la
fascia si somma allo sfocato dell'obiettivo. Accenderlo riavvia il render (serve la profondità di
ogni pixel); tutti gli altri valori si vedono subito sull'immagine, senza rifare il render.

| Controllo | Valori | Cosa fa |
|---|---|---|
| Posizione | −0,5 – 1,5 | Dove passa la fascia nitida: 0 = bordo in alto, 0,5 = centro, 1 = bordo in basso (sinistra / destra se la fascia è verticale) |
| Angolo | −90° – 90° | 0° = fascia orizzontale, 90° = verticale, in mezzo inclinata; pulsanti **Orizzontale** e **Verticale** |
| Larghezza nitida | 0 – 1 | Larghezza della fascia del tutto nitida, in frazione dell'altezza dell'immagine (0 = solo una linea) |
| Transizione sopra / sotto | 0 – 1 | Spazio per passare da nitido a sfocato del tutto, su ogni lato (0 = passaggio netto); **Transizioni uguali** copia l'una sull'altra |
| Sfocatura | 0 – 5 % | Sfocatura massima ai bordi, come raggio in percentuale della larghezza dell'immagine |
| Rispetta la profondità | sì / no | Un oggetto vicino e nitido non sbava sullo sfondo sfocato (niente aloni ai bordi) |

Lo sfocato si raccoglie da copie dell'immagine via via ridotte, così i riflessi luminosi diventano
uno sfocato morbido invece di copie ripetute. Funziona allo stesso modo su CPU e GPU.
