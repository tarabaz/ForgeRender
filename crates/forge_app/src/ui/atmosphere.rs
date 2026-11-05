//! Scheda Effetti: foschia d'ambiente (cinque tipi), polvere in sospensione e adesivi (decal).

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::icons;
use eframe::egui;
use forge_scene::{Decal, Haze, HazeKind, NodeIndex};

fn kind_name(k: HazeKind) -> &'static str {
    match k {
        HazeKind::Ambient => tr("Nebbia bassa"),
        HazeKind::GasClouds => tr("Nuvole di gas"),
        HazeKind::ToxicFloor => tr("Fumo tossico a terra"),
        HazeKind::Layers => tr("Strati sospesi"),
        HazeKind::Billows => tr("Riccioli di vapore"),
    }
}

fn kind_hint(k: HazeKind) -> &'static str {
    match k {
        HazeKind::Ambient => tr("Nebbia che si stende a terra e si dirada salendo, con banchi e riccioli"),
        HazeKind::GasClouds => tr("Aria quasi limpida con ammassi densi qua e là, come nuvole di gas sospese"),
        HazeKind::ToxicFloor => tr("Uno strato di pochi centimetri che aleggia sul pavimento, raccolto in filamenti come fumo tossico"),
        HazeKind::Layers => tr("Veli orizzontali sovrapposti a quote diverse, ondulati e bucati, come foschia in una valle"),
        HazeKind::Billows => tr("Vortici dentro i vortici, con contrasto alto: riccioli e spirali di vapore ben visibili"),
    }
}

