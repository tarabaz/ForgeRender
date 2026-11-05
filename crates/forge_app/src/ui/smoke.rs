//! Fumo volumetrico nella scheda Effetti: il pulsante con i preset e un pannello per ogni nube
//! (riquadro, forma, densità, dettagli, quote). Il riquadro si sposta e si ridimensiona anche con
//! il gizmo nella viewport (selezionando la nube).

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::icons;
use eframe::egui;
use forge_math::Vec3;
use forge_scene::{SmokeCloud, SmokeMode, SmokeShape, SMOKE_PRESETS};

pub fn shape_name(s: SmokeShape) -> &'static str {
    match s {
        SmokeShape::Cloud => tr("Nube"),
        SmokeShape::Plume => tr("Pennacchio"),
        SmokeShape::Wisp => tr("Filo d'incenso"),
        SmokeShape::Bank => tr("Banco"),
    }
}

fn shape_hint(s: SmokeShape) -> &'static str {
    match s {
        SmokeShape::Cloud => tr("Un ammasso tondeggiante che riempie il riquadro, sospeso a mezz'aria"),
        SmokeShape::Plume => tr("Esce stretto dal fondo del riquadro, sale allargandosi e si dissolve"),
        SmokeShape::Wisp => tr("Sottile e liscio in basso, poi si rompe in riccioli salendo"),
        SmokeShape::Bank => tr("Riempie tutto il riquadro con i bordi sfilacciati"),
    }
}

/// Punto di partenza per un nuovo fumo: sopra i componenti selezionati, altrimenti al centro
/// della scena appoggiato in basso. Restituisce il centro del fondo del riquadro.
fn base_point(app: &ForgeApp) -> Vec3 {
    let mut b = forge_math::Aabb::EMPTY;
    for &i in &app.selection {
        b.grow(&app.scene.bounds_of(i));
    }
    if !b.is_empty() {
        return Vec3::new(b.center().x, b.max.y + 0.005, b.center().z);
    }
    let s = app.scene.bounds();
    if s.is_empty() {
        return Vec3::ZERO;
    }
    Vec3::new(s.center().x, s.min.y, s.center().z)
}

/// Pulsante "Fumo volumetrico" con i preset: ogni scelta aggiunge una nube nuova.
pub fn add_button(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.menu_button(tr("Fumo volumetrico"), |ui| {
        for p in &SMOKE_PRESETS {
            if ui.button(tr(p.name)).on_hover_text(tr(p.hint)).clicked() {
                let base = base_point(app);
                let mut s = (p.make)();
                let n = app.scene.smokes.iter().map(|s| s.n).max().unwrap_or(0) + 1;
                s.n = n;
                s.seed = n;
                s.name = tr(p.name).to_string();
                // La nube sospesa sta un po' sopra la base; le altre forme nascono dal fondo.
                let lift = if s.shape == SmokeShape::Cloud { s.size.y * 0.25 } else { 0.0 };
                s.center = base + Vec3::new(0.0, s.size.y * 0.5 + lift, 0.0);
                app.scene.smokes.push(s);
                app.selected_smoke = Some(app.scene.smokes.len() - 1);
                app.selected_fog = None;
                app.selected_light = None;
                app.selection.clear();
                app.shading_changed();
                ui.close();
            }
        }
    })
    .response
    .on_hover_text(tr(
        "Nubi di fumo vere, con riccioli, sbuffi e bordi sfilacciati, in un riquadro che sposti e ridimensioni \
         con il gizmo: incenso, vapore, pennacchi, fumo denso",
    ));
}

