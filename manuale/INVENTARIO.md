# Inventario dei controlli dell'interfaccia

Generato da `tools/manuale/inventario.py` dai sorgenti: non modificare a mano.

**366 controlli** in 17 zone dell'interfaccia.

| Zona | Controlli | Senza testo fisso |
|---|---|---|
| Avanzata | 5 | 3 |
| Avanzata · Ambiente | 12 | 3 |
| Avanzata · Camera | 25 | 8 |
| Avanzata · Effetti | 21 | 6 |
| Avanzata · Luci | 25 | 8 |
| Avanzata · Materiali | 46 | 16 |
| Avanzata · Oggetto | 11 | 3 |
| Avanzata · Render | 29 | 13 |
| Barra dei menu | 77 | 13 |
| Benvenuto | 1 | 0 |
| Finestre di dialogo | 16 | 7 |
| Impostazioni | 19 | 5 |
| Libreria | 20 | 5 |
| Pannello Scena | 9 | 2 |
| Semplice | 20 | 9 |
| Toolbar | 21 | 11 |
| Viewport · strumenti | 9 | 0 |

## Avanzata

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | (variabile) | `show` | 20 | `avanzata.show` |
| campo numerico | (variabile) | `vec3_edit` | 43 | `avanzata.vec3_edit` |
| elenco a tendina | Camera: {current} | `camera_picker` | 2137 | `avanzata.camera_current` |
| scelta | Vista della viewport | `camera_picker` | 2142 | `avanzata.vista_della_viewport` |
| scelta | (variabile) | `camera_picker` | 2148 | `avanzata.camera_picker` |

## Avanzata · Ambiente

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | Uniforme | `environment` | 1127 | `avanzata_ambiente.uniforme` |
| scelta | Cielo a gradiente | `environment` | 1128 | `avanzata_ambiente.cielo_a_gradiente` |
| scelta | Mappa HDRI | `environment` | 1129 | `avanzata_ambiente.mappa_hdri` |
| colore | Intensità | `environment` | 1154 | `avanzata_ambiente.intensita` |
| cursore | (variabile) | `environment` | 1157 | `avanzata_ambiente.environment` |
| colore | Orizzonte | `environment` | 1167 | `avanzata_ambiente.orizzonte` |
| colore | Suolo | `environment` | 1170 | `avanzata_ambiente.suolo` |
| colore | Intensità | `environment` | 1173 | `avanzata_ambiente.intensita_2` |
| cursore | (variabile) | `environment` | 1176 | `avanzata_ambiente.environment_2` |
| cursore | Rotazione (°) | `environment` | 1192 | `avanzata_ambiente.rotazione` |
| cursore | (variabile) | `environment` | 1195 | `avanzata_ambiente.environment_3` |
| casella | Ambiente visibile come sfondo | `environment` | 1207 | `avanzata_ambiente.ambiente_visibile_come_sfondo` |