/// Pulsanti "Foschia", "Polvere in sospensione" e "Adesivo" della riga "Aggiungi effetto".
pub fn add_buttons(app: &mut ForgeApp, ui: &mut egui::Ui) {
    // Foschia: un menu con i tipi (una sola foschia per scena; il tipo si cambia anche dopo).
    ui.add_enabled_ui(!app.scene.haze.enabled, |ui| {
        ui.menu_button(tr("Foschia"), |ui| {
            for k in HazeKind::ALL {
                if ui.button(kind_name(k)).on_hover_text(kind_hint(k)).clicked() {
                    app.scene.haze = Haze::of_kind(k);
                    app.shading_changed();
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr(
            "Aria con nebbia, gas o fumo: le luci che la attraversano disegnano raggi con le ombre degli oggetti",
        ))
        .on_disabled_hover_text(tr("Già nella scena: le impostazioni sono qui sotto"));
    });
    crate::ui::smoke::add_button(app, ui);
    if ui
        .add_enabled(!app.scene.haze_dust.enabled, egui::Button::new(tr("Polvere in sospensione")))
        .on_hover_text(tr(
            "Granelli nell'aria che si vedono solo dentro i fasci di spot e sole, come la polvere nei raggi \
             che entrano da una finestra: altrove sono invisibili",
        ))
        .on_disabled_hover_text(tr("Già nella scena: le impostazioni sono qui sotto"))
        .clicked()
    {
        app.scene.haze_dust.enabled = true;
        app.scene.haze_dust.region = Some(app.scene.default_dust_region());
        app.selected_dust = true;
        dust_changed(app);
    }
    if ui
        .button(tr("Adesivo (decal)"))
        .on_hover_text(tr(
            "Un'immagine (logo, etichetta, PNG con trasparenza) applicata sull'oggetto selezionato, nel \
             punto che guardi: segue anche le superfici curve e si sposta con l'oggetto",
        ))
        .clicked()
    {
        add_decal(app);
    }
}

fn pick_image() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter(tr("Immagini"), &["png", "jpg", "jpeg", "webp", "bmp", "tga"])
        .pick_file()
}

/// Proporzioni dell'immagine (altezza / larghezza).
fn aspect_of(path: &std::path::Path) -> f32 {
    image::image_dimensions(path)
        .map(|(w, h)| h as f32 / w.max(1) as f32)
        .unwrap_or(1.0)
}

/// Raggio dalla camera della vista verso il centro dei componenti selezionati.
fn view_ray(app: &ForgeApp, among: &[NodeIndex]) -> Option<(forge_math::Vec3, forge_math::Vec3, f32)> {
    let mut b = forge_math::Aabb::EMPTY;
    for &i in among {
        b.grow(&app.scene.bounds_of(i));
    }
    if b.is_empty() {
        return None;
    }
    let cam = &app.scene.camera;
    let d = (b.center() - cam.position).normalize_or(forge_math::Vec3::NEG_Z);
    Some((cam.position, d, (b.max - b.min).max_element()))
}

fn add_decal(app: &mut ForgeApp) {
    let among: Vec<NodeIndex> = app.selection.iter().copied().collect();
    let Some((o, d, size)) = view_ray(app, &among) else {
        app.toast(tr("Seleziona prima l'oggetto su cui applicare l'adesivo"));
        return;
    };
    let Some(image) = pick_image() else { return };
    let up = app.scene.camera.up;
    let width = (size * 0.3).clamp(0.005, 2.0);
    let Some(mut decal) = app.scene.new_decal(&among, o, d, up, width, aspect_of(&image)) else {
        app.toast(tr(
            "L'oggetto selezionato non si vede dalla vista: giralo verso la camera e riprova",
        ));
        return;
    };
    decal.name = image
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| decal.name.clone());
    decal.image = Some(image);
    app.scene.decals.push(decal);
    app.scene.sync_effects();
    app.geometry_changed();
    app.appearance_changed();
}

/// Nebbia bassa, polvere e adesivi della scena (quelli presenti).
pub fn sections(app: &mut ForgeApp, ui: &mut egui::Ui) {
    if app.scene.haze.enabled {
        haze(app, ui);
        ui.separator();
    }
    if !app.scene.smokes.is_empty() {
        crate::ui::smoke::sections(app, ui);
        ui.separator();
    }
    if app.scene.haze_dust.enabled {
        dust(app, ui);
        ui.separator();
    }
    if !app.scene.decals.is_empty() {
        decals(app, ui);
        ui.separator();
    }
}

fn haze(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut remove = false;
    let id = ui.make_persistent_id("foschia-ambiente");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            ui.strong(tf("Foschia: {}", &[kind_name(app.scene.haze.kind).to_string()]));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(icons::DELETE).on_hover_text(tr("Togli la foschia")).clicked() {
                    remove = true;
                }
            });
        })
        .body(|ui| {
            let h = &mut app.scene.haze;
            egui::Grid::new("foschia").num_columns(2).show(ui, |ui| {
                ui.label(tr("Tipo"));
                let mut kind = h.kind;
                egui::ComboBox::from_id_salt("tipo-foschia")
                    .selected_text(kind_name(kind))
                    .show_ui(ui, |ui| {
                        for k in HazeKind::ALL {
                            ui.selectable_value(&mut kind, k, kind_name(k)).on_hover_text(kind_hint(k));
                        }
                    })
                    .response
                    .on_hover_text(tr(
                        "Cambiare tipo riparte dalle sue impostazioni di partenza (colore e forma restano)",
                    ));
                if kind != h.kind {
                    let (seed, quality) = (h.seed, h.quality);
                    *h = Haze::of_kind(kind);
                    h.seed = seed;
                    h.quality = quality;
                    changed = true;
                }
                ui.end_row();
                ui.label(tr("Solo raggi delle luci"));
                changed |= ui
                    .checkbox(&mut h.rays_only, "")
                    .on_hover_text(tr(
                        "Acceso: la foschia mostra solo la luce di spot, sole e lampade che la attraversa (i raggi), senza \
                         velare né scurire gli oggetti. Spento: foschia fisica, che vela tutto con la luce dell'ambiente",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Densità"));
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(egui::Slider::new(&mut h.density, 0.001..=50.0).logarithmic(true).max_decimals(3))
                        .on_hover_text(tr(
                            "Quanto è spessa l'aria (1/m): più alta = raggi più luminosi e, se la foschia è fisica, oggetti \
                             più velati",
                        ))
                        .changed();
                    // Scala: distanza a cui un oggetto sparirebbe nella foschia (contrasto al 5 %).
                    let vis = 3.0 / h.density.max(1e-4);
                    let text = if vis >= 1.0 {
                        format!("{vis:.1} m")
                    } else {
                        format!("{:.0} cm", vis * 100.0)
                    };
                    ui.label(egui::RichText::new(tf("si vede fino a ~{}", &[text])).small().weak())
                        .on_hover_text(tr(
                            "Scala della densità: a questa distanza un oggetto sparirebbe nella foschia (nei punti più \
                             densi prima, in quelli più tenui dopo)",
                        ));
                });
                ui.end_row();
                ui.label(tr("Addensamenti"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.clumping, 0.0..=2.0))
                    .on_hover_text(tr(
                        "Zone grandi più dense e altre più tenui, sparse nella scena: 0 = uguale ovunque, 1 = ammassi \
                         marcati e vuoti, 2 = pochi ammassi molto concentrati",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Altezza di partenza"));
                ui.horizontal(|ui| {
                    let mut raised = h.bottom_height > 0.0;
                    if ui
                        .checkbox(&mut raised, "")
                        .on_hover_text(tr(
                            "Fa partire la foschia da un'altezza dal pavimento invece che da terra: con l'altezza massima \
                             si ottiene una fascia sospesa, per esempio da 2,2 a 2,8 m sotto il soffitto",
                        ))
                        .changed()
                    {
                        h.bottom_height = if raised { 1.0 } else { 0.0 };
                        if raised && h.top_height > 0.0 && h.top_height <= h.bottom_height {
                            h.top_height = h.bottom_height + 0.5;
                        }
                        changed = true;
                    }
                    if raised {
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut h.bottom_height, 0.01..=20.0)
                                    .logarithmic(true)
                                    .suffix(" m")
                                    .max_decimals(2),
                            )
                            .on_hover_text(tr(
                                "Da che altezza dal pavimento comincia la foschia (sotto, solo qualche sbuffo che pende)",
                            ))
                            .changed();
                    }
                });
                ui.end_row();
                ui.label(tr("Altezza massima"));
                ui.horizontal(|ui| {
                    let mut limited = h.top_height > 0.0;
                    if ui
                        .checkbox(&mut limited, "")
                        .on_hover_text(tr("Ferma la foschia a un'altezza dal pavimento, salvo qualche sbuffo che sale"))
                        .changed()
                    {
                        h.top_height = if limited { h.bottom_height + 0.5 } else { 0.0 };
                        changed = true;
                    }
                    if limited {
                        let lo = h.bottom_height + 0.02;
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut h.top_height, lo..=lo + 20.0)
                                    .logarithmic(true)
                                    .suffix(" m")
                                    .max_decimals(2),
                            )
                            .on_hover_text(tr("Fino a che altezza dal pavimento arriva la foschia"))
                            .changed();
                        h.top_height = h.top_height.max(lo);
                    }
                });
                ui.end_row();
                if h.top_height > 0.0 || h.bottom_height > 0.0 {
                    ui.label(tr("Sbuffi"));
                    changed |= ui
                        .add(egui::Slider::new(&mut h.puffs, 0.0..=1.0))
                        .on_hover_text(tr(
                            "Quanti sbuffi salgono a caso sopra l'altezza massima e pendono sotto quella di partenza: \
                             0 = taglio netto",
                        ))
                        .changed();
                    ui.end_row();
                }
                ui.label(tr("Colore"));
                changed |= ui
                    .color_edit_button_rgb(&mut h.color)
                    .on_hover_text(tr(
                        "Colore della luce diffusa dalla foschia: bianco = aria neutra, caldo = polvere, freddo = nebbia",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Raggi in controluce"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.anisotropy, -0.5..=0.95))
                    .on_hover_text(tr(
                        "Quanto la foschia manda la luce in avanti: alto = raggi brillanti guardando verso la luce, \
                         basso = alone uniforme",
                    ))
                    .changed();
                ui.end_row();
                let (var_label, var_hint) = match h.kind {
                    HazeKind::GasClouds => (
                        tr("Quante nuvole"),
                        tr("Quanta aria coprono gli ammassi: 0 = pochi e piccoli, 1 = metà dell'aria"),
                    ),
                    HazeKind::ToxicFloor => (
                        tr("Filamenti"),
                        tr("Quanto il fumo si raccoglie in linee: 0 = strato uniforme, 1 = solo filamenti"),
                    ),
                    _ => (
                        tr("Vortici"),
                        tr("Quanto l'aria è irregolare: 0 = uniforme, 1 = banchi densi, riccioli e vuoti"),
                    ),
                };
                ui.label(var_label);
                changed |= ui
                    .add(egui::Slider::new(&mut h.variation, 0.0..=1.0))
                    .on_hover_text(var_hint)
                    .changed();
                ui.end_row();
                ui.label(tr("Grandezza dei vortici"));
                changed |= ui
                    .add(
                        egui::Slider::new(&mut h.swirl_scale, 0.01..=20.0)
                            .logarithmic(true)
                            .suffix(" m")
                            .max_decimals(2),
                    )
                    .on_hover_text(tr("Dimensione tipica dei banchi e dei riccioli"))
                    .changed();
                ui.end_row();
                ui.label(tr("Forza dei vortici"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.warp, 0.0..=8.0).max_decimals(2))
                    .on_hover_text(tr(
                        "Quanto il rumore viene attorcigliato: 0 = macchie tonde, 1,5 = banchi mossi, 3–4 = riccioli \
                         e spirali ben visibili, 6–8 = vortici estremi",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Dettaglio frattale"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.octaves, 1..=10))
                    .on_hover_text(tr(
                        "Ottave del frattale: 1–2 = forme morbide e grandi, 6–10 = dettagli sempre più fini dentro le \
                         forme (servono qualità alte per vederli)",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Ruvidità"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.roughness, 0.2..=0.85).max_decimals(2))
                    .on_hover_text(tr(
                        "Quanto pesano i dettagli fini: bassa = bordi morbidi, alta = bordi frastagliati e accentuati",
                    ))
                    .changed();
                ui.end_row();
                let (fall_label, fall_hint, range) = match h.kind {
                    HazeKind::ToxicFloor => (
                        tr("Spessore dello strato"),
                        tr("Altezza dello strato di fumo sopra il pavimento (o sopra l'altezza di partenza)"),
                        0.005..=2.0,
                    ),
                    HazeKind::Layers => (
                        tr("Distanza tra gli strati"),
                        tr("Distanza verticale tra un velo e il successivo"),
                        0.05..=8.0,
                    ),
                    _ => (
                        tr("Si dirada salendo"),
                        tr("Altezza in cui la foschia si dimezza salendo dal punto più basso della scena: \
                            0 = uguale ovunque, piccola = nebbia a terra"),
                        0.0..=20.0,
                    ),
                };
                ui.label(fall_label);
                changed |= ui
                    .add(egui::Slider::new(&mut h.falloff_height, range).suffix(" m").max_decimals(2))
                    .on_hover_text(fall_hint)
                    .changed();
                ui.end_row();
                ui.label(tr("Qualità"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.quality, 1..=4))
                    .on_hover_text(tr(
                        "Quanto è fitta la griglia della foschia: 1 = bozza (1 milione di celle), 2 = normale (4 milioni), \
                         3 = alta (12 milioni), 4 = massima (24 milioni, 48 MB). Più alta = dettagli fini e bordi netti, \
                         calcolo più lungo (la barra in basso mostra l'avanzamento)",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Forma"));
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut h.seed).range(1..=9999)).changed();
                    if ui
                        .small_button(tr("Nuova"))
                        .on_hover_text(tr("Altra disposizione dei vortici"))
                        .clicked()
                    {
                        h.seed = h.seed % 9999 + 1;
                        changed = true;
                    }
                });
                ui.end_row();
            });
        });
    if remove {
        app.scene.haze.enabled = false;
        changed = true;
    }
    if changed {
        app.shading_changed();
    }
}

/// Polvere aggiunta, tolta o cambiata: si rifà il suo nodo.
pub(crate) fn dust_changed(app: &mut ForgeApp) {
    let nodes = app.scene.nodes.len();
    app.scene.sync_effects();
    if !app.scene.haze_dust.enabled || app.scene.nodes.len() != nodes {
        // Un nodo della polvere è sparito o nato: gli indici dei nodi dopo di lui cambiano.
        app.selection.clear();
    }
    app.geometry_changed();
    app.appearance_changed();
}

fn dust(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut remove = false;
    let mut select = false;
    let id = ui.make_persistent_id("polvere-sospesa");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            ui.strong(tr("Polvere in sospensione"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(icons::DELETE).on_hover_text(tr("Togli la polvere")).clicked() {
                    remove = true;
                }
            });
        })
        .body(|ui| {
            ui.label(
                egui::RichText::new(tr(
                    "Si vede solo dentro i fasci di spot e sole: serve un raggio che attraversi l'aria. Lampadine, \
                     pannelli, ambiente e luce di rimbalzo non la illuminano.",
                ))
                .small()
                .weak(),
            );
            let auto_region = app.scene.default_dust_region();
            let selected = app.selected_dust;
            let dust = &mut app.scene.haze_dust;
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(selected, tr("Riquadro della polvere"))
                    .on_hover_text(tr(
                        "Seleziona il riquadro: il gizmo lo sposta e lo ridimensiona (Sposta / Scala). I granelli ci sono \
                         solo lì dentro: deve contenere tutto il fascio, fino al piano illuminato",
                    ))
                    .clicked()
                {
                    select = true;
                }
                if ui
                    .small_button(tr("Adatta alla scena"))
                    .on_hover_text(tr("Riquadro attorno agli oggetti fino alle spot (al massimo 6 m per lato)"))
                    .clicked()
                {
                    dust.region = Some(auto_region);
                    changed = true;
                }
            });
            let (mut c, mut s) = dust.region.unwrap_or(auto_region);
            if crate::ui::inspector::vec3_edit(ui, tr("Centro (m)"), &mut c, 0.01)
                | crate::ui::inspector::vec3_edit(ui, tr("Dimensioni (m)"), &mut s, 0.01)
            {
                dust.region = Some((c, s.max(forge_math::Vec3::splat(0.01))));
                changed = true;
            }
            egui::Grid::new("polvere").num_columns(2).show(ui, |ui| {
                ui.label(tr("Quantità"));
                changed |= ui
                    .add(
                        egui::Slider::new(&mut dust.per_m3, 1.0..=30000.0)
                            .logarithmic(true)
                            .suffix(tr(" /m³"))
                            .max_decimals(0),
                    )
                    .on_hover_text(tr("Granelli per metro cubo (al massimo 150 000 in tutto)"))
                    .changed();
                ui.end_row();
                ui.label(tr("Dimensione"));
                changed |= ui
                    .add(
                        egui::Slider::new(&mut dust.size_mm, 0.3..=18.0)
                            .logarithmic(true)
                            .suffix(" mm")
                            .max_decimals(2),
                    )
                    .on_hover_text(tr(
                        "Diametro medio dei granelli: più grandi = puntini più visibili (sotto la grandezza di un pixel il \
                         denoiser li scambia per rumore)",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Brillantezza"));
                changed |= ui
                    .add(
                        egui::Slider::new(&mut dust.brightness, 1.0..=50.0)
                            .logarithmic(true)
                            .max_decimals(1),
                    )
                    .on_hover_text(tr("Quanto brillano i granelli nel fascio"))
                    .changed();
                ui.end_row();
                ui.label(tr("Variazione dimensione"));
                changed |= ui
                    .add(egui::Slider::new(&mut dust.size_spread, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
                    .on_hover_text(tr(
                        "Quanto la dimensione dei granelli si scosta a caso da quella impostata: 0 = tutti uguali, 50 % = da \
                         metà al doppio, 100 % = da piccolissimi al triplo",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Opacità"));
                changed |= ui
                    .add(egui::Slider::new(&mut dust.opacity, 0.05..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
                    .on_hover_text(tr(
                        "Granelli semitrasparenti: diffondono meno luce e, se fanno ombra, ne fermano solo una parte",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Colore"));
                changed |= ui.color_edit_button_rgb(&mut dust.color).changed();
                ui.end_row();
                ui.label(tr("Secondo colore"));
                ui.horizontal(|ui| {
                    changed |= ui.color_edit_button_rgb(&mut dust.color2).changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut dust.color2_share, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
                        .on_hover_text(tr("Parte dei granelli con il secondo colore (0 = tutti del primo)"))
                        .changed();
                });
                ui.end_row();
                ui.label(tr("Forma"));
                ui.horizontal(|ui| {
                    changed |= ui
                        .selectable_value(&mut dust.shape, forge_scene::DustShape::Grains, tr("Granelli"))
                        .on_hover_text(tr("Granelli tondi: diffondono la luce uguale da ogni lato"))
                        .changed();
                    changed |= ui
                        .selectable_value(&mut dust.shape, forge_scene::DustShape::Flakes, tr("Scaglie"))
                        .on_hover_text(tr(
                            "Scaglie piatte orientate a caso, come polvere di fata o brillantini: riflettono come piccoli \
                             specchi, quindi brillano solo quelle girate giuste tra luce e camera e le altre restano spente",
                        ))
                        .changed();
                });
                ui.end_row();
                if dust.shape == forge_scene::DustShape::Flakes {
                    ui.label(tr("Luccichio"));
                    changed |= ui
                        .add(egui::Slider::new(&mut dust.sparkle, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
                        .on_hover_text(tr("Parte della luce riflessa a specchio dalle scaglie (il resto è diffusa)"))
                        .changed();
                    ui.end_row();
                    ui.label(tr("Ruvidità del riflesso"));
                    changed |= ui
                        .add(egui::Slider::new(&mut dust.flake_roughness, 0.02..=0.6).max_decimals(2))
                        .on_hover_text(tr(
                            "Bassa: poche scintille piccole e intense. Alta: più scaglie accese ma più tenui",
                        ))
                        .changed();
                    ui.end_row();
                }
                ui.label(tr("Fanno ombra"));
                changed |= ui
                    .checkbox(&mut dust.shadows, "")
                    .on_hover_text(tr(
                        "I granelli fermano la luce: piccole sagome scure dentro i fasci (con la foschia) e sul piano \
                         illuminato. Con pochi granelli grandi si vedono meglio",
                    ))
                    .changed();
                ui.end_row();
            });
        });
    if select {
        app.selected_dust = true;
        app.selected_fog = None;
        app.selected_smoke = None;
        app.selected_light = None;
        app.selection.clear();
    }
    if remove {
        app.scene.haze_dust.enabled = false;
        app.selected_dust = false;
        changed = true;
    }
    if changed {
        dust_changed(app);
    }
}

fn decals(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.strong(tf("Adesivi ({})", &[app.scene.decals.len().to_string()]));
    let mut geometry = false;
    let mut look = false;
    let mut remove = None;
    let mut replace = None;
    let mut reposition = None;
    for (i, d) in app.scene.decals.iter_mut().enumerate() {
        ui.push_id(("adesivo", d.n), |ui| {
            let id = ui.make_persistent_id("adesivo");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    geometry |= ui
                        .checkbox(&mut d.enabled, "")
                        .on_hover_text(tr("Visibile (togliendo la spunta resta nella scena, nascosto)"))
                        .changed();
                    ui.label(&d.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(icons::DELETE).on_hover_text(tr("Elimina l'adesivo")).clicked() {
                            remove = Some(i);
                        }
                    });
                })
                .body(|ui| {
                    ui.horizontal(|ui| {
                        let name = d
                            .image
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| tr("nessuna immagine").into());
                        ui.label(name);
                        if ui.small_button(tr("Cambia immagine…")).clicked() {
                            replace = Some(i);
                        }
                    });
                    egui::Grid::new("adesivo-grid").num_columns(2).show(ui, |ui| {
                        let aspect = d.height / d.width.max(1e-6);
                        ui.label(tr("Larghezza"));
                        if ui
                            .add(
                                egui::Slider::new(&mut d.width, 0.002..=3.0)
                                    .logarithmic(true)
                                    .suffix(" m")
                                    .max_decimals(3),
                            )
                            .on_hover_text(tr("L'altezza segue, con le proporzioni dell'immagine"))
                            .changed()
                        {
                            d.height = d.width * aspect;
                            d.depth = d.depth.max(d.width.max(d.height) * 0.25);
                            geometry = true;
                        }
                        ui.end_row();
                        ui.label(tr("Rotazione"));
                        geometry |= ui.add(egui::Slider::new(&mut d.rotation_deg, -180.0..=180.0).suffix("°")).changed();
                        ui.end_row();
                        ui.label(tr("Sposta"));
                        ui.horizontal(|ui| {
                            for (k, label) in [(0, icons::ARROWS_H), (1, icons::ARROWS_V)] {
                                geometry |= ui
                                    .add(
                                        egui::DragValue::new(&mut d.offset[k])
                                            .speed(0.0005)
                                            .prefix(format!("{label} "))
                                            .suffix(" m")
                                            .max_decimals(4),
                                    )
                                    .on_hover_text(tr("Sposta l'adesivo sulla superficie (orizzontale, verticale)"))
                                    .changed();
                            }
                        });
                        ui.end_row();
                        ui.label(tr("Profondità"));
                        geometry |= ui
                            .add(
                                egui::Slider::new(&mut d.depth, 0.001..=1.0)
                                    .logarithmic(true)
                                    .suffix(" m")
                                    .max_decimals(3),
                            )
                            .on_hover_text(tr(
                                "Fin dove arriva la proiezione davanti e dietro il punto: più alta per superfici \
                                 molto curve, più bassa se l'adesivo sbuca anche sul lato opposto di un oggetto sottile",
                            ))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Ruvidità"));
                        look |= ui
                            .add(egui::Slider::new(&mut d.roughness, 0.0..=1.0))
                            .on_hover_text(tr("0 = adesivo lucido (riflessi netti), 1 = carta opaca"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Metallicità"));
                        look |= ui
                            .add(egui::Slider::new(&mut d.metallic, 0.0..=1.0))
                            .on_hover_text(tr("Per etichette metallizzate (oro, argento)"))
                            .changed();
                        ui.end_row();
                    });
                    if ui
                        .button(tr("Riposiziona dalla vista"))
                        .on_hover_text(tr("Rimette l'adesivo al centro dell'oggetto visto dalla camera di adesso"))
                        .clicked()
                    {
                        reposition = Some(i);
                    }
                    ui.label(
                        egui::RichText::new(tr("Usa un PNG con la trasparenza: le parti trasparenti lasciano vedere l'oggetto"))
                            .weak()
                            .small(),
                    );
                });
        });
    }
    if let Some(i) = replace {
        if let Some(p) = pick_image() {
            let d = &mut app.scene.decals[i];
            d.height = d.width * aspect_of(&p);
            d.image = Some(p);
            geometry = true;
            look = true;
        }
    }
    if let Some(i) = reposition {
        let target = app.scene.find(&app.scene.decals[i].target);
        if let Some((o, dir, _)) = target.and_then(|t| view_ray(app, &[t])) {
            let up = app.scene.camera.up;
            let old = app.scene.decals[i].clone();
            if let Some(nd) = app
                .scene
                .new_decal(&[target.unwrap()], o, dir, up, old.width, old.height / old.width.max(1e-6))
            {
                app.scene.decals[i] = Decal {
                    n: old.n,
                    name: old.name,
                    image: old.image,
                    roughness: old.roughness,
                    metallic: old.metallic,
                    enabled: old.enabled,
                    ..nd
                };
                geometry = true;
            }
        }
    }
    if let Some(i) = remove {
        app.scene.decals.remove(i);
        app.selection.clear();
        geometry = true;
    }
    if geometry || look {
        app.scene.sync_effects();
        if geometry {
            app.geometry_changed();
        }
        app.appearance_changed();
    }
}