/// Pannelli delle nubi di fumo presenti.
pub fn sections(app: &mut ForgeApp, ui: &mut egui::Ui) {
    if app.scene.smokes.is_empty() {
        return;
    }
    ui.strong(tf("Fumo volumetrico ({})", &[app.scene.smokes.len().to_string()]));
    let mut changed = false;
    let mut remove = None;
    let mut select = None;
    let selected = app.selected_smoke;
    let base = base_point(app);
    let has_selection = !app.selection.is_empty();
    let preview = &mut app.smoke_preview;
    let mut select_source = false;
    for (i, s) in app.scene.smokes.iter_mut().enumerate() {
        ui.push_id(("fumo", s.n), |ui| {
            let id = ui.make_persistent_id("fumo");
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
            let opened_for = egui::Id::NULL.with("fumo-aperto-per-selezione");
            let last: Option<usize> = ui.ctx().data(|d| d.get_temp(opened_for)).flatten();
            if selected == Some(i) && last != Some(i) {
                state.set_open(true);
                ui.ctx().data_mut(|d| d.insert_temp(opened_for, Some(i)));
            }
            state
                .show_header(ui, |ui| {
                    changed |= ui
                        .checkbox(&mut s.enabled, "")
                        .on_hover_text(tr("Attivo nel render (togliendo la spunta resta nella scena, spento)"))
                        .changed();
                    if ui
                        .selectable_label(selected == Some(i), &s.name)
                        .on_hover_text(tr("Seleziona: il gizmo sposta e ridimensiona il riquadro del fumo"))
                        .clicked()
                    {
                        select = Some(i);
                    }
                    ui.label(egui::RichText::new(shape_name(s.shape)).small().weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(icons::DELETE).on_hover_text(tr("Elimina il fumo")).clicked() {
                            remove = Some(i);
                        }
                    });
                })
                .body(|ui| {
                    changed |= body(ui, s, base, has_selection, &mut select, i, preview, &mut select_source);
                });
        });
    }
    if let Some(i) = select {
        app.smoke_source_handle = select_source;
        app.selected_dust = false;
        app.selected_smoke = Some(i);
        app.selected_fog = None;
        app.selected_light = None;
        app.selection.clear();
    }
    if let Some(i) = remove {
        app.scene.smokes.remove(i);
        app.selected_smoke = None;
        changed = true;
    }
    if changed {
        app.shading_changed();
    }
}

/// Comandi del tempo nelle impostazioni del fumo: il tempo lo decide la timeline (cursore, durata e
/// inizio dell'effetto). Nascosti, non tolti: basta accendere qui per riaverli.
pub(crate) const SHOW_TIME_CONTROLS: bool = false;