## Avanzata · Camera

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | Punta su oggetto | `camera` | 1624 | `avanzata_camera.punta_su_oggetto` |
| pulsante | (variabile) | `camera` | 1686 | `avanzata_camera.camera` |
| pulsante | Inquadra tutto | `camera` | 1698 | `avanzata_camera.inquadra_tutto` |
| pulsante | Inquadra selezione | `camera` | 1701 | `avanzata_camera.inquadra_selezione` |
| campo di testo | (variabile) | `render_cameras` | 2060 | `avanzata_camera.render_cameras` |
| pulsante | + Nuova camera dalla vista | `render_cameras` | 2107 | `avanzata_camera.nuova_camera_dalla_vista` |
| casella | Pilota | `render_cameras` | 2114 | `avanzata_camera.pilota` |
| scelta | Prospettica | `lens_controls` | 2182 | `avanzata_camera.prospettica` |
| scelta | Ortografica | `lens_controls` | 2186 | `avanzata_camera.ortografica` |
| casella | Quella che la prospettiva avrebbe alla distanza del bersaglio (segu… | `lens_controls` | 2198 | `avanzata_camera.quella_che_la_prospettiva_avrebbe_alla_d` |
| campo numerico |  m | `lens_controls` | 2208 | `avanzata_camera.m` |
| campo numerico |  mm | `lens_controls` | 2223 | `avanzata_camera.mm` |
| casella | (variabile) | `lens_controls` | 2254 | `avanzata_camera.lens_controls` |
| elenco a tendina | (variabile) | `lens_controls` | 2263 | `avanzata_camera.lens_controls_2` |
| scelta | (variabile) | `lens_controls` | 2269 | `avanzata_camera.lens_controls_3` |
| campo numerico | (variabile) | `lens_controls` | 2281 | `avanzata_camera.lens_controls_4` |
| campo numerico |  m | `lens_controls` | 2294 | `avanzata_camera.m_2` |
| scelta | Scegli col clic | `lens_controls` | 2302 | `avanzata_camera.scegli_col_clic` |
| pulsante | Sulla selezione | `lens_controls` | 2310 | `avanzata_camera.sulla_selezione` |
| elenco a tendina | (variabile) | `lens_controls` | 2347 | `avanzata_camera.lens_controls_5` |
| scelta | {n} lamelle | `lens_controls` | 2355 | `avanzata_camera.n_lamelle` |
| scelta | {n} lamelle | `lens_controls` | 2358 | `avanzata_camera.n_lamelle_2` |
| campo numerico | (variabile) | `lens_controls` | 2367 | `avanzata_camera.lens_controls_6` |
| campo numerico | orizz.  | `lens_controls` | 2382 | `avanzata_camera.orizz` |
| campo numerico | vert.  | `lens_controls` | 2389 | `avanzata_camera.vert` |

## Avanzata · Effetti

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Ghiaccio secco | `effects` | 1241 | `avanzata_effetti.ghiaccio_secco` |
| pulsante | (variabile) | `effects` | 1267 | `avanzata_effetti.effects` |
| casella | Attivo nel render (togliendo la spunta resta nella scena, spento) | `effects` | 1301 | `avanzata_effetti.attivo_nel_render_togliendo_la_spunta_re` |
| scelta | Seleziona: il gizmo sposta l'emettitore | `effects` | 1305 | `avanzata_effetti.seleziona_il_gizmo_sposta_l_emettitore` |
| campo di testo | Il gizmo sposta | `effects` | 1319 | `avanzata_effetti.il_gizmo_sposta` |
| scelta | (variabile) | `effects` | 1335 | `avanzata_effetti.effects_2` |
| campo numerico | Nuovo | `effects` | 1385 | `avanzata_effetti.nuovo` |
| cursore | (variabile) | `effects` | 1398 | `avanzata_effetti.effects_3` |
| cursore | Raggio sorgente (m) | `effects` | 1405 | `avanzata_effetti.raggio_sorgente_m` |
| cursore | Densità | `effects` | 1409 | `avanzata_effetti.densita` |
| cursore | Coefficiente di estinzione della nebbia piena (1/m): più alto = più… | `effects` | 1414 | `avanzata_effetti.coefficiente_di_estinzione_della_nebbia_` |
| cursore | Dimensione di sbuffi, filamenti e vortici | `effects` | 1420 | `avanzata_effetti.dimensione_di_sbuffi_filamenti_e_vortici` |
| cursore | Risalita | `effects` | 1425 | `avanzata_effetti.risalita` |
| cursore | (variabile) | `effects` | 1429 | `avanzata_effetti.effects_4` |
| cursore | Velocità con cui la nebbia si dissolve nell'aria (0 = mai) | `effects` | 1436 | `avanzata_effetti.velocita_con_cui_la_nebbia_si_dissolve_n` |
| colore | Intensità scattering | `effects` | 1441 | `avanzata_effetti.intensita_scattering` |
| cursore | Quanta luce la nebbia diffonde invece di assorbire (1 = bianca, 0 =… | `effects` | 1446 | `avanzata_effetti.quanta_luce_la_nebbia_diffonde_invece_di` |
| cursore | 0 = diffusione uniforme; positiva = più luminosa in controluce | `effects` | 1460 | `avanzata_effetti.0_diffusione_uniforme_positiva_piu_lumin` |
| casella | Non attraversa gli oggetti | `effects` | 1466 | `avanzata_effetti.non_attraversa_gli_oggetti` |
| scelta | (variabile) | `effects` | 1480 | `avanzata_effetti.effects_5` |
| cursore | (variabile) | `effects` | 1490 | `avanzata_effetti.effects_6` |

## Avanzata · Luci

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Sostituisce tutte le luci, adattandole alle dimensioni della scena | `lights` | 871 | `avanzata_luci.sostituisce_tutte_le_luci_adattandole_al` |
| pulsante | (variabile) | `lights` | 884 | `avanzata_luci.lights` |
| campo di testo | (variabile) | `lights` | 908 | `avanzata_luci.lights_2` |
| elenco a tendina | Tipo | `lights` | 918 | `avanzata_luci.tipo` |
| scelta | Area (rettangolare) | `lights` | 921 | `avanzata_luci.area_rettangolare` |
| scelta | Puntiforme | `lights` | 922 | `avanzata_luci.puntiforme` |
| scelta | Spot | `lights` | 923 | `avanzata_luci.spot` |
| scelta | Direzionale | `lights` | 924 | `avanzata_luci.direzionale` |
| scelta | Portale (finestra) | `lights` | 925 | `avanzata_luci.portale_finestra` |
| casella | Accesa | `lights` | 941 | `avanzata_luci.accesa` |
| campo numerico | Altezza (m) | `lights` | 956 | `avanzata_luci.altezza_m` |
| campo numerico | (variabile) | `lights` | 959 | `avanzata_luci.lights_3` |
| colore | Temperatura | `lights` | 965 | `avanzata_luci.temperatura` |
| campo numerico | (variabile) | `lights` | 984 | `avanzata_luci.lights_4` |
| campo numerico | Altezza (m) | `lights` | 990 | `avanzata_luci.altezza_m_2` |
| campo numerico | (variabile) | `lights` | 993 | `avanzata_luci.lights_5` |
| cursore | (variabile) | `lights` | 1009 | `avanzata_luci.lights_6` |
| cursore | 0 % = bordo netto, 100 % = sfuma dal centro (maniglia vuota nella v… | `lights` | 1021 | `avanzata_luci.0_bordo_netto_100_sfuma_dal_centro_manig` |
| casella | Limitato | `lights` | 1037 | `avanzata_luci.limitato` |
| campo numerico | (variabile) | `lights` | 1055 | `avanzata_luci.lights_7` |
| cursore | (variabile) | `lights` | 1065 | `avanzata_luci.lights_8` |
| casella | Proietta ombre | `lights` | 1087 | `avanzata_luci.proietta_ombre` |
| casella | Visibile alla camera | `lights` | 1090 | `avanzata_luci.visibile_alla_camera` |
| pulsante | Duplica | `lights` | 1093 | `avanzata_luci.duplica` |
| pulsante | Elimina | `lights` | 1094 | `avanzata_luci.elimina` |

## Avanzata · Materiali

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Salva | `library_save_controls` | 71 | `avanzata_materiali.salva` |
| pulsante | (variabile) | `library_save_controls` | 92 | `avanzata_materiali.library_save_controls` |
| pulsante | Ripristina | `library_save_controls` | 106 | `avanzata_materiali.ripristina` |
| campo di testo | Salva | `library_save_controls` | 123 | `avanzata_materiali.salva_2` |
| pulsante | Salva | `library_save_controls` | 125 | `avanzata_materiali.salva_3` |
| pulsante | Annulla | `library_save_controls` | 163 | `avanzata_materiali.annulla` |
| campo di testo | cerca… | `materials` | 229 | `avanzata_materiali.cerca` |
| scelta | (variabile) | `materials` | 274 | `avanzata_materiali.materials` |
| pulsante | Libreria con texture (ambientCG, Poly Haven)… | `materials` | 284 | `avanzata_materiali.libreria_con_texture_ambientcg_poly_have` |
| pulsante | Assegna alla selezione | `materials` | 296 | `avanzata_materiali.assegna_alla_selezione` |
| pulsante | Rimuovi assegnazione | `materials` | 305 | `avanzata_materiali.rimuovi_assegnazione` |
| campo di testo | (variabile) | `materials` | 317 | `avanzata_materiali.materials_2` |
| pulsante | Assegna ({}) | `materials` | 329 | `avanzata_materiali.assegna` |
| pulsante | Usa come predefinito per i componenti senza assegnazione | `materials` | 346 | `avanzata_materiali.usa_come_predefinito_per_i_componenti_se` |
| pulsante | Nuovo | `materials` | 355 | `avanzata_materiali.nuovo` |
| pulsante | Duplica | `materials` | 366 | `avanzata_materiali.duplica` |
| pulsante | Elimina | `materials` | 381 | `avanzata_materiali.elimina` |
| campo di testo | Scala e posizionamento texture | `materials` | 412 | `avanzata_materiali.scala_e_posizionamento_texture` |
| colore | (variabile) | `materials` | 437 | `avanzata_materiali.materials_3` |
| cursore | (variabile) | `materials` | 443 | `avanzata_materiali.materials_4` |
| cursore | (variabile) | `materials` | 449 | `avanzata_materiali.materials_5` |
| cursore | 0.5 = riflesso fisico derivato dall'IOR | `materials` | 455 | `avanzata_materiali.0_5_riflesso_fisico_derivato_dall_ior` |
| cursore | Transmission | `materials` | 460 | `avanzata_materiali.transmission` |
| cursore | Trasparenza dielettrica (vetro, liquidi) | `materials` | 464 | `avanzata_materiali.trasparenza_dielettrica_vetro_liquidi` |
| colore | (variabile) | `materials` | 471 | `avanzata_materiali.materials_6` |
| campo numerico |  cm | `materials` | 480 | `avanzata_materiali.cm` |
| campo numerico | (variabile) | `materials` | 489 | `avanzata_materiali.materials_7` |
| colore | Intensità emissione | `materials` | 497 | `avanzata_materiali.intensita_emissione` |
| campo numerico | Categoria | `materials` | 501 | `avanzata_materiali.categoria` |
| campo di testo | (variabile) | `materials` | 505 | `avanzata_materiali.materials_8` |
| casella | È una mappa di lucidità (gloss) | `materials` | 530 | `avanzata_materiali.e_una_mappa_di_lucidita_gloss` |
| casella | Formato DirectX | `materials` | 551 | `avanzata_materiali.formato_directx` |
| cursore | (variabile) | `materials` | 567 | `avanzata_materiali.materials_9` |
| scelta | Box (CAD) | `mapping_controls` | 591 | `avanzata_materiali.box_cad` |
| scelta | UV della mesh | `mapping_controls` | 595 | `avanzata_materiali.uv_della_mesh` |
| cursore | (variabile) | `mapping_controls` | 607 | `avanzata_materiali.mapping_controls` |
| campo numerico | (variabile) | `mapping_controls` | 635 | `avanzata_materiali.mapping_controls_2` |
| cursore | Quante volte la texture si ripete sull'intervallo UV 0–1 del modello | `mapping_controls` | 652 | `avanzata_materiali.quante_volte_la_texture_si_ripete_sull_i` |
| cursore | (variabile) | `mapping_controls` | 661 | `avanzata_materiali.mapping_controls_3` |
| campo numerico | U  | `mapping_controls` | 667 | `avanzata_materiali.u` |
| campo numerico | V  | `mapping_controls` | 670 | `avanzata_materiali.v` |
| pulsante | Usa la scala del displacement del file ({mm:.1} mm) | `mtlx_values` | 724 | `avanzata_materiali.usa_la_scala_del_displacement_del_file_m` |
| cursore | Dettaglio fine dell'ombreggiatura (non cambia la sagoma) | `relief_controls` | 744 | `avanzata_materiali.dettaglio_fine_dell_ombreggiatura_non_ca` |
| cursore | (variabile) | `relief_controls` | 752 | `avanzata_materiali.relief_controls` |
| cursore | (variabile) | `relief_controls` | 760 | `avanzata_materiali.relief_controls_2` |
| cursore | (variabile) | `relief_controls` | 784 | `avanzata_materiali.relief_controls_3` |

## Avanzata · Oggetto

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| campo numerico | {name}  | `axis_row` | 18 | `avanzata_oggetto.name` |
| pulsante | Duplica | `show` | 53 | `avanzata_oggetto.duplica` |
| pulsante | Elimina | `show` | 64 | `avanzata_oggetto.elimina` |
| pulsante | Appoggia sul pavimento | `show` | 74 | `avanzata_oggetto.appoggia_sul_pavimento` |
| pulsante | Centra nell'origine | `show` | 84 | `avanzata_oggetto.centra_nell_origine` |
| pulsante | Reimposta | `show` | 94 | `avanzata_oggetto.reimposta` |
| casella | Snap a contatto | `show` | 110 | `avanzata_oggetto.snap_a_contatto` |
| pulsante | (variabile) | `show` | 125 | `avanzata_oggetto.show` |
| casella | Mantieni proporzioni | `show` | 173 | `avanzata_oggetto.mantieni_proporzioni` |
| campo numerico | (variabile) | `show` | 186 | `avanzata_oggetto.show_2` |
| campo numerico | (variabile) | `show` | 196 | `avanzata_oggetto.show_3` |

## Avanzata · Render

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| elenco a tendina | Dispositivo | `render` | 1725 | `avanzata_render.dispositivo` |
| scelta | (variabile) | `render` | 1729 | `avanzata_render.render` |
| cursore | 0 = automatico (thread logici − 1, per lasciare reattiva l'interfac… | `render` | 1757 | `avanzata_render.0_automatico_thread_logici_1_per_lasciar` |
| scelta | (variabile) | `render` | 1776 | `avanzata_render.render_2` |
| campo numerico |  px | `render` | 1785 | `avanzata_render.px` |
| campo numerico |  px | `render` | 1789 | `avanzata_render.px_2` |
| scelta | (variabile) | `render` | 1797 | `avanzata_render.render_3` |
| scelta | Personalizzato | `render` | 1803 | `avanzata_render.personalizzato` |
| scelta | Illimitato | `render` | 1808 | `avanzata_render.illimitato` |
| campo numerico | campioni:  | `render` | 1820 | `avanzata_render.campioni` |
| casella | Arresto a soglia di rumore | `render` | 1838 | `avanzata_render.arresto_a_soglia_di_rumore` |
| cursore | (variabile) | `render` | 1855 | `avanzata_render.render_4` |
| campo numerico | (variabile) | `render` | 1868 | `avanzata_render.render_5` |
| cursore | Russian roulette da | `render` | 1881 | `avanzata_render.russian_roulette_da` |
| cursore | Clamp indiretto | `render` | 1884 | `avanzata_render.clamp_indiretto` |
| campo numerico | (variabile) | `render` | 1888 | `avanzata_render.render_6` |
| campo numerico | (variabile) | `render` | 1895 | `avanzata_render.render_7` |
| elenco a tendina | Tone mapping | `render` | 1901 | `avanzata_render.tone_mapping` |
| scelta | (variabile) | `render` | 1905 | `avanzata_render.render_8` |
| cursore | Esposizione (EV) | `render` | 1909 | `avanzata_render.esposizione_ev` |
| elenco a tendina | Modalità | `denoiser` | 1944 | `avanzata_render.modalita` |
| scelta | (variabile) | `denoiser` | 1948 | `avanzata_render.denoiser` |
| casella | Usa CUDA (GPU NVIDIA) | `denoiser` | 1960 | `avanzata_render.usa_cuda_gpu_nvidia` |
| elenco a tendina | (variabile) | `denoiser` | 1973 | `avanzata_render.denoiser_2` |
| scelta | (variabile) | `denoiser` | 1975 | `avanzata_render.denoiser_3` |
| campo numerico |  campioni | `denoiser` | 1990 | `avanzata_render.campioni_2` |
| cursore | (variabile) | `denoiser` | 2001 | `avanzata_render.denoiser_4` |
| casella | Preserva i dettagli (albedo e normali) | `denoiser` | 2013 | `avanzata_render.preserva_i_dettagli_albedo_e_normali` |
| scelta | (variabile) | `denoiser` | 2021 | `avanzata_render.denoiser_5` |

## Barra dei menu

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| menu | File | `show` | 17 | `barra_dei_menu.file` |
| menu | Modifica | `show` | 18 | `barra_dei_menu.modifica` |
| menu | Scena | `show` | 19 | `barra_dei_menu.scena` |
| menu | Oggetti | `show` | 20 | `barra_dei_menu.oggetti` |
| menu | Vista | `show` | 21 | `barra_dei_menu.vista` |
| menu | Rendering | `show` | 22 | `barra_dei_menu.rendering` |
| menu | Libreria | `show` | 23 | `barra_dei_menu.libreria` |
| comando | La mia libreria | `show` | 24 | `barra_dei_menu.la_mia_libreria` |
| comando | Scarica da ambientCG / Poly Haven… | `show` | 25 | `barra_dei_menu.scarica_da_ambientcg_poly_haven` |
| comando | Strumenti | `show` | 27 | `barra_dei_menu.strumenti` |
| menu | Strumenti | `show` | 29 | `barra_dei_menu.strumenti_2` |
| casella | (variabile) | `show` | 33 | `barra_dei_menu.show` |
| menu | Guida | `show` | 40 | `barra_dei_menu.guida` |
| pulsante | Scorciatoie da tastiera | `show` | 41 | `barra_dei_menu.scorciatoie_da_tastiera` |
| comando | Benvenuto e informazioni… | `show` | 46 | `barra_dei_menu.benvenuto_e_informazioni` |
| pulsante | (variabile) | `item_with` | 65 | `barra_dei_menu.item_with` |
| comando | (variabile) | `tab_item` | 86 | `barra_dei_menu.tab_item` |
| comando | Nuova scena vuota | `file_menu` | 90 | `barra_dei_menu.nuova_scena_vuota` |
| comando | Nuova scena di test | `file_menu` | 91 | `barra_dei_menu.nuova_scena_di_test` |
| comando | Progetti recenti | `file_menu` | 93 | `barra_dei_menu.progetti_recenti` |
| menu | Progetti recenti | `file_menu` | 95 | `barra_dei_menu.progetti_recenti_2` |
| pulsante | (variabile) | `file_menu` | 101 | `barra_dei_menu.file_menu` |
| comando | Salva progetto | `file_menu` | 108 | `barra_dei_menu.salva_progetto` |
| comando | Salva progetto con nome… | `file_menu` | 109 | `barra_dei_menu.salva_progetto_con_nome` |
| comando | Importa modello… | `file_menu` | 111 | `barra_dei_menu.importa_modello` |
| comando | Esporta immagine… | `file_menu` | 112 | `barra_dei_menu.esporta_immagine` |
| comando | Esci | `file_menu` | 114 | `barra_dei_menu.esci` |
| comando | Annulla | `edit_menu` | 118 | `barra_dei_menu.annulla` |
| comando | Ripeti | `edit_menu` | 119 | `barra_dei_menu.ripeti` |
| comando | Duplica | `edit_menu` | 121 | `barra_dei_menu.duplica` |
| comando | Elimina | `edit_menu` | 122 | `barra_dei_menu.elimina` |
| comando | Nascondi selezione | `edit_menu` | 124 | `barra_dei_menu.nascondi_selezione` |
| comando | Nascondi tutto il resto | `edit_menu` | 125 | `barra_dei_menu.nascondi_tutto_il_resto` |
| comando | Mostra tutto | `edit_menu` | 126 | `barra_dei_menu.mostra_tutto` |
| comando | Impostazioni… | `edit_menu` | 128 | `barra_dei_menu.impostazioni` |
| comando | Ripristina impostazioni predefinite | `edit_menu` | 129 | `barra_dei_menu.ripristina_impostazioni_predefinite` |
| comando | Prepara scena prodotto | `scene_menu` | 133 | `barra_dei_menu.prepara_scena_prodotto` |
| comando | Aspetto | `scene_menu` | 134 | `barra_dei_menu.aspetto` |
| comando | Aspetto | `scene_menu` | 135 | `barra_dei_menu.aspetto_2` |
| menu | Aspetto | `scene_menu` | 137 | `barra_dei_menu.aspetto_3` |
| pulsante | (variabile) | `scene_menu` | 141 | `barra_dei_menu.scene_menu` |
| comando | Ambiente e HDRI… | `scene_menu` | 150 | `barra_dei_menu.ambiente_e_hdri` |
| comando | Ambiente e HDRI… | `scene_menu` | 151 | `barra_dei_menu.ambiente_e_hdri_2` |
| comando | Ambiente e HDRI… | `scene_menu` | 153 | `barra_dei_menu.ambiente_e_hdri_3` |
| comando | Effetti e nebbia… | `scene_menu` | 154 | `barra_dei_menu.effetti_e_nebbia` |
| pulsante | (variabile) | `objects_menu` | 159 | `barra_dei_menu.objects_menu` |
| menu | Luce | `objects_menu` | 165 | `barra_dei_menu.luce` |
| pulsante | (variabile) | `objects_menu` | 167 | `barra_dei_menu.objects_menu_2` |
| comando | Posizione e dimensioni… | `objects_menu` | 174 | `barra_dei_menu.posizione_e_dimensioni` |
| comando | Materiali… | `objects_menu` | 175 | `barra_dei_menu.materiali` |
| comando | Luci… | `objects_menu` | 176 | `barra_dei_menu.luci` |
| comando | Inquadra tutto | `view_menu` | 180 | `barra_dei_menu.inquadra_tutto` |
| comando | Inquadra selezione | `view_menu` | 181 | `barra_dei_menu.inquadra_selezione` |
| comando | Viste standard | `view_menu` | 183 | `barra_dei_menu.viste_standard` |
| comando | Viste standard | `view_menu` | 184 | `barra_dei_menu.viste_standard_2` |
| menu | Viste standard | `view_menu` | 185 | `barra_dei_menu.viste_standard_3` |
| pulsante | (variabile) | `view_menu` | 187 | `barra_dei_menu.view_menu` |
| pulsante | (variabile) | `view_menu` | 197 | `barra_dei_menu.view_menu_2` |
| comando | Camera e obiettivo… | `view_menu` | 206 | `barra_dei_menu.camera_e_obiettivo` |
| comando | Avvia render | `render_menu` | 210 | `barra_dei_menu.avvia_render` |
| comando | Dispositivo di calcolo | `render_menu` | 211 | `barra_dei_menu.dispositivo_di_calcolo` |
| comando | Dispositivo di calcolo | `render_menu` | 212 | `barra_dei_menu.dispositivo_di_calcolo_2` |
| menu | Dispositivo di calcolo | `render_menu` | 214 | `barra_dei_menu.dispositivo_di_calcolo_3` |
| scelta | Scheda video (GPU) | `render_menu` | 218 | `barra_dei_menu.scheda_video_gpu` |
| scelta | Processore (CPU) | `render_menu` | 221 | `barra_dei_menu.processore_cpu` |
| menu | Qualità | `render_menu` | 228 | `barra_dei_menu.qualita` |
| pulsante | {} · {spp} campioni | `render_menu` | 233 | `barra_dei_menu.spp_campioni` |
| menu | Formato | `render_menu` | 242 | `barra_dei_menu.formato` |
| pulsante | (variabile) | `render_menu` | 246 | `barra_dei_menu.render_menu` |
| casella | (variabile) | `render_menu` | 259 | `barra_dei_menu.render_menu_2` |
| comando | Impostazioni di render e denoiser… | `render_menu` | 263 | `barra_dei_menu.impostazioni_di_render_e_denoiser` |
| comando | Esporta immagine… | `render_menu` | 265 | `barra_dei_menu.esporta_immagine_2` |
| scelta | Controllo IA | `ai_control` | 278 | `barra_dei_menu.controllo_ia` |
| elenco a tendina | (variabile) | `language_switch` | 300 | `barra_dei_menu.language_switch` |
| scelta | (variabile) | `language_switch` | 305 | `barra_dei_menu.language_switch_2` |
| scelta | Semplice | `mode_switch` | 324 | `barra_dei_menu.semplice` |
| scelta | Avanzata | `mode_switch` | 326 | `barra_dei_menu.avanzata` |

## Benvenuto

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Chiudi | `show` | 59 | `benvenuto.chiudi` |

## Finestre di dialogo

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| elenco a tendina | (variabile) | `import_options_window` | 95 | `finestre_di_dialogo.import_options_window` |
| scelta | (variabile) | `import_options_window` | 99 | `finestre_di_dialogo.import_options_window_2` |
| elenco a tendina | (variabile) | `import_options_window` | 104 | `finestre_di_dialogo.import_options_window_3` |
| scelta | (variabile) | `import_options_window` | 108 | `finestre_di_dialogo.import_options_window_4` |
| scelta | Bassa | `import_options_window` | 116 | `finestre_di_dialogo.bassa` |
| scelta | Media | `import_options_window` | 117 | `finestre_di_dialogo.media` |
| scelta | Alta | `import_options_window` | 118 | `finestre_di_dialogo.alta` |
| casella | Appoggia sul pavimento dopo l'importazione | `import_options_window` | 122 | `finestre_di_dialogo.appoggia_sul_pavimento_dopo_l_importazio` |
| cursore | (variabile) | `import_options_window` | 125 | `finestre_di_dialogo.import_options_window_5` |
| pulsante | Importa | `import_options_window` | 140 | `finestre_di_dialogo.importa` |
| pulsante | Annulla | `import_options_window` | 141 | `finestre_di_dialogo.annulla` |
| pulsante | Salva | `confirm_discard_window` | 195 | `finestre_di_dialogo.salva` |
| pulsante | Non salvare | `confirm_discard_window` | 198 | `finestre_di_dialogo.non_salvare` |
| pulsante | Annulla | `confirm_discard_window` | 201 | `finestre_di_dialogo.annulla_2` |
| pulsante | (variabile) | `dialogs` | 280 | `finestre_di_dialogo.dialogs` |
| pulsante | (variabile) | `dialogs` | 297 | `finestre_di_dialogo.dialogs_2` |

## Impostazioni

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | (variabile) | `window` | 58 | `impostazioni.window` |
| pulsante | Ripristina predefiniti | `window` | 62 | `impostazioni.ripristina_predefiniti` |
| scelta | (variabile) | `language` | 104 | `impostazioni.language` |
| elenco a tendina | (variabile) | `library` | 125 | `impostazioni.library` |
| scelta | (variabile) | `library` | 129 | `impostazioni.library_2` |
| scelta | Poly Haven | `library` | 135 | `impostazioni.poly_haven` |
| scelta | Poly Haven | `library` | 136 | `impostazioni.poly_haven_2` |
| scelta | Spento (consigliato) | `library` | 141 | `impostazioni.spento_consigliato` |
| scelta | Valore proposto (3 % del lato della texture) | `library` | 143 | `impostazioni.valore_proposto_3_del_lato_della_texture` |
| scelta | Dal file MaterialX della fonte, se c'è | `library` | 149 | `impostazioni.dal_file_materialx_della_fonte_se_c_e` |
| pulsante | Cambia… | `paths` | 177 | `impostazioni.cambia` |
| pulsante | Usa la predefinita | `paths` | 183 | `impostazioni.usa_la_predefinita` |
| pulsante | Apri | `paths` | 189 | `impostazioni.apri` |
| scelta | (variabile) | `rendering` | 218 | `impostazioni.rendering` |
| scelta | Campioni per pixel | `rendering` | 219 | `impostazioni.campioni_per_pixel` |
| campo numerico | 0 = progressivo senza limite | `rendering` | 223 | `impostazioni.0_progressivo_senza_limite` |
| campo numerico | 0 = automatico (thread logici − 1) | `rendering` | 227 | `impostazioni.0_automatico_thread_logici_1` |
| casella | Quando cambi materiali, luci o camera il rendering riparte da solo | `rendering` | 231 | `impostazioni.quando_cambi_materiali_luci_o_camera_il_` |
| casella | Mostra all'avvio | `about` | 240 | `impostazioni.mostra_all_avvio` |

## Libreria

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | La mia libreria | `window` | 507 | `libreria.la_mia_libreria` |
| scelta | Online (ambientCG, Poly Haven) | `window` | 508 | `libreria.online_ambientcg_poly_haven` |
| pulsante | Apri cartella | `window` | 511 | `libreria.apri_cartella` |
| pulsante | Aggiorna | `window` | 517 | `libreria.aggiorna` |
| campo di testo | cerca (nome, categoria, tag)… | `mine` | 551 | `libreria.cerca_nome_categoria_tag` |
| pulsante | Importa cartella… | `mine` | 559 | `libreria.importa_cartella` |
| pulsante | Importa zip… | `mine` | 574 | `libreria.importa_zip` |
| cursore |  px | `mine` | 629 | `libreria.px` |
| pulsante | Aggiorna categorie | `mine` | 641 | `libreria.aggiorna_categorie` |
| scelta | (variabile) | `mine` | 719 | `libreria.mine` |
| pulsante | Apri cartella | `material_card` | 903 | `libreria.apri_cartella_2` |
| pulsante | Elimina dalla libreria | `material_card` | 907 | `libreria.elimina_dalla_libreria` |
| scelta | (variabile) | `online_tab` | 920 | `libreria.online_tab` |
| scelta | Materiali | `online_tab` | 925 | `libreria.materiali` |
| scelta | (variabile) | `online_tab` | 926 | `libreria.online_tab_2` |
| campo di testo | es. wood, metal, studio… | `online_tab` | 929 | `libreria.es_wood_metal_studio` |
| pulsante | Cerca | `online_tab` | 934 | `libreria.cerca` |
| pulsante | Aggiorna catalogo | `online_tab` | 936 | `libreria.aggiorna_catalogo` |
| elenco a tendina | (variabile) | `online_tab` | 945 | `libreria.online_tab_3` |
| scelta | (variabile) | `online_tab` | 950 | `libreria.online_tab_4` |

## Pannello Scena

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| casella | Mostra/nascondi | `show` | 73 | `pannello_scena.mostra_nascondi` |
| scelta | (variabile) | `show` | 81 | `pannello_scena.show` |
| pulsante | Inquadra | `show` | 99 | `pannello_scena.inquadra` |
| pulsante | Nascondi | `show` | 103 | `pannello_scena.nascondi` |
| pulsante | Mostra tutto | `show` | 107 | `pannello_scena.mostra_tutto` |
| pulsante | Duplica (Ctrl+D) | `show` | 111 | `pannello_scena.duplica_ctrl_d` |
| pulsante | Elimina dal progetto (Canc) | `show` | 116 | `pannello_scena.elimina_dal_progetto_canc` |
| casella | Accendi/spegni | `show` | 221 | `pannello_scena.accendi_spegni` |
| scelta | (variabile) | `show` | 225 | `pannello_scena.show_2` |

## Semplice

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | (variabile) | `big_button` | 17 | `semplice.big_button` |
| pulsante | (variabile) | `big_button` | 22 | `semplice.big_button_2` |
| pulsante | (variabile) | `choice` | 46 | `semplice.choice` |
| pulsante | (variabile) | `choice` | 57 | `semplice.choice_2` |
| pulsante | Importa modello… | `product` | 85 | `semplice.importa_modello` |
| pulsante | Appoggia sulla base | `product` | 93 | `semplice.appoggia_sulla_base` |
| pulsante | Inquadra il prodotto | `product` | 101 | `semplice.inquadra_il_prodotto` |
| pulsante | Adatta la scena al prodotto | `product` | 109 | `semplice.adatta_la_scena_al_prodotto` |
| pulsante | (variabile) | `look` | 128 | `semplice.look` |
| pulsante | Prepara scena prodotto | `look` | 152 | `semplice.prepara_scena_prodotto` |
| pulsante | Annulla preset | `look` | 167 | `semplice.annulla_preset` |
| pulsante | Apri la libreria materiali | `materials` | 209 | `semplice.apri_la_libreria_materiali` |
| pulsante | (variabile) | `format_quality` | 226 | `semplice.format_quality` |
| pulsante | (variabile) | `format_quality` | 252 | `semplice.format_quality_2` |
| pulsante | Scheda video (GPU) | `format_quality` | 280 | `semplice.scheda_video_gpu` |
| pulsante | Processore (CPU) | `format_quality` | 296 | `semplice.processore_cpu` |
| pulsante | (variabile) | `render` | 324 | `semplice.render` |
| pulsante | Salva immagine… | `render` | 344 | `semplice.salva_immagine` |
| pulsante | Tutte le luci: {tip} | `light_colors` | 372 | `semplice.tutte_le_luci_tip` |
| colore | (variabile) | `light_colors` | 384 | `semplice.light_colors` |

## Toolbar

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Libreria | `show` | 17 | `toolbar.libreria` |
| pulsante | Libreria | `show` | 20 | `toolbar.libreria_2` |
| pulsante | (variabile) | `show` | 23 | `toolbar.show` |
| pulsante | (variabile) | `show` | 31 | `toolbar.show_2` |
| pulsante | Pannello Luci… | `show` | 37 | `toolbar.pannello_luci` |
| pulsante | (variabile) | `show` | 48 | `toolbar.show_3` |
| pulsante | Camera e obiettivo… | `show` | 54 | `toolbar.camera_e_obiettivo` |
| pulsante | (variabile) | `show` | 63 | `toolbar.show_4` |
| pulsante | Ambiente e HDRI… | `show` | 72 | `toolbar.ambiente_e_hdri` |
| pulsante | Apri la libreria materiali | `show` | 78 | `toolbar.apri_la_libreria_materiali` |
| pulsante | Materiali… | `show` | 82 | `toolbar.materiali` |
| pulsante | Stop | `show` | 89 | `toolbar.stop` |
| pulsante | Riavvia | `show` | 90 | `toolbar.riavvia` |
| pulsante | Esporta | `show` | 91 | `toolbar.esporta` |
| casella | (variabile) | `show` | 96 | `toolbar.show_5` |
| scelta | (variabile) | `show` | 103 | `toolbar.show_6` |
| pulsante | (variabile) | `cmd_button` | 145 | `toolbar.cmd_button` |
| pulsante | (variabile) | `cmd_button` | 148 | `toolbar.cmd_button_2` |
| pulsante | (variabile) | `render_button` | 160 | `toolbar.render_button` |
| pulsante | (variabile) | `dropdown` | 169 | `toolbar.dropdown` |
| pulsante | (variabile) | `tool_button` | 175 | `toolbar.tool_button` |

## Viewport · strumenti

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | Globale | `toolbar` | 1059 | `viewport_strumenti.globale` |
| scelta | Globale | `toolbar` | 1062 | `viewport_strumenti.globale_2` |
| scelta | Locale | `toolbar` | 1064 | `viewport_strumenti.locale` |
| menu | Snap (Ctrl) | `toolbar` | 1067 | `viewport_strumenti.snap_ctrl` |
| campo numerico | Spostamento:  | `toolbar` | 1070 | `viewport_strumenti.spostamento` |
| campo numerico | Rotazione:  | `toolbar` | 1076 | `viewport_strumenti.rotazione` |
| campo numerico | Scala: × | `toolbar` | 1082 | `viewport_strumenti.scala` |
| pulsante | Annulla | `toolbar` | 1090 | `viewport_strumenti.annulla` |
| pulsante | Ripeti | `toolbar` | 1097 | `viewport_strumenti.ripeti` |
