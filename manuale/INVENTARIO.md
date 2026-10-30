# Inventario dei controlli dell'interfaccia

Generato da `tools/manuale/inventario.py` dai sorgenti: non modificare a mano.

**376 controlli** in 18 zone dell'interfaccia.

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
| Barra dei menu | 78 | 33 |
| Benvenuto | 1 | 0 |
| Finestre di dialogo | 16 | 7 |
| Impostazioni | 19 | 5 |
| Libreria | 20 | 5 |
| Pannello Scena | 9 | 2 |
| Semplice | 20 | 9 |
| Toolbar | 21 | 11 |
| Viewport · strumenti | 9 | 0 |
| manual.rs | 9 | 4 |

## Avanzata

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | (variabile) | `show` | 21 | `avanzata.show` |
| campo numerico | (variabile) | `vec3_edit` | 44 | `avanzata.vec3_edit` |
| elenco a tendina | Camera: {current} | `camera_picker` | 2150 | `avanzata.camera_current` |
| scelta | Vista della viewport | `camera_picker` | 2155 | `avanzata.vista_della_viewport` |
| scelta | (variabile) | `camera_picker` | 2161 | `avanzata.camera_picker` |

## Avanzata · Ambiente

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | Uniforme | `environment` | 1137 | `avanzata_ambiente.uniforme` |
| scelta | Cielo a gradiente | `environment` | 1138 | `avanzata_ambiente.cielo_a_gradiente` |
| scelta | Mappa HDRI | `environment` | 1139 | `avanzata_ambiente.mappa_hdri` |
| colore | Intensità | `environment` | 1164 | `avanzata_ambiente.intensita` |
| cursore | (variabile) | `environment` | 1167 | `avanzata_ambiente.environment` |
| colore | Orizzonte | `environment` | 1177 | `avanzata_ambiente.orizzonte` |
| colore | Suolo | `environment` | 1180 | `avanzata_ambiente.suolo` |
| colore | Intensità | `environment` | 1183 | `avanzata_ambiente.intensita_2` |
| cursore | (variabile) | `environment` | 1186 | `avanzata_ambiente.environment_2` |
| cursore | Rotazione (°) | `environment` | 1202 | `avanzata_ambiente.rotazione` |
| cursore | (variabile) | `environment` | 1205 | `avanzata_ambiente.environment_3` |
| casella | Ambiente visibile come sfondo | `environment` | 1217 | `avanzata_ambiente.ambiente_visibile_come_sfondo` |