/// Parametri della simulazione fisica del fumo.
fn sim_controls(ui: &mut egui::Ui, s: &mut SmokeCloud) -> bool {
    let mut changed = false;
    let m = &mut s.sim;
    changed |= crate::ui::inspector::vec3_edit(ui, tr("Sorgente (m dal fondo)"), &mut m.source_offset, 0.005);
    changed |= ui
        .checkbox(&mut m.particles, tr("Fumo a particelle (veli e filamenti sottili)"))
        .on_hover_text(tr(
            "Il fumo viaggia su particelle che non si sfumano: veli sottili che si ripiegano e filamenti lunghi (vapore, incenso, sigaretta) invece della nuvola gonfia. Calcolo più lungo",
        ))
        .changed();
    changed |= ui
        .checkbox(&mut m.air_push, tr("Spostamento d'aria degli oggetti animati"))
        .on_hover_text(tr(
            "Gli oggetti animati spingono davvero l'aria: nelle equazioni del fluido sono bordi che si muovono con la loro velocità, quindi la massa d'aria e il fumo si spostano e dietro nascono scia e vortici. Spento: il fumo si adatta attorno all'oggetto senza essere spinto",
        ))
        .changed();
    egui::Grid::new("fumo-sim").num_columns(2).show(ui, |ui| {
        if m.particles {
            changed |= slider(
                ui,
                tr("Sorgente a chiazze"),
                tr("0 = il fumo esce da tutta la sorgente, 1 = da pochi punti che si spostano piano: nastri separati, come il vapore di una tazza"),
                &mut m.patchiness,
                0.0..=1.0,
            );
        }
        ui.label(tr("Raggio della sorgente"));
        changed |= ui
            .add(egui::Slider::new(&mut m.source_radius, 0.002..=2.0).logarithmic(true).suffix(" m").max_decimals(3))
            .on_hover_text(tr("Grandezza della bocca da cui esce il fumo (sfera attorno al punto della sorgente)"))
            .changed();
        ui.end_row();
        changed |= slider(ui, tr("Quantità di fumo"), tr("Quanto fumo esce al secondo"), &mut m.emission, 0.0..=30.0);
        changed |= slider(
            ui,
            tr("Calore"),
            tr("0 = fumo freddo (non sale), 1 = caldo tipico, 3 o più = molto caldo, sale veloce"),
            &mut m.temperature,
            0.0..=10.0,
        );
        changed |= slider(ui, tr("Spinta del caldo"), tr("Quanto il caldo spinge verso l'alto"), &mut m.buoyancy, 0.0..=30.0);
        changed |= slider(
            ui,
            tr("Peso del fumo"),
            tr("Alto = gas pesante che scende e si stende sul piano (come il ghiaccio secco)"),
            &mut m.weight,
            0.0..=20.0,
        );
        changed |= slider(
            ui,
            tr("Velocità di uscita"),
            tr("Velocità con cui il fumo esce dalla sorgente (m/s): negativa = verso il basso"),
            &mut m.exit_speed,
            -3.0..=5.0,
        );
        changed |= slider(ui, tr("Raffreddamento"), tr("Quanto presto il fumo si raffredda e smette di salire"), &mut m.cooling, 0.0..=10.0);
        changed |= slider(ui, tr("Dissolvenza"), tr("Quanto presto il fumo svanisce nell'aria"), &mut m.dissipation, 0.0..=5.0);
        changed |= slider(
            ui,
            tr("Bordi sfumati"),
            tr("Fascia vicino alle pareti e al soffitto del riquadro (frazione del lato) in cui il fumo si dissolve: non si ammassa contro il riquadro e non finisce tagliato netto. 0 = pieno fino al bordo"),
            &mut m.edge_fade,
            0.0..=0.4,
        );
        changed |= slider(ui, tr("Vortici"), tr("Riccioli che nascono nel moto (confinamento della vorticità)"), &mut m.vorticity, 0.0..=20.0);
        changed |= slider(ui, tr("Turbolenza"), tr("Agitazione dell'aria: rompe la colonna in sbuffi"), &mut m.turbulence, 0.0..=20.0);
        ui.label(tr("Grandezza della turbolenza"));
        changed |= ui
            .add(egui::Slider::new(&mut m.turbulence_scale, 0.005..=5.0).logarithmic(true).suffix(" m").max_decimals(3))
            .changed();
        ui.end_row();
        if SHOW_TIME_CONTROLS {
            ui.label(tr("Istante"));
            changed |= ui
                .add(egui::Slider::new(&mut m.time, 0.1..=20.0).logarithmic(true).suffix(" s").max_decimals(2))
                .on_hover_text(tr("Secondi di simulazione: l'istante che si congela per il render"))
                .changed();
            ui.end_row();
        }
        ui.label(tr("Qualità della simulazione"));
        changed |= ui
            .add(egui::Slider::new(&mut m.quality, 1..=4))
            .on_hover_text(tr("Celle della simulazione sul lato più lungo: 1 = 48, 2 = 72, 3 = 100, 4 = 136. Più alta = moto più fine, calcolo molto più lungo"))
            .changed();
        ui.end_row();
    });
    changed |= crate::ui::inspector::vec3_edit(ui, tr("Vento (m/s)"), &mut m.wind, 0.01);
    changed
}

fn slider(ui: &mut egui::Ui, label: &str, hint: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    ui.label(label);
    let r = ui.add(egui::Slider::new(v, range).max_decimals(2)).on_hover_text(hint).changed();
    ui.end_row();
    r
}

#[allow(clippy::too_many_arguments)]
fn body(
    ui: &mut egui::Ui,
    s: &mut SmokeCloud,
    base: Vec3,
    has_selection: bool,
    select: &mut Option<usize>,
    i: usize,
    preview: &mut crate::ui::smoke_preview::SmokePreviewState,
    source: &mut bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut s.name);
    });
    ui.label(
        egui::RichText::new(tr(
            "Il riquadro è la zona in cui esiste il fumo: selezionalo e spostalo o ridimensionalo con il gizmo \
             (Sposta / Scala) nella viewport.",
        ))
        .small()
        .weak(),
    );
    let before = (s.center, s.size, s.rotation_deg);
    let moved = crate::ui::inspector::vec3_edit(ui, tr("Centro (m)"), &mut s.center, 0.01);
    let bottom = s.center.y - s.size.y * 0.5;
    if crate::ui::inspector::vec3_edit(ui, tr("Dimensioni (m)"), &mut s.size, 0.01) {
        // Il fondo resta dov'è: l'altezza cresce verso l'alto (non va sotto il piano).
        s.size = s.size.max(Vec3::splat(0.01));
        s.center.y = bottom + s.size.y * 0.5;
        *select = Some(i);
        changed = true;
    }
    if moved {
        *select = Some(i);
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Rotazione"));
        changed |= ui
            .add(
                egui::DragValue::new(&mut s.rotation_deg)
                    .speed(1.0)
                    .suffix("°")
                    .range(-360.0..=360.0),
            )
            .on_hover_text(tr("Rotazione del riquadro attorno alla verticale (anche con il gizmo Ruota)"))
            .changed();
    });
    ui.horizontal(|ui| {
        let label = if has_selection {
            tr("Appoggia sulla selezione")
        } else {
            tr("Appoggia in basso al centro")
        };
        if ui
            .small_button(label)
            .on_hover_text(tr(
                "Mette il fondo del riquadro sopra i componenti selezionati (o sul fondo della scena)",
            ))
            .clicked()
        {
            s.center = base + Vec3::new(0.0, s.size.y * 0.5, 0.0);
            changed = true;
        }
    });
    if before != (s.center, s.size, s.rotation_deg) {
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Come nasce"));
        changed |= ui
            .selectable_value(&mut s.mode, SmokeMode::Procedural, tr("Procedurale"))
            .on_hover_text(tr("Forma scelta e rumore frattale: immediato, pieno controllo della forma"))
            .changed();
        changed |= ui
            .selectable_value(&mut s.mode, SmokeMode::Simulated, tr("Simulato"))
            .on_hover_text(tr(
                "Simulazione fisica: il fumo esce da una sorgente, il caldo sale, il gas pesante scende, i vortici \
                 nascono da soli e gli oggetti lo deviano. Si congela all'istante scelto (qualche secondo di calcolo)",
            ))
            .changed();
    });
    if s.mode == SmokeMode::Simulated {
        if ui
            .button(tr("Seleziona la sorgente"))
            .on_hover_text(tr("Il gizmo sposta la sorgente del fumo (Sposta) o ne cambia il raggio (Scala); nella viewport è la sfera azzurra, si seleziona anche con un clic"))
            .clicked()
        {
            *select = Some(i);
            *source = true;
        }
        crate::ui::smoke_preview::controls(ui, preview, s.sanitized().sim.time);
        let mut full = !s.preview_only;
        if ui
            .checkbox(&mut full, tr("Calcolo vero per il render"))
            .on_hover_text(tr(
                "Spento: il fumo resta solo come anteprima a punti nella viewport (veloce) e il render non fa la \
                 simulazione completa. Utile per sistemare posizione, sorgente e come gira attorno agli oggetti; \
                 poi lo riaccendi per il render vero",
            ))
            .changed()
        {
            s.preview_only = !full;
            changed = true;
        }
        if s.preview_only {
            ui.colored_label(
                egui::Color32::from_rgb(240, 180, 80),
                tr("Solo anteprima: nel render questo fumo non c'è finché non riaccendi il calcolo vero."),
            );
        }
        if let Some((cell, r)) = s.sim_warning() {
            ui.colored_label(
                egui::Color32::from_rgb(240, 180, 80),
                tf(
                    "Riquadro grande per la sorgente: celle della simulazione di {} cm, sorgente di {} cm. Il fumo nasce in pochi grumi e un gas pesante può sparire nel pavimento: rimpicciolisci il riquadro attorno al fumo, alza la qualità o allarga la sorgente.",
                    &[format!("{:.1}", cell * 100.0), format!("{:.1}", r * 100.0)],
                ),
            );
        } else {
            ui.label(
                egui::RichText::new(tf("Celle della simulazione: {} cm", &[format!("{:.1}", s.sim_cell() * 100.0)]))
                    .small()
                    .weak(),
            );
        }
        changed |= sim_controls(ui, s);
        ui.label(
            egui::RichText::new(tr("Dettaglio fine sul bordo (sopra la simulazione):"))
                .small()
                .weak(),
        );
    }
    egui::Grid::new("fumo-grid").num_columns(2).show(ui, |ui| {
        if s.mode == SmokeMode::Procedural {
            ui.label(tr("Forma"));
            egui::ComboBox::from_id_salt("forma-fumo")
                .selected_text(shape_name(s.shape))
                .show_ui(ui, |ui| {
                    for k in SmokeShape::ALL {
                        changed |= ui
                            .selectable_value(&mut s.shape, k, shape_name(k))
                            .on_hover_text(shape_hint(k))
                            .changed();
                    }
                });
            ui.end_row();
        }
        ui.label(tr("Densità"));
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::Slider::new(&mut s.density, 0.5..=2000.0).logarithmic(true).max_decimals(1))
                .on_hover_text(tr(
                    "Quanto è spesso il fumo pieno (1/m): 5–20 = leggero e trasparente, 50–150 = denso, oltre 300 = quasi \
                     opaco. I fili sottili (incenso) ne vogliono di più per vedersi",
                ))
                .changed();
            let vis = 3.0 / s.density.max(1e-3);
            let text = if vis >= 1.0 {
                format!("{vis:.1} m")
            } else {
                format!("{:.1} cm", vis * 100.0)
            };
            ui.label(egui::RichText::new(tf("opaco oltre ~{}", &[text])).small().weak())
                .on_hover_text(tr("Spessore di fumo pieno oltre il quale non si vede più attraverso"));
        });
        ui.end_row();
        ui.label(tr("Contrasto"));
        changed |= ui
            .add(egui::Slider::new(&mut s.contrast, 0.5..=4.0).max_decimals(2))
            .on_hover_text(tr(
                "Più alto: le parti diradate diventano trasparenti e i grumi restano densi e fanno ombra su sé stessi, così un fumo leggero tiene il rilievo e i dettagli. Per un fumo trasparente ma con contrasto abbassa la densità e alza il contrasto (2–3). 1 = come prima",
            ))
            .changed();
        ui.end_row();
        ui.label(tr("Colore"));
        changed |= crate::ui::color_picker::rgb_edit_hint(
            ui,
            &mut s.color,
            tr("Bianco = vapore, grigio = fumo, scuro = fuliggine che assorbe la luce"),
        );
        ui.end_row();
        changed |= slider(
            ui,
            tr("Controluce"),
            tr("Quanto il fumo manda la luce in avanti: alto = brilla guardando verso la luce, come il fumo nei fasci"),
            &mut s.anisotropy,
            -0.5..=0.95,
        );
        ui.label(tr("Grandezza dei riccioli"));
        changed |= ui
            .add(
                egui::Slider::new(&mut s.detail_scale, 0.005..=10.0)
                    .logarithmic(true)
                    .suffix(" m")
                    .max_decimals(3),
            )
            .on_hover_text(tr(
                "Dimensione dei riccioli e degli sbuffi più grandi; i dettagli fini scendono da qui",
            ))
            .changed();
        ui.end_row();
        changed |= slider(
            ui,
            tr("Forza dei vortici"),
            tr("Quanto il fumo si attorciglia: 0 = forme morbide, 1–2 = riccioli, 4–8 = vortici estremi"),
            &mut s.turbulence,
            0.0..=8.0,
        );
        ui.label(tr("Dettaglio frattale"));
        changed |= ui
            .add(egui::Slider::new(&mut s.octaves, 1..=10))
            .on_hover_text(tr(
                "Ottave del frattale: 1–3 = morbido, 6–10 = dettagli sempre più fini (servono qualità alte)",
            ))
            .changed();
        ui.end_row();
        changed |= slider(
            ui,
            tr("Ruvidità"),
            tr("Quanto pesano i dettagli fini: bassa = bordi morbidi, alta = frastagliati"),
            &mut s.roughness,
            0.2..=0.85,
        );
        changed |= slider(
            ui,
            tr("Sbuffi"),
            tr("Lobi tondi a cavolfiore sul bordo, tipici del fumo denso che si gonfia"),
            &mut s.billows,
            0.0..=1.0,
        );
        changed |= slider(
            ui,
            tr("Filamenti"),
            tr("Quanto i bordi si sfilacciano in fili sottili"),
            &mut s.wisps,
            0.0..=1.0,
        );
        changed |= slider(
            ui,
            tr("Morbidezza dei bordi"),
            tr("0 = bordo netto, 1 = il fumo sfuma piano nell'aria"),
            &mut s.softness,
            0.0..=1.0,
        );
        changed |= slider(
            ui,
            tr("Variazione interna"),
            tr("Zone più dense e più rade dentro la nube: 0 = uniforme"),
            &mut s.variation,
            0.0..=1.0,
        );
        if s.mode == SmokeMode::Procedural && matches!(s.shape, SmokeShape::Plume | SmokeShape::Wisp) {
            changed |= slider(
                ui,
                tr("Si allarga"),
                tr("Quanto la colonna si apre salendo: 0 = resta stretta, 1 = larga come il riquadro"),
                &mut s.spread,
                0.0..=1.0,
            );
            changed |= slider(
                ui,
                tr("Serpeggia"),
                tr("Quanto la colonna ondeggia salendo"),
                &mut s.meander,
                0.0..=1.0,
            );
            changed |= slider(
                ui,
                tr("Si dissolve"),
                tr("Quanto il fumo si dirada salendo: 0 = denso fino in cima"),
                &mut s.fade,
                0.0..=1.0,
            );
        }
        let h = s.size.y;
        ui.label(tr("Altezza di partenza"));
        ui.horizontal(|ui| {
            let mut on = s.bottom_height > 0.0;
            if ui
                .checkbox(&mut on, "")
                .on_hover_text(tr("Fa cominciare il fumo più in alto del fondo del riquadro"))
                .changed()
            {
                s.bottom_height = if on { h * 0.25 } else { 0.0 };
                changed = true;
            }
            if on {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut s.bottom_height, 0.001..=h.max(0.002))
                            .suffix(" m")
                            .max_decimals(3),
                    )
                    .on_hover_text(tr("Da che altezza sopra il fondo del riquadro comincia il fumo"))
                    .changed();
            }
        });
        ui.end_row();
        ui.label(tr("Altezza massima"));
        ui.horizontal(|ui| {
            let mut on = s.top_height > 0.0;
            if ui
                .checkbox(&mut on, "")
                .on_hover_text(tr("Ferma il fumo a un'altezza sopra il fondo del riquadro"))
                .changed()
            {
                s.top_height = if on { (s.bottom_height + h * 0.5).min(h) } else { 0.0 };
                changed = true;
            }
            if on {
                let lo = (s.bottom_height + 0.002).min(h);
                changed |= ui
                    .add(
                        egui::Slider::new(&mut s.top_height, lo..=h.max(lo + 0.001))
                            .suffix(" m")
                            .max_decimals(3),
                    )
                    .on_hover_text(tr("Fino a che altezza sopra il fondo del riquadro arriva il fumo"))
                    .changed();
            }
        });
        ui.end_row();
        ui.label(tr("Niente fumo negli oggetti"));
        changed |= ui
            .checkbox(&mut s.collisions, "")
            .on_hover_text(tr("Il fumo non entra dentro gli oggetti della scena che attraversano il riquadro"))
            .changed();
        ui.end_row();
        ui.label(tr("Qualità"));
        changed |= ui
            .add(egui::Slider::new(&mut s.quality, 1..=4))
            .on_hover_text(tr(
                "Quanto è fitta la griglia del fumo: 1 = bozza (fino a 3 milioni di celle), 2 = normale (7), \
                 3 = alta (14), 4 = massima (24). Più alta = dettagli fini e bordi netti, calcolo più lungo",
            ))
            .changed();
        ui.end_row();
        ui.label(tr("Variante"));
        ui.horizontal(|ui| {
            changed |= ui.add(egui::DragValue::new(&mut s.seed).range(1..=9999)).changed();
            if ui
                .small_button(tr("Nuova"))
                .on_hover_text(tr("Altra disposizione di riccioli e sbuffi"))
                .clicked()
            {
                s.seed = s.seed % 9999 + 1;
                changed = true;
            }
        });
        ui.end_row();
    });
    changed
}