## Avanzata · Camera

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| scelta | Punta su oggetto | `camera` | 1634 | `avanzata_camera.punta_su_oggetto` |
| pulsante | (variabile) | `camera` | 1696 | `avanzata_camera.camera` |
| pulsante | Inquadra tutto | `camera` | 1708 | `avanzata_camera.inquadra_tutto` |
| pulsante | Inquadra selezione | `camera` | 1711 | `avanzata_camera.inquadra_selezione` |
| campo di testo | (variabile) | `render_cameras` | 2073 | `avanzata_camera.render_cameras` |
| pulsante | + Nuova camera dalla vista | `render_cameras` | 2120 | `avanzata_camera.nuova_camera_dalla_vista` |
| casella | Pilota | `render_cameras` | 2127 | `avanzata_camera.pilota` |
| scelta | Prospettica | `lens_controls` | 2195 | `avanzata_camera.prospettica` |
| scelta | Ortografica | `lens_controls` | 2199 | `avanzata_camera.ortografica` |
| casella | Quella che la prospettiva avrebbe alla distanza del bersaglio (segu… | `lens_controls` | 2211 | `avanzata_camera.quella_che_la_prospettiva_avrebbe_alla_d` |
| campo numerico |  m | `lens_controls` | 2221 | `avanzata_camera.m` |
| campo numerico |  mm | `lens_controls` | 2236 | `avanzata_camera.mm` |
| casella | (variabile) | `lens_controls` | 2267 | `avanzata_camera.lens_controls` |
| elenco a tendina | (variabile) | `lens_controls` | 2276 | `avanzata_camera.lens_controls_2` |
| scelta | (variabile) | `lens_controls` | 2282 | `avanzata_camera.lens_controls_3` |
| campo numerico | (variabile) | `lens_controls` | 2294 | `avanzata_camera.lens_controls_4` |
| campo numerico |  m | `lens_controls` | 2307 | `avanzata_camera.m_2` |
| scelta | Scegli col clic | `lens_controls` | 2315 | `avanzata_camera.scegli_col_clic` |
| pulsante | Sulla selezione | `lens_controls` | 2323 | `avanzata_camera.sulla_selezione` |
| elenco a tendina | (variabile) | `lens_controls` | 2360 | `avanzata_camera.lens_controls_5` |
| scelta | {n} lamelle | `lens_controls` | 2368 | `avanzata_camera.n_lamelle` |
| scelta | {n} lamelle | `lens_controls` | 2371 | `avanzata_camera.n_lamelle_2` |
| campo numerico | (variabile) | `lens_controls` | 2380 | `avanzata_camera.lens_controls_6` |
| campo numerico | orizz.  | `lens_controls` | 2395 | `avanzata_camera.orizz` |
| campo numerico | vert.  | `lens_controls` | 2402 | `avanzata_camera.vert` |

## Avanzata · Effetti

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Ghiaccio secco | `effects` | 1251 | `avanzata_effetti.ghiaccio_secco` |
| pulsante | (variabile) | `effects` | 1277 | `avanzata_effetti.effects` |
| casella | Attivo nel render (togliendo la spunta resta nella scena, spento) | `effects` | 1311 | `avanzata_effetti.attivo_nel_render_togliendo_la_spunta_re` |
| scelta | Seleziona: il gizmo sposta l'emettitore | `effects` | 1315 | `avanzata_effetti.seleziona_il_gizmo_sposta_l_emettitore` |
| campo di testo | Il gizmo sposta | `effects` | 1329 | `avanzata_effetti.il_gizmo_sposta` |
| scelta | (variabile) | `effects` | 1345 | `avanzata_effetti.effects_2` |
| campo numerico | Nuovo | `effects` | 1395 | `avanzata_effetti.nuovo` |
| cursore | (variabile) | `effects` | 1408 | `avanzata_effetti.effects_3` |
| cursore | Raggio sorgente (m) | `effects` | 1415 | `avanzata_effetti.raggio_sorgente_m` |
| cursore | Densità | `effects` | 1419 | `avanzata_effetti.densita` |
| cursore | Coefficiente di estinzione della nebbia piena (1/m): più alto = più… | `effects` | 1424 | `avanzata_effetti.coefficiente_di_estinzione_della_nebbia_` |
| cursore | Dimensione di sbuffi, filamenti e vortici | `effects` | 1430 | `avanzata_effetti.dimensione_di_sbuffi_filamenti_e_vortici` |
| cursore | Risalita | `effects` | 1435 | `avanzata_effetti.risalita` |
| cursore | (variabile) | `effects` | 1439 | `avanzata_effetti.effects_4` |
| cursore | Velocità con cui la nebbia si dissolve nell'aria (0 = mai) | `effects` | 1446 | `avanzata_effetti.velocita_con_cui_la_nebbia_si_dissolve_n` |
| colore | Intensità scattering | `effects` | 1451 | `avanzata_effetti.intensita_scattering` |
| cursore | Quanta luce la nebbia diffonde invece di assorbire (1 = bianca, 0 =… | `effects` | 1456 | `avanzata_effetti.quanta_luce_la_nebbia_diffonde_invece_di` |
| cursore | 0 = diffusione uniforme; positiva = più luminosa in controluce | `effects` | 1470 | `avanzata_effetti.0_diffusione_uniforme_positiva_piu_lumin` |
| casella | Non attraversa gli oggetti | `effects` | 1476 | `avanzata_effetti.non_attraversa_gli_oggetti` |
| scelta | (variabile) | `effects` | 1490 | `avanzata_effetti.effects_5` |
| cursore | (variabile) | `effects` | 1500 | `avanzata_effetti.effects_6` |

## Avanzata · Luci

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Sostituisce tutte le luci, adattandole alle dimensioni della scena | `lights` | 881 | `avanzata_luci.sostituisce_tutte_le_luci_adattandole_al` |
| pulsante | (variabile) | `lights` | 894 | `avanzata_luci.lights` |
| campo di testo | (variabile) | `lights` | 918 | `avanzata_luci.lights_2` |
| elenco a tendina | Tipo | `lights` | 928 | `avanzata_luci.tipo` |
| scelta | Area (rettangolare) | `lights` | 931 | `avanzata_luci.area_rettangolare` |
| scelta | Puntiforme | `lights` | 932 | `avanzata_luci.puntiforme` |
| scelta | Spot | `lights` | 933 | `avanzata_luci.spot` |
| scelta | Direzionale | `lights` | 934 | `avanzata_luci.direzionale` |
| scelta | Portale (finestra) | `lights` | 935 | `avanzata_luci.portale_finestra` |
| casella | Accesa | `lights` | 951 | `avanzata_luci.accesa` |
| campo numerico | Altezza (m) | `lights` | 966 | `avanzata_luci.altezza_m` |
| campo numerico | (variabile) | `lights` | 969 | `avanzata_luci.lights_3` |
| colore | Temperatura | `lights` | 975 | `avanzata_luci.temperatura` |
| campo numerico | (variabile) | `lights` | 994 | `avanzata_luci.lights_4` |
| campo numerico | Altezza (m) | `lights` | 1000 | `avanzata_luci.altezza_m_2` |
| campo numerico | (variabile) | `lights` | 1003 | `avanzata_luci.lights_5` |
| cursore | (variabile) | `lights` | 1019 | `avanzata_luci.lights_6` |
| cursore | 0 % = bordo netto, 100 % = sfuma dal centro (maniglia vuota nella v… | `lights` | 1031 | `avanzata_luci.0_bordo_netto_100_sfuma_dal_centro_manig` |
| casella | Limitato | `lights` | 1047 | `avanzata_luci.limitato` |
| campo numerico | (variabile) | `lights` | 1065 | `avanzata_luci.lights_7` |
| cursore | (variabile) | `lights` | 1075 | `avanzata_luci.lights_8` |
| casella | Proietta ombre | `lights` | 1097 | `avanzata_luci.proietta_ombre` |
| casella | Visibile alla camera | `lights` | 1100 | `avanzata_luci.visibile_alla_camera` |
| pulsante | Duplica | `lights` | 1103 | `avanzata_luci.duplica` |
| pulsante | Elimina | `lights` | 1104 | `avanzata_luci.elimina` |

## Avanzata · Materiali

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Salva | `library_save_controls` | 72 | `avanzata_materiali.salva` |
| pulsante | (variabile) | `library_save_controls` | 93 | `avanzata_materiali.library_save_controls` |
| pulsante | Ripristina | `library_save_controls` | 107 | `avanzata_materiali.ripristina` |
| campo di testo | Salva | `library_save_controls` | 124 | `avanzata_materiali.salva_2` |
| pulsante | Salva | `library_save_controls` | 126 | `avanzata_materiali.salva_3` |
| pulsante | Annulla | `library_save_controls` | 164 | `avanzata_materiali.annulla` |
| campo di testo | cerca… | `materials` | 230 | `avanzata_materiali.cerca` |
| scelta | (variabile) | `materials` | 275 | `avanzata_materiali.materials` |
| pulsante | Libreria con texture (ambientCG, Poly Haven)… | `materials` | 285 | `avanzata_materiali.libreria_con_texture_ambientcg_poly_have` |
| pulsante | Assegna alla selezione | `materials` | 297 | `avanzata_materiali.assegna_alla_selezione` |
| pulsante | Rimuovi assegnazione | `materials` | 306 | `avanzata_materiali.rimuovi_assegnazione` |
| campo di testo | (variabile) | `materials` | 318 | `avanzata_materiali.materials_2` |
| pulsante | Assegna ({}) | `materials` | 330 | `avanzata_materiali.assegna` |
| pulsante | Usa come predefinito per i componenti senza assegnazione | `materials` | 347 | `avanzata_materiali.usa_come_predefinito_per_i_componenti_se` |
| pulsante | Nuovo | `materials` | 356 | `avanzata_materiali.nuovo` |
| pulsante | Duplica | `materials` | 367 | `avanzata_materiali.duplica` |
| pulsante | Elimina | `materials` | 382 | `avanzata_materiali.elimina` |
| campo di testo | Scala e posizionamento texture | `materials` | 413 | `avanzata_materiali.scala_e_posizionamento_texture` |
| colore | (variabile) | `materials` | 438 | `avanzata_materiali.materials_3` |
| cursore | (variabile) | `materials` | 444 | `avanzata_materiali.materials_4` |
| cursore | (variabile) | `materials` | 454 | `avanzata_materiali.materials_5` |
| cursore | 0.5 = riflesso fisico derivato dall'IOR | `materials` | 462 | `avanzata_materiali.0_5_riflesso_fisico_derivato_dall_ior` |
| cursore | Transmission | `materials` | 470 | `avanzata_materiali.transmission` |
| cursore | Trasparenza dielettrica (vetro, liquidi) | `materials` | 474 | `avanzata_materiali.trasparenza_dielettrica_vetro_liquidi` |
| colore | (variabile) | `materials` | 481 | `avanzata_materiali.materials_6` |
| campo numerico |  cm | `materials` | 490 | `avanzata_materiali.cm` |
| campo numerico | (variabile) | `materials` | 499 | `avanzata_materiali.materials_7` |
| colore | Intensità emissione | `materials` | 507 | `avanzata_materiali.intensita_emissione` |
| campo numerico | Categoria | `materials` | 511 | `avanzata_materiali.categoria` |
| campo di testo | (variabile) | `materials` | 515 | `avanzata_materiali.materials_8` |
| casella | È una mappa di lucidità (gloss) | `materials` | 540 | `avanzata_materiali.e_una_mappa_di_lucidita_gloss` |
| casella | Formato DirectX | `materials` | 561 | `avanzata_materiali.formato_directx` |
| cursore | (variabile) | `materials` | 577 | `avanzata_materiali.materials_9` |
| scelta | Box (CAD) | `mapping_controls` | 601 | `avanzata_materiali.box_cad` |
| scelta | UV della mesh | `mapping_controls` | 605 | `avanzata_materiali.uv_della_mesh` |
| cursore | (variabile) | `mapping_controls` | 617 | `avanzata_materiali.mapping_controls` |
| campo numerico | (variabile) | `mapping_controls` | 645 | `avanzata_materiali.mapping_controls_2` |
| cursore | Quante volte la texture si ripete sull'intervallo UV 0–1 del modello | `mapping_controls` | 662 | `avanzata_materiali.quante_volte_la_texture_si_ripete_sull_i` |
| cursore | (variabile) | `mapping_controls` | 671 | `avanzata_materiali.mapping_controls_3` |
| campo numerico | U  | `mapping_controls` | 677 | `avanzata_materiali.u` |
| campo numerico | V  | `mapping_controls` | 680 | `avanzata_materiali.v` |
| pulsante | Usa la scala del displacement del file ({mm:.1} mm) | `mtlx_values` | 734 | `avanzata_materiali.usa_la_scala_del_displacement_del_file_m` |
| cursore | Dettaglio fine dell'ombreggiatura (non cambia la sagoma) | `relief_controls` | 754 | `avanzata_materiali.dettaglio_fine_dell_ombreggiatura_non_ca` |
| cursore | (variabile) | `relief_controls` | 762 | `avanzata_materiali.relief_controls` |
| cursore | (variabile) | `relief_controls` | 770 | `avanzata_materiali.relief_controls_2` |
| cursore | (variabile) | `relief_controls` | 794 | `avanzata_materiali.relief_controls_3` |

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
| elenco a tendina | Dispositivo | `render` | 1735 | `avanzata_render.dispositivo` |
| scelta | (variabile) | `render` | 1739 | `avanzata_render.render` |
| cursore | 0 = automatico (thread logici − 1, per lasciare reattiva l'interfac… | `render` | 1767 | `avanzata_render.0_automatico_thread_logici_1_per_lasciar` |
| scelta | (variabile) | `render` | 1786 | `avanzata_render.render_2` |
| campo numerico |  px | `render` | 1795 | `avanzata_render.px` |
| campo numerico |  px | `render` | 1799 | `avanzata_render.px_2` |
| scelta | (variabile) | `render` | 1807 | `avanzata_render.render_3` |
| scelta | Personalizzato | `render` | 1813 | `avanzata_render.personalizzato` |
| scelta | Illimitato | `render` | 1818 | `avanzata_render.illimitato` |
| campo numerico | campioni:  | `render` | 1830 | `avanzata_render.campioni` |
| casella | Arresto a soglia di rumore | `render` | 1848 | `avanzata_render.arresto_a_soglia_di_rumore` |
| cursore | (variabile) | `render` | 1865 | `avanzata_render.render_4` |
| campo numerico | (variabile) | `render` | 1878 | `avanzata_render.render_5` |
| cursore | Russian roulette da | `render` | 1891 | `avanzata_render.russian_roulette_da` |
| cursore | Clamp indiretto | `render` | 1894 | `avanzata_render.clamp_indiretto` |
| campo numerico | (variabile) | `render` | 1898 | `avanzata_render.render_6` |
| campo numerico | (variabile) | `render` | 1905 | `avanzata_render.render_7` |
| elenco a tendina | Tone mapping | `render` | 1911 | `avanzata_render.tone_mapping` |
| scelta | (variabile) | `render` | 1915 | `avanzata_render.render_8` |
| cursore | Esposizione (EV) | `render` | 1919 | `avanzata_render.esposizione_ev` |
| elenco a tendina | Modalità | `denoiser` | 1957 | `avanzata_render.modalita` |
| scelta | (variabile) | `denoiser` | 1961 | `avanzata_render.denoiser` |
| casella | Usa CUDA (GPU NVIDIA) | `denoiser` | 1973 | `avanzata_render.usa_cuda_gpu_nvidia` |
| elenco a tendina | (variabile) | `denoiser` | 1986 | `avanzata_render.denoiser_2` |
| scelta | (variabile) | `denoiser` | 1988 | `avanzata_render.denoiser_3` |
| campo numerico |  campioni | `denoiser` | 2003 | `avanzata_render.campioni_2` |
| cursore | (variabile) | `denoiser` | 2014 | `avanzata_render.denoiser_4` |
| casella | Preserva i dettagli (albedo e normali) | `denoiser` | 2026 | `avanzata_render.preserva_i_dettagli_albedo_e_normali` |
| scelta | (variabile) | `denoiser` | 2034 | `avanzata_render.denoiser_5` |

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
| comando | (variabile) | `show` | 24 | `barra_dei_menu.show` |
| comando | (variabile) | `show` | 25 | `barra_dei_menu.show_2` |
| comando | Strumenti | `show` | 27 | `barra_dei_menu.strumenti` |
| menu | Strumenti | `show` | 29 | `barra_dei_menu.strumenti_2` |
| casella | (variabile) | `show` | 33 | `barra_dei_menu.show_3` |
| menu | Guida | `show` | 40 | `barra_dei_menu.guida` |
| comando | (variabile) | `show` | 41 | `barra_dei_menu.show_4` |
| comando | Scorciatoie da tastiera | `show` | 42 | `barra_dei_menu.scorciatoie_da_tastiera` |
| comando | Benvenuto e informazioni… | `show` | 44 | `barra_dei_menu.benvenuto_e_informazioni` |
| pulsante | (variabile) | `item_with` | 67 | `barra_dei_menu.item_with` |
| comando | (variabile) | `tab_item` | 88 | `barra_dei_menu.tab_item` |
| comando | (variabile) | `file_menu` | 92 | `barra_dei_menu.file_menu` |
| comando | (variabile) | `file_menu` | 93 | `barra_dei_menu.file_menu_2` |
| comando | Progetti recenti | `file_menu` | 95 | `barra_dei_menu.progetti_recenti` |
| menu | Progetti recenti | `file_menu` | 97 | `barra_dei_menu.progetti_recenti_2` |
| pulsante | (variabile) | `file_menu` | 103 | `barra_dei_menu.file_menu_3` |
| comando | (variabile) | `file_menu` | 110 | `barra_dei_menu.file_menu_4` |
| comando | (variabile) | `file_menu` | 111 | `barra_dei_menu.file_menu_5` |
| comando | Importa modello… | `file_menu` | 113 | `barra_dei_menu.importa_modello` |
| comando | (variabile) | `file_menu` | 114 | `barra_dei_menu.file_menu_6` |
| comando | (variabile) | `file_menu` | 116 | `barra_dei_menu.file_menu_7` |
| comando | (variabile) | `edit_menu` | 120 | `barra_dei_menu.edit_menu` |
| comando | (variabile) | `edit_menu` | 121 | `barra_dei_menu.edit_menu_2` |
| comando | (variabile) | `edit_menu` | 123 | `barra_dei_menu.edit_menu_3` |
| comando | (variabile) | `edit_menu` | 124 | `barra_dei_menu.edit_menu_4` |
| comando | Nascondi selezione | `edit_menu` | 126 | `barra_dei_menu.nascondi_selezione` |
| comando | (variabile) | `edit_menu` | 127 | `barra_dei_menu.edit_menu_5` |
| comando | Mostra tutto | `edit_menu` | 128 | `barra_dei_menu.mostra_tutto` |
| comando | (variabile) | `edit_menu` | 130 | `barra_dei_menu.edit_menu_6` |
| comando | (variabile) | `edit_menu` | 131 | `barra_dei_menu.edit_menu_7` |
| comando | (variabile) | `scene_menu` | 135 | `barra_dei_menu.scene_menu` |
| comando | Aspetto | `scene_menu` | 136 | `barra_dei_menu.aspetto` |
| comando | Aspetto | `scene_menu` | 137 | `barra_dei_menu.aspetto_2` |
| menu | Aspetto | `scene_menu` | 139 | `barra_dei_menu.aspetto_3` |
| pulsante | (variabile) | `scene_menu` | 143 | `barra_dei_menu.scene_menu_2` |
| comando | Ambiente e HDRI… | `scene_menu` | 152 | `barra_dei_menu.ambiente_e_hdri` |
| comando | Ambiente e HDRI… | `scene_menu` | 153 | `barra_dei_menu.ambiente_e_hdri_2` |
| comando | Ambiente e HDRI… | `scene_menu` | 155 | `barra_dei_menu.ambiente_e_hdri_3` |
| comando | Effetti e nebbia… | `scene_menu` | 156 | `barra_dei_menu.effetti_e_nebbia` |
| pulsante | (variabile) | `objects_menu` | 161 | `barra_dei_menu.objects_menu` |
| menu | Luce | `objects_menu` | 167 | `barra_dei_menu.luce` |
| pulsante | (variabile) | `objects_menu` | 169 | `barra_dei_menu.objects_menu_2` |
| comando | Posizione e dimensioni… | `objects_menu` | 176 | `barra_dei_menu.posizione_e_dimensioni` |
| comando | Materiali… | `objects_menu` | 177 | `barra_dei_menu.materiali` |
| comando | Luci… | `objects_menu` | 178 | `barra_dei_menu.luci` |
| comando | Inquadra tutto | `view_menu` | 182 | `barra_dei_menu.inquadra_tutto` |
| comando | (variabile) | `view_menu` | 183 | `barra_dei_menu.view_menu` |
| comando | Viste standard | `view_menu` | 185 | `barra_dei_menu.viste_standard` |
| comando | Viste standard | `view_menu` | 186 | `barra_dei_menu.viste_standard_2` |
| menu | Viste standard | `view_menu` | 187 | `barra_dei_menu.viste_standard_3` |
| pulsante | (variabile) | `view_menu` | 189 | `barra_dei_menu.view_menu_2` |
| pulsante | (variabile) | `view_menu` | 199 | `barra_dei_menu.view_menu_3` |
| comando | Camera e obiettivo… | `view_menu` | 208 | `barra_dei_menu.camera_e_obiettivo` |
| comando | (variabile) | `render_menu` | 212 | `barra_dei_menu.render_menu` |
| comando | Dispositivo di calcolo | `render_menu` | 213 | `barra_dei_menu.dispositivo_di_calcolo` |
| comando | Dispositivo di calcolo | `render_menu` | 214 | `barra_dei_menu.dispositivo_di_calcolo_2` |
| menu | Dispositivo di calcolo | `render_menu` | 216 | `barra_dei_menu.dispositivo_di_calcolo_3` |
| scelta | Scheda video (GPU) | `render_menu` | 220 | `barra_dei_menu.scheda_video_gpu` |
| scelta | Processore (CPU) | `render_menu` | 223 | `barra_dei_menu.processore_cpu` |
| menu | Qualità | `render_menu` | 230 | `barra_dei_menu.qualita` |
| pulsante | {} · {spp} campioni | `render_menu` | 235 | `barra_dei_menu.spp_campioni` |
| menu | Formato | `render_menu` | 244 | `barra_dei_menu.formato` |
| pulsante | (variabile) | `render_menu` | 248 | `barra_dei_menu.render_menu_2` |
| casella | (variabile) | `render_menu` | 261 | `barra_dei_menu.render_menu_3` |
| comando | Impostazioni di render e denoiser… | `render_menu` | 265 | `barra_dei_menu.impostazioni_di_render_e_denoiser` |
| comando | (variabile) | `render_menu` | 267 | `barra_dei_menu.render_menu_4` |
| scelta | Controllo IA | `ai_control` | 280 | `barra_dei_menu.controllo_ia` |
| elenco a tendina | (variabile) | `language_switch` | 302 | `barra_dei_menu.language_switch` |
| scelta | (variabile) | `language_switch` | 307 | `barra_dei_menu.language_switch_2` |
| scelta | Semplice | `mode_switch` | 326 | `barra_dei_menu.semplice` |
| scelta | Avanzata | `mode_switch` | 328 | `barra_dei_menu.avanzata` |

## Benvenuto

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Chiudi | `show` | 59 | `benvenuto.chiudi` |

## Finestre di dialogo

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| elenco a tendina | (variabile) | `import_options_window` | 97 | `finestre_di_dialogo.import_options_window` |
| scelta | (variabile) | `import_options_window` | 101 | `finestre_di_dialogo.import_options_window_2` |
| elenco a tendina | (variabile) | `import_options_window` | 106 | `finestre_di_dialogo.import_options_window_3` |
| scelta | (variabile) | `import_options_window` | 110 | `finestre_di_dialogo.import_options_window_4` |
| scelta | Bassa | `import_options_window` | 118 | `finestre_di_dialogo.bassa` |
| scelta | Media | `import_options_window` | 119 | `finestre_di_dialogo.media` |
| scelta | Alta | `import_options_window` | 120 | `finestre_di_dialogo.alta` |
| casella | Appoggia sul pavimento dopo l'importazione | `import_options_window` | 124 | `finestre_di_dialogo.appoggia_sul_pavimento_dopo_l_importazio` |
| cursore | (variabile) | `import_options_window` | 127 | `finestre_di_dialogo.import_options_window_5` |
| pulsante | Importa | `import_options_window` | 142 | `finestre_di_dialogo.importa` |
| pulsante | Annulla | `import_options_window` | 143 | `finestre_di_dialogo.annulla` |
| pulsante | Salva | `confirm_discard_window` | 198 | `finestre_di_dialogo.salva` |
| pulsante | Non salvare | `confirm_discard_window` | 203 | `finestre_di_dialogo.non_salvare` |
| pulsante | Annulla | `confirm_discard_window` | 206 | `finestre_di_dialogo.annulla_2` |
| pulsante | (variabile) | `dialogs` | 286 | `finestre_di_dialogo.dialogs` |
| pulsante | (variabile) | `dialogs` | 303 | `finestre_di_dialogo.dialogs_2` |

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
| scelta | (variabile) | `show` | 104 | `toolbar.show_6` |
| pulsante | (variabile) | `cmd_button` | 147 | `toolbar.cmd_button` |
| pulsante | (variabile) | `cmd_button` | 150 | `toolbar.cmd_button_2` |
| pulsante | (variabile) | `render_button` | 162 | `toolbar.render_button` |
| pulsante | (variabile) | `dropdown` | 171 | `toolbar.dropdown` |
| pulsante | (variabile) | `tool_button` | 177 | `toolbar.tool_button` |

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

## manual.rs

| Tipo | Testo | Funzione | Riga | Id proposto |
|---|---|---|---|---|
| pulsante | Apri la pagina del manuale (F1) | `help_button` | 126 | `manual_rs.apri_la_pagina_del_manuale_f1` |
| pulsante | Home | `contents` | 187 | `manual_rs.home` |
| pulsante | Indietro | `contents` | 194 | `manual_rs.indietro` |
| pulsante | Avanti | `contents` | 202 | `manual_rs.avanti` |
| campo di testo | Cerca nel manuale… | `contents` | 210 | `manual_rs.cerca_nel_manuale` |
| elenco a tendina | (variabile) | `contents` | 218 | `manual_rs.contents` |
| scelta | (variabile) | `contents` | 223 | `manual_rs.contents_2` |
| scelta | (variabile) | `contents` | 260 | `manual_rs.contents_3` |
| scelta | (variabile) | `contents` | 280 | `manual_rs.contents_4` |
