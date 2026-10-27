//! Project-attached channel editor. Rectangles follow the recorded Cubase layout;
//! colors follow OpenTrack's current Windows theme.
use super::*;
use opentrack_core::project::ChannelEqualizer;

const HEADER: f32 = 20.0;

fn at(r: Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(r.min + Vec2::new(x, y), Vec2::new(w, h))
}

fn text(ui: &egui::Ui, r: Rect, value: &str, size: f32, align: Align2) {
    ui.painter().text(
        align.pos_in_rect(&r),
        align,
        value,
        FontId::proportional(size),
        theme::palette(ui).text,
    );
}

fn context(response: &egui::Response, state: &mut ChannelSettingsUiState) {
    show_channel_context_menu(response, state);
}

fn button(
    ui: &mut egui::Ui,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    label: &'static str,
    active: bool,
) -> egui::Response {
    mixer::control(
        ui,
        r,
        id,
        mixer::Icon::Text(label),
        active,
        false,
        true,
        None,
    )
}
fn power(
    ui: &mut egui::Ui,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    active: bool,
) -> egui::Response {
    let response = button(ui, r, id, "", active);
    let ink = project_controls::label_ink(if active {
        theme::palette(ui).accent
    } else {
        theme::toolbar_button_fill(ui.visuals())
    });
    crate::icon_system::paint_app_icon(ui, r.shrink(1.0), crate::icon_system::IconId::Power, ink);
    response
}

fn header(
    ui: &mut egui::Ui,
    r: Rect,
    label: &str,
    active: bool,
    state: &mut ChannelSettingsUiState,
) -> bool {
    let p = theme::palette(ui);
    let bar = at(r, 0.0, 0.0, r.width(), HEADER);
    ui.painter().rect_filled(bar, 0.0, p.surface_raised);
    let name = ui.interact(
        Rect::from_min_max(bar.min, bar.max - Vec2::new(45.0, 0.0)),
        ui.id().with("section-title"),
        Sense::click(),
    );
    text(
        ui,
        bar.shrink2(Vec2::new(5.0, 0.0)),
        label,
        10.0,
        Align2::LEFT_CENTER,
    );
    context(&name, state);
    let preset_indicator = mixer::control(
        ui,
        at(r, r.width() - 43.0, 1.0, 20.0, 18.0),
        "section-presets",
        mixer::Icon::Eq,
        false,
        false,
        false,
        None,
    );
    if label == "Inserts" {
        preset_indicator.on_disabled_hover_text("Shows Active Inserts");
    }
    let response = mixer::control(
        ui,
        at(r, r.width() - 21.0, 1.0, 20.0, 18.0),
        "section-bypass",
        match label {
            "Inserts" => mixer::Icon::Insert,
            "Sends" => mixer::Icon::Send,
            _ => mixer::Icon::Eq,
        },
        active,
        false,
        true,
        None,
    )
    .on_hover_text(match label {
        "Inserts" => "Inserts State (Bypass Inserts with click / Context-menu shows usage)",
        "Sends" => "Sends State (Bypass Sends with click / Context-menu shows usage)",
        _ => "EQs State (Bypass EQs with click)",
    });
    context(&response, state);
    response.clicked()
}

fn menu(response: &egui::Response, width: f32, content: impl FnOnce(&mut egui::Ui)) {
    let config =
        egui::containers::menu::MenuConfig::new().style(theme::channel_settings_menu_popup_style);
    egui::Popup::menu(response)
        .id(response.id.with("left-click-menu"))
        .width(width)
        .style(theme::channel_settings_menu_popup_style)
        .info(
            egui::UiStackInfo::new(egui::UiKind::Menu)
                .with_tag_value(egui::containers::menu::MenuConfig::MENU_CONFIG_TAG, config),
        )
        .show(content);
}

pub(super) fn show(
    ui: &mut egui::Ui,
    project: &mut Project,
    meters: &HashMap<u64, [f32; 2]>,
    peak_holds: &mut HashMap<u64, metering::Hold>,
    state: &mut ChannelSettingsUiState,
    bus_target: &mut Option<bool>,
    input_peaks: [f32; 2],
    output_peaks: [f32; 2],
    vst3_plugin_catalog: &[PluginDescriptor],
) -> bool {
    let mut names = project
        .tracks
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>();
    let count = names.len();
    names.extend([
        project.input_channel.strip.name.clone(),
        project.output_channel.strip.name.clone(),
    ]);
    let index = match *bus_target {
        Some(true) => count,
        Some(false) => count + 1,
        None => match project
            .selected_track
            .map(|i| i as usize)
            .filter(|i| *i < count)
        {
            Some(index) => index,
            None => {
                ui.label("Select a channel to edit.");
                return false;
            }
        },
    };
    state.bus_context = index >= count;
    let peak = if index == count {
        input_peaks
    } else if index == count + 1 {
        output_peaks
    } else {
        meters
            .get(&project.tracks[index].id)
            .copied()
            .unwrap_or([0.0; 2])
    };
    let track = if index == count {
        &mut project.input_channel.strip
    } else if index == count + 1 {
        &mut project.output_channel.strip
    } else {
        &mut project.tracks[index]
    };
    let color = project_controls::track_color(track);
    let area = ui.available_rect_before_wrap();
    let visible = state.preferences.visible_items.clone();
    let has_fader = visible.contains(&ChannelSettingsViewItem::Fader);
    let items = visible
        .into_iter()
        .filter(|item| !state.bus_context || item.available_for_bus())
        .filter(|item| {
            item.implemented() && !(*item == ChannelSettingsViewItem::ControlStrip && has_fader)
        })
        .filter(|item| {
            !(*item == ChannelSettingsViewItem::SendRouting
                && state
                    .preferences
                    .visible_items
                    .contains(&ChannelSettingsViewItem::Sends))
        })
        .collect::<Vec<_>>();
    let base_width = |item: ChannelSettingsViewItem| match item {
        ChannelSettingsViewItem::Inserts
        | ChannelSettingsViewItem::Sends
        | ChannelSettingsViewItem::SendRouting => 154.0,
        ChannelSettingsViewItem::Fader | ChannelSettingsViewItem::ControlStrip => 92.0,
        _ => 474.0,
    };
    let gaps = 4.0 * items.len().saturating_sub(1) as f32;
    let total: f32 = items.iter().map(|item| base_width(*item)).sum();
    let scale = ((area.width() - gaps) / total.max(1.0)).min(1.0);
    let eq_extra = (area.width() - gaps - total).max(0.0);
    let mut x = area.left();
    let mut changed = false;
    let mut next = None;
    for item in items {
        let eq = matches!(
            item,
            ChannelSettingsViewItem::Equalizers
                | ChannelSettingsViewItem::EqualizersCurve
                | ChannelSettingsViewItem::EqualizersCurveDials
        );
        let width = base_width(item) * scale + if eq { eq_extra } else { 0.0 };
        let r = Rect::from_min_size(Pos2::new(x, area.top()), Vec2::new(width, area.height()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("channel-editor", track.id, item.label()))
                .max_rect(r),
        );
        child.set_clip_rect(r);
        child.spacing_mut().item_spacing = Vec2::ZERO;
        match item {
            ChannelSettingsViewItem::Inserts => {
                changed |= inserts(&mut child, r, track, state, vst3_plugin_catalog)
            }
            ChannelSettingsViewItem::Sends | ChannelSettingsViewItem::SendRouting => {
                if track.id >= opentrack_core::project::INPUT_BUS_ID {
                    child.add_enabled_ui(false, |ui| {
                        sends(ui, r, track, state);
                    });
                } else {
                    changed |= sends(&mut child, r, track, state);
                }
            }
            ChannelSettingsViewItem::Fader | ChannelSettingsViewItem::ControlStrip => {
                changed |= strip(
                    &mut child,
                    r,
                    track,
                    &names,
                    index,
                    &mut next,
                    peak,
                    peak_holds.entry(track.id).or_default(),
                    has_fader,
                    state,
                )
            }
            _ => {
                changed |= equalizer(
                    &mut child,
                    r,
                    &mut track.equalizer,
                    item != ChannelSettingsViewItem::Equalizers,
                    item == ChannelSettingsViewItem::EqualizersCurveDials,
                    state,
                    color,
                )
            }
        }
        x += width + 4.0;
    }
    if let Some(next) = next {
        if next >= count {
            *bus_target = Some(next == count);
        } else {
            *bus_target = None;
            project.selected_track = Some(next as u32);
        }
    }
    ui.advance_cursor_after_rect(area);
    changed
}

fn slot_frame(ui: &egui::Ui, r: Rect, label: &str) {
    let p = theme::palette(ui);
    ui.painter().rect_filled(r, 1.0, p.canvas);
    ui.painter()
        .rect_stroke(r, 1.0, Stroke::new(1.0, p.grid), egui::StrokeKind::Inside);
    let bar = at(r, 0.0, 0.0, r.width(), 15.0);
    ui.painter().rect_filled(bar, 0.0, p.surface_raised);
    text(
        ui,
        bar.shrink2(Vec2::new(4.0, 0.0)),
        label,
        9.0,
        Align2::RIGHT_CENTER,
    );
    crate::icon_system::paint_app_icon(
        ui,
        Rect::from_center_size(r.min + Vec2::new(7.0, 25.0), Vec2::splat(12.0)),
        crate::icon_system::IconId::ChevronRight,
        p.muted,
    );
}

const EQ_BAND_1_FILTERS: &[(FilterType, &str)] = &[
    (FilterType::ParametricI, "Parametric I"),
    (FilterType::LowShelfI, "Low Shelf I"),
    (FilterType::HighPassI, "High Pass I"),
    (FilterType::HighPassII, "High Pass II"),
    (FilterType::ParametricII, "Parametric II"),
    (FilterType::LowShelfII, "Low Shelf II"),
    (FilterType::LowShelfIII, "Low Shelf III"),
    (FilterType::LowShelfIV, "Low Shelf IV"),
];

const EQ_PARAMETRIC_FILTERS: &[(FilterType, &str)] = &[
    (FilterType::ParametricI, "Parametric I"),
    (FilterType::ParametricII, "Parametric II"),
];

// The manual confirms EQ4 supports parametric, shelving, high-pass, and low-pass types.
// This list is limited to the corresponding filter implementations currently in the model;
// Cubase's exact EQ4 menu inventory has not been observed.
const EQ_BAND_4_FILTERS: &[(FilterType, &str)] = &[
    (FilterType::ParametricI, "Parametric I"),
    (FilterType::HighShelf, "High Shelf II"),
    (FilterType::HighPassI, "High Pass I"),
    (FilterType::HighPassII, "High Pass II"),
    (FilterType::LowPass, "Low Pass"),
    (FilterType::ParametricII, "Parametric II"),
];

fn eq_filter_types(band_index: usize) -> &'static [(FilterType, &'static str)] {
    match band_index {
        0 => EQ_BAND_1_FILTERS,
        1 | 2 => EQ_PARAMETRIC_FILTERS,
        _ => EQ_BAND_4_FILTERS,
    }
}

fn eq_filter_selected(current: FilterType, choice: FilterType) -> bool {
    current == choice
        || matches!(
            (current, choice),
            (FilterType::Bell, FilterType::ParametricI)
                | (FilterType::LowShelf, FilterType::LowShelfIV)
                | (FilterType::HighPass, FilterType::HighPassI)
        )
}

fn channel_filter_type_name(filter: FilterType) -> &'static str {
    match filter {
        // The reference's restored EQ4 state showed High Shelf II; the model stores one generic
        // HighShelf variant, so do not fabricate the unobserved I/III/IV variants.
        FilterType::HighShelf => "High Shelf II",
        _ => filter_type_name(filter),
    }
}

fn inserts(
    ui: &mut egui::Ui,
    r: Rect,
    track: &mut Track,
    state: &mut ChannelSettingsUiState,
    vst3_plugin_catalog: &[PluginDescriptor],
) -> bool {
    let mut changed = false;
    if header(
        ui,
        r,
        "Inserts",
        !track.inserts_bypassed
            && track.inserts.iter().any(|insert| {
                insert.enabled
                    && !matches!(
                        &insert.kind,
                        InsertEffectKind::LegacyUnavailableCompressor { .. }
                    )
            }),
        state,
    ) {
        track.inserts_bypassed = !track.inserts_bypassed;
        changed = true;
    }
    let row = (r.height() - 24.0) / 8.0;
    for i in 0..8 {
        let cell = at(r, 0.0, 24.0 + i as f32 * row, r.width(), row - 3.0);
        slot_frame(ui, cell, &format!("i {}", i + 1));
        let enabled = track.inserts.get(i).is_some_and(|effect| effect.enabled);
        let is_legacy_compressor = track.inserts.get(i).is_some_and(|effect| {
            matches!(
                &effect.kind,
                InsertEffectKind::LegacyUnavailableCompressor { .. }
            )
        });
        let p = power(
            ui,
            at(cell, 1.0, 1.0, 14.0, 13.0),
            ("insert-power", i),
            enabled && !is_legacy_compressor,
        )
        .on_hover_text(if is_legacy_compressor {
            "Removed built-in effect; replace it with a VST3 plug-in or remove the slot.".to_owned()
        } else {
            format!(
                "{} Insert {}",
                if enabled { "Deactivate" } else { "Activate" },
                i + 1
            )
        });
        context(&p, state);
        if p.clicked() && !is_legacy_compressor {
            if let Some(effect) = track.inserts.get_mut(i) {
                effect.enabled = !effect.enabled;
                changed = true;
            }
        }
        let type_selector_rect = at(cell, 0.0, 15.0, 14.0, cell.height() - 15.0);
        let has_effect = track.inserts.get(i).is_some();
        let type_selector = ui.interact(
            type_selector_rect,
            ui.id().with(("insert-type", i)),
            Sense::click(),
        );
        let type_selector = if has_effect {
            type_selector.on_hover_text(format!("Select Insert Type {}", i + 1))
        } else {
            type_selector.on_hover_text(format!("Activate Insert {}", i + 1))
        };
        context(&type_selector, state);
        let body = Rect::from_min_max(cell.min + Vec2::new(14.0, 15.0), cell.max);
        let response = ui.interact(body, ui.id().with(("insert", i)), Sense::click());
        context(&response, state);
        if has_effect {
            let is_vst3 = track
                .inserts
                .get(i)
                .is_some_and(|effect| matches!(&effect.kind, InsertEffectKind::Vst3 { .. }));
            if let Some(effect) = track.inserts.get(i) {
                text(
                    ui,
                    body.shrink2(Vec2::new(0.0, 0.0)),
                    &insert_effect_name(&effect.kind),
                    10.0,
                    Align2::LEFT_CENTER,
                );
            }
            let response = if is_legacy_compressor {
                response.on_hover_text(
                    "This removed built-in effect is inactive. Replace it with a VST3 plug-in.",
                )
            } else {
                response
            };
            if response.clicked() && is_vst3 {
                state.vst3_parameter_slot = Some((track.id, i));
            }
            let mut selected = None;
            menu(&type_selector, 200.0, |ui| {
                show_insert_effect_picker(ui, &mut selected, vst3_plugin_catalog)
            });
            match selected {
                Some(InsertSelection::Add(kind)) => {
                    if let Some(effect) = track.inserts.get_mut(i) {
                        let enable = matches!(kind, InsertEffectKind::Vst3 { .. });
                        effect.kind = kind;
                        if enable {
                            state.vst3_parameter_slot = Some((track.id, i));
                            effect.enabled = true;
                        }
                        changed = true;
                    }
                }
                Some(InsertSelection::Remove) => {
                    track.inserts.remove(i);
                    changed = true;
                }
                None => {}
            }
        } else {
            let mut selected = None;
            menu(&type_selector, 200.0, |ui| {
                show_insert_effect_picker(ui, &mut selected, vst3_plugin_catalog)
            });
            menu(&response, 200.0, |ui| {
                show_insert_effect_picker(ui, &mut selected, vst3_plugin_catalog)
            });
            match selected {
                Some(InsertSelection::Add(kind)) => {
                    if matches!(kind, InsertEffectKind::Vst3 { .. }) {
                        state.vst3_parameter_slot = Some((track.id, i));
                    }
                    track.inserts.insert(
                        i.min(track.inserts.len()),
                        InsertEffect {
                            enabled: true,
                            kind,
                        },
                    );
                    changed = true;
                }
                Some(InsertSelection::Remove) | None => {}
            }
        }
    }
    changed
}

fn sends(
    ui: &mut egui::Ui,
    r: Rect,
    track: &mut Track,
    state: &mut ChannelSettingsUiState,
) -> bool {
    let mut changed = false;
    if header(
        ui,
        r,
        "Sends",
        !track.sends_bypassed
            && (track.send_routing.iter().any(Option::is_some)
                || state
                    .enabled_sends
                    .iter()
                    .any(|(track_id, _)| *track_id == track.id)),
        state,
    ) {
        track.sends_bypassed = !track.sends_bypassed;
        changed = true;
    }
    let row = (r.height() - 24.0) / 8.0;
    for i in 0..8 {
        let cell = at(r, 0.0, 24.0 + i as f32 * row, r.width(), row - 3.0);
        slot_frame(ui, cell, &format!("s {}", i + 1));
        let key = (track.id, i);
        let enabled = state.enabled_sends.contains(&key);
        let power = power(
            ui,
            at(cell, 1.0, 1.0, 14.0, 13.0),
            ("send-power", i),
            enabled,
        )
        .on_hover_text(format!(
            "{} Send {}",
            if enabled { "Deactivate" } else { "Activate" },
            i + 1
        ));
        context(&power, state);
        if power.clicked() && !state.enabled_sends.remove(&key) {
            state.enabled_sends.insert(key);
        }
        let body = Rect::from_min_max(cell.min + Vec2::new(0.0, 15.0), cell.max);
        let response = ui.interact(body, ui.id().with(("send", i)), Sense::click());
        context(&response, state);
        if let Some(label) = track.send_routing.get(i).and_then(Option::as_deref) {
            text(
                ui,
                cell.shrink2(Vec2::new(13.0, 0.0)),
                label,
                10.0,
                Align2::CENTER_CENTER,
            );
        }
        let mut selected = None;
        menu(&response, 140.0, |ui| {
            if ui.button("No Bus").clicked() {
                selected = Some(None);
                ui.close();
            }
            ui.menu_button("Outputs", |ui| {
                for destination in ["Stereo Out", "Left", "Right"] {
                    if ui.button(destination).clicked() {
                        selected = Some(Some(destination.to_owned()));
                        ui.close();
                    }
                }
            });
        });
        if let Some(destination) = selected {
            track
                .send_routing
                .resize(track.send_routing.len().max(i + 1), None);
            track.send_routing[i] = destination;
            changed = true;
        }
    }
    changed
}

fn number(
    ui: &mut egui::Ui,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    suffix: &str,
    tooltip: impl Into<egui::WidgetText>,
) -> bool {
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(id).max_rect(r));
    child.spacing_mut().button_padding = Vec2::new(3.0, 0.0);
    child.spacing_mut().interact_size.y = r.height();
    child.style_mut().explanation_tooltips = false;
    child.style_mut().drag_value_text_style = egui::TextStyle::Small;
    let p = theme::palette(&child);
    child.visuals_mut().widgets.inactive.bg_fill = p.canvas;
    child.visuals_mut().widgets.inactive.weak_bg_fill = p.canvas;
    child.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
    let response = child
        .add_sized(
            r.size(),
            egui::DragValue::new(value)
                .range(range)
                .speed(speed)
                .fixed_decimals(1)
                .suffix(suffix),
        )
        .on_hover_text(tooltip);
    if response.secondary_clicked() {
        response.request_focus();
    }
    response.changed()
}

fn parameter_bar(
    ui: &mut egui::Ui,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut f32,
    min: f32,
    max: f32,
    logarithmic: bool,
    centered: bool,
    enabled: bool,
    color: Color32,
) -> bool {
    let response = ui.interact(
        r,
        ui.id().with(("eq-parameter-bar", id)),
        Sense::click_and_drag(),
    );
    let old_value = *value;
    let primary_down = ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary));
    if (response.clicked_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Primary)
        || (response.is_pointer_button_down_on() && primary_down))
        && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
    {
        let t = ((pointer.x - r.left()) / r.width()).clamp(0.0, 1.0);
        *value = if logarithmic {
            min * (max / min).powf(t)
        } else {
            min + (max - min) * t
        };
    }

    let p = theme::palette(ui);
    let rail = Rect::from_center_size(r.center(), Vec2::new(r.width(), 2.0));
    ui.painter().rect_filled(rail, 1.0, p.surface_raised);
    let t = if logarithmic {
        (value.clamp(min, max) / min).ln() / (max / min).ln()
    } else {
        (value.clamp(min, max) - min) / (max - min)
    };
    let position = rail.left() + rail.width() * t;
    let zero = rail.left() + rail.width() * 0.5;
    let fill = if centered {
        Rect::from_min_max(
            Pos2::new(position.min(zero), rail.top()),
            Pos2::new(position.max(zero), rail.bottom()),
        )
    } else {
        Rect::from_min_max(
            rail.min,
            Pos2::new(position.max(rail.left()), rail.bottom()),
        )
    };
    let hovered = enabled && response.hovered();
    let ink = if hovered {
        theme::hover_button_fill(color, ui.visuals())
    } else if enabled {
        color
    } else {
        p.muted
    };
    if fill.width() > 0.0 {
        ui.painter().rect_filled(fill, 1.0, ink);
    }
    ui.painter().rect_filled(
        Rect::from_center_size(
            Pos2::new(position, r.center().y),
            Vec2::new(if hovered { 3.0 } else { 2.0 }, r.height()),
        ),
        0.0,
        ink,
    );

    (*value - old_value).abs() > 0.0001
}

fn equalizer(
    ui: &mut egui::Ui,
    r: Rect,
    eq: &mut ChannelEqualizer,
    curve: bool,
    dials: bool,
    state: &mut ChannelSettingsUiState,
    color: Color32,
) -> bool {
    let mut changed = false;
    if header(
        ui,
        r,
        "Equalizers",
        !eq.bypassed && eq.bands.iter().any(|b| b.enabled),
        state,
    ) {
        eq.bypassed = !eq.bypassed;
        changed = true;
    }
    let p = theme::palette(ui);
    let preset = at(r, 0.0, 24.0, r.width(), 16.0);
    ui.painter().rect_filled(preset, 0.0, p.surface_raised);
    let bands_height = if dials { 125.0 } else { 98.0 };
    let bands_top = if curve {
        r.height() - bands_height
    } else {
        44.0
    };
    if curve {
        state
            .analyzer
            .controls(ui, at(r, 0.0, 22.0, r.width(), 24.0));
        changed |= graph(
            ui,
            at(r, 0.0, 50.0, r.width(), (bands_top - 54.0).max(80.0)),
            eq,
            state,
            color,
        );
    }
    let width = (r.width() - 12.0) / 4.0;
    for (i, band) in eq.bands.iter_mut().enumerate() {
        let cell = at(r, i as f32 * (width + 4.0), bands_top, width, bands_height);
        ui.painter().rect_filled(cell, 0.0, p.canvas);
        let title = at(cell, 0.0, 0.0, width, 16.0);
        ui.painter().rect_filled(title, 0.0, p.surface_raised);
        let label = ui
            .interact(
                at(cell, 18.0, 0.0, width - 18.0, 16.0),
                ui.id().with(("eq-label", i)),
                Sense::click(),
            )
            .on_hover_text(format!("EQ Band {} Active", i + 1));
        text(
            ui,
            title.shrink2(Vec2::new(4.0, 0.0)),
            &format!("eq {}", i + 1),
            10.0,
            Align2::RIGHT_CENTER,
        );
        context(&label, state);
        let response = power(
            ui,
            at(cell, 0.0, 0.0, 15.0, 15.0),
            ("eq-power", i),
            band.enabled,
        )
        .on_hover_text(format!("EQ Band {} Active", i + 1));
        if response.clicked() {
            band.enabled = !band.enabled;
            changed = true;
        }
        context(&response, state);
        changed |= number(
            ui,
            at(cell, 1.0, 18.0, width - 2.0, 18.0),
            ("eq-gain", i),
            &mut band.gain_db,
            -24.0..=24.0,
            0.1,
            "",
            format!("EQ Band {} Gain", i + 1),
        );
        changed |= parameter_bar(
            ui,
            at(cell, 4.0, 36.0, width - 8.0, 6.0),
            ("eq-gain-bar", i),
            &mut band.gain_db,
            -24.0,
            24.0,
            false,
            true,
            band.enabled && !eq.bypassed,
            color,
        );
        changed |= number(
            ui,
            at(cell, 1.0, 44.0, width - 2.0, 18.0),
            ("eq-frequency", i),
            &mut band.frequency_hz,
            20.0..=20000.0,
            1.0,
            " Hz",
            format!("EQ Band {} Freq", i + 1),
        );
        changed |= parameter_bar(
            ui,
            at(cell, 4.0, 62.0, width - 8.0, 6.0),
            ("eq-frequency-bar", i),
            &mut band.frequency_hz,
            20.0,
            20000.0,
            true,
            false,
            band.enabled && !eq.bypassed,
            color,
        );
        let filter_rect = at(cell, 1.0, 70.0, width * 0.66, 18.0);
        let response = ui.interact(
            filter_rect,
            ui.id().with(("filter-type", i)),
            Sense::click(),
        );
        if response.hovered() {
            ui.painter().rect_filled(
                filter_rect,
                1.0,
                theme::hover_button_fill(p.surface_raised, ui.visuals()),
            );
            ui.painter().rect_stroke(
                filter_rect,
                1.0,
                ui.visuals().widgets.hovered.bg_stroke,
                egui::StrokeKind::Inside,
            );
        }
        text(
            ui,
            filter_rect,
            channel_filter_type_name(band.filter),
            9.0,
            Align2::LEFT_CENTER,
        );
        context(&response, state);
        menu(&response, 150.0, |ui| {
            for (kind, label) in eq_filter_types(i).iter().copied() {
                if channel_menu_item(ui, eq_filter_selected(band.filter, kind), label).clicked() {
                    band.filter = kind;
                    changed = true;
                    ui.close();
                }
            }
        });
        changed |= number(
            ui,
            at(cell, width * 0.68, 70.0, width * 0.31, 18.0),
            ("eq-q", i),
            &mut band.q,
            0.1..=10.0,
            0.01,
            "",
            format!("EQ Band {} Q", i + 1),
        );
        changed |= parameter_bar(
            ui,
            at(cell, 4.0, 88.0, width - 8.0, 6.0),
            ("eq-q-bar", i),
            &mut band.q,
            0.1,
            10.0,
            true,
            false,
            band.enabled && !eq.bypassed,
            color,
        );
        if dials {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("eq-dials", i))
                    .max_rect(at(cell, 0.0, 99.0, width, 24.0)),
                |ui| {
                    ui.horizontal(|ui| {
                        show_eq_dial(ui, &mut band.gain_db, -24.0, 24.0, false, &mut changed);
                        show_eq_dial(
                            ui,
                            &mut band.frequency_hz,
                            20.0,
                            20000.0,
                            true,
                            &mut changed,
                        );
                        show_eq_dial(ui, &mut band.q, 0.1, 10.0, true, &mut changed);
                    });
                },
            );
        }
    }
    changed
}

#[derive(Clone, Copy)]
struct GraphDrag {
    band: usize,
    offset: Vec2,
}

fn node_position(plot: Rect, frequency: f32, gain: f32) -> Pos2 {
    Pos2::new(
        plot.left() + plot.width() * (frequency.clamp(20.0, 20000.0) / 20.0).ln() / 1000.0_f32.ln(),
        plot.center().y - gain / 48.0 * plot.height(),
    )
}

fn graph(
    ui: &mut egui::Ui,
    r: Rect,
    eq: &mut ChannelEqualizer,
    state: &mut ChannelSettingsUiState,
    color: Color32,
) -> bool {
    let p = theme::palette(ui);
    let plot = r.shrink2(Vec2::new(30.0, 18.0));
    let response = ui.interact(r, ui.id().with("eq-graph"), Sense::click_and_drag());
    let hover_position = response.hover_pos();
    let drag_id = response.id.with("drag");
    let mut changed = false;
    let pressed = response.is_pointer_button_down_on()
        && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    if pressed
        || response.drag_started_by(egui::PointerButton::Primary)
        || response.clicked_by(egui::PointerButton::Primary)
    {
        if let Some(pos) = ui
            .input(|i| i.pointer.press_origin())
            .or(response.interact_pointer_pos())
        {
            let nearest = eq
                .bands
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    node_position(plot, a.frequency_hz, a.gain_db)
                        .distance_sq(pos)
                        .total_cmp(&node_position(plot, b.frequency_hz, b.gain_db).distance_sq(pos))
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            let node = node_position(
                plot,
                eq.bands[nearest].frequency_hz,
                eq.bands[nearest].gain_db,
            );
            let offset = if node.distance(pos) < 14.0 {
                node - pos
            } else {
                Vec2::ZERO
            };
            ui.data_mut(|d| {
                d.insert_temp(
                    drag_id,
                    GraphDrag {
                        band: nearest,
                        offset,
                    },
                )
            });
            changed |= !eq.bands[nearest].enabled || eq.bypassed;
            eq.bands[nearest].enabled = true;
            eq.bypassed = false;
        }
    }
    if pressed
        || response.dragged_by(egui::PointerButton::Primary)
        || response.clicked_by(egui::PointerButton::Primary)
    {
        if let (Some(pos), Some(drag)) = (
            response.interact_pointer_pos(),
            ui.data(|d| d.get_temp::<GraphDrag>(drag_id)),
        ) {
            let pos = pos + drag.offset;
            let band = &mut eq.bands[drag.band];
            let frequency =
                20.0 * 1000.0_f32.powf(((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0));
            let gain = ((plot.center().y - pos.y) / plot.height() * 48.0).clamp(-24.0, 24.0);
            changed |= (band.frequency_hz - frequency).abs() > 0.001
                || (band.gain_db - gain).abs() > 0.001;
            band.frequency_hz = frequency;
            band.gain_db = gain;
        }
    }
    ui.painter().rect_filled(r, 1.0, p.canvas);
    for gain in [-24, -18, -12, -6, 0, 6, 12, 18, 24] {
        let y = node_position(plot, 20.0, gain as f32).y;
        ui.painter().line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.0, p.surface_raised),
        );
        text(
            ui,
            Rect::from_min_max(
                Pos2::new(r.left(), y - 6.0),
                Pos2::new(plot.left() - 4.0, y + 6.0),
            ),
            &gain.to_string(),
            8.0,
            Align2::RIGHT_CENTER,
        );
    }
    for frequency in [
        20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
    ] {
        let x = node_position(plot, frequency, 0.0).x;
        ui.painter().line_segment(
            [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
            Stroke::new(1.0, p.surface_raised),
        );
        let label = if frequency >= 1000.0 {
            format!("{}k", frequency / 1000.0)
        } else {
            format!("{frequency:.0}")
        };
        text(
            ui,
            Rect::from_center_size(Pos2::new(x, r.bottom() - 7.0), Vec2::new(30.0, 12.0)),
            &label,
            8.0,
            Align2::CENTER_CENTER,
        );
    }
    state.analyzer.paint(ui, plot, color);
    let points = (0..=256)
        .map(|i| {
            let frequency = 20.0 * 1000.0_f32.powf(i as f32 / 256.0);
            let gain = opentrack_audio::channel_equalizer_response_db(
                eq,
                frequency,
                state.analyzer.rate(None),
            )
            .clamp(-24.0, 24.0);
            node_position(plot, frequency, gain)
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.5, color)));
    if response.hovered() {
        ui.painter().rect_stroke(
            r.shrink(0.5),
            1.0,
            ui.visuals().widgets.hovered.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    for (i, band) in eq.bands.iter().enumerate() {
        let pos = node_position(plot, band.frequency_hz, band.gain_db);
        let active = band.enabled && !eq.bypassed;
        let hovered = hover_position.is_some_and(|pointer| pos.distance(pointer) <= 9.0);
        let node_fill = if hovered {
            theme::hover_button_fill(color, ui.visuals())
        } else if active {
            color
        } else {
            p.surface_raised
        };
        ui.painter()
            .circle_filled(pos, if hovered { 4.8 } else { 4.0 }, node_fill);
        ui.painter().circle_stroke(
            pos,
            if hovered { 6.0 } else { 5.0 },
            Stroke::new(1.0, if hovered || active { color } else { p.muted }),
        );
        text(
            ui,
            Rect::from_center_size(pos - Vec2::new(0.0, 11.0), Vec2::splat(12.0)),
            &(i + 1).to_string(),
            9.0,
            Align2::CENTER_CENTER,
        );
    }
    changed
}

fn strip(
    ui: &mut egui::Ui,
    r: Rect,
    track: &mut Track,
    names: &[String],
    index: usize,
    next: &mut Option<usize>,
    peak: [f32; 2],
    held_peak: &mut metering::Hold,
    fader: bool,
    state: &mut ChannelSettingsUiState,
) -> bool {
    let mut changed = false;
    let p = theme::palette(ui);
    let title = at(r, 0.0, 0.0, r.width(), 20.0);
    let color = project_controls::track_color(track);
    ui.painter().rect_filled(title, 0.0, color);
    if state.editing_track_id == Some(track.id) {
        let mut edit = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("rename", track.id))
                .max_rect(title),
        );
        let mut out = egui::TextEdit::singleline(&mut state.rename_draft)
            .font(FontId::proportional(11.0))
            .desired_width(title.width())
            .show(&mut edit);
        if state.rename_focus_requested {
            out.response.request_focus();
            out.state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(state.rename_draft.chars().count()),
                )));
            out.state.store(ui.ctx(), out.response.id);
            state.rename_focus_requested = false;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.editing_track_id = None;
        } else if out.response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let name = state.rename_draft.trim();
            if !name.is_empty() && name != track.name {
                track.name = name.to_owned();
                changed = true;
            }
            state.editing_track_id = None;
        }
    } else {
        let response = ui
            .interact(title, ui.id().with("channel-name"), Sense::click())
            .on_hover_text("Channel Name (Use double-click to edit)");
        ui.painter().text(
            title.left_center() + Vec2::new(4.0, 0.0),
            Align2::LEFT_CENTER,
            &track.name,
            FontId::proportional(11.0),
            project_controls::label_ink(color),
        );
        context(&response, state);
        if response.double_clicked() {
            state.editing_track_id = Some(track.id);
            state.rename_draft = track.name.clone();
            state.rename_focus_requested = true;
        }
    }
    let selector = ui.interact(
        at(r, 0.0, 23.0, r.width(), 18.0),
        ui.id().with("channel-selector"),
        Sense::click(),
    );
    let c = selector.rect.left_center() + Vec2::new(8.0, 0.0);
    crate::icon_system::paint_app_icon(
        ui,
        Rect::from_center_size(c, Vec2::splat(12.0)),
        crate::icon_system::IconId::ChevronDown,
        p.muted,
    );
    text(
        ui,
        selector.rect,
        &(index + 1).to_string(),
        12.0,
        Align2::RIGHT_CENTER,
    );
    context(&selector, state);
    menu(&selector, 160.0, |ui| {
        for (i, name) in names.iter().enumerate() {
            if channel_menu_item(ui, i == index, name).clicked() {
                *next = Some(i);
                ui.close();
            }
        }
    });
    let is_bus = track.id >= opentrack_core::project::INPUT_BUS_ID;
    for input in [true, false] {
        let field = at(
            r,
            2.0,
            if input { 48.0 } else { 63.0 },
            r.width() - 4.0,
            14.0,
        );
        ui.painter().rect_filled(field, 0.0, p.canvas);
        let label = if is_bus {
            if track.id == opentrack_core::project::INPUT_BUS_ID {
                if input {
                    "Device Input 1/2"
                } else {
                    "To Tracks"
                }
            } else if input {
                "From Tracks"
            } else {
                "Device Output 1/2"
            }
        } else if input {
            track
                .input_routing
                .as_deref()
                .unwrap_or(if track.audio_channels == 1 {
                    "Left - Stereo In"
                } else {
                    "Stereo In"
                })
        } else {
            track.output_routing.as_deref().unwrap_or("Stereo Out")
        };
        text(
            ui,
            field.shrink2(Vec2::new(2.0, 0.0)),
            label,
            10.0,
            Align2::LEFT_CENTER,
        );
        let response = ui
            .interact(field, ui.id().with(("routing", input)), Sense::click())
            .on_hover_text(if input {
                "Input Routing (all selected with Shift + ALT + click)"
            } else {
                "Output Routing (all selected with Shift + ALT + click)"
            });
        context(&response, state);
        let mut selected = None;
        if !is_bus {
            menu(&response, 170.0, |ui| {
                if ui.button("No Bus").clicked() {
                    selected = Some("No Bus".to_owned());
                    ui.close();
                }
                if input {
                    for label in ["Stereo In", "Left - Stereo In", "Right - Stereo In"] {
                        if ui
                            .add_enabled(
                                label != "Stereo In" || track.audio_channels != 1,
                                egui::Button::new(label),
                            )
                            .clicked()
                        {
                            selected = Some(label.to_owned());
                            ui.close();
                        }
                    }
                } else {
                    for label in ["Stereo Out", "Left", "Right"] {
                        if ui.button(label).clicked() {
                            selected = Some(label.to_owned());
                            ui.close();
                        }
                    }
                }
            });
        }
        if let Some(value) = selected {
            if input {
                track.input_routing = Some(value);
            } else {
                track.output_routing = Some(value);
            }
            changed = true;
        }
    }
    text(
        ui,
        at(r, 2.0, 79.0, 80.0, 13.0),
        if track.audio_channels == 1 {
            "mono"
        } else {
            "stereo"
        },
        10.0,
        Align2::LEFT_CENTER,
    );
    let phase = mixer::control(
        ui,
        at(r, 4.0, 96.0, 16.0, 15.0),
        "phase",
        mixer::Icon::Phase,
        track.input_phase_inverted,
        false,
        true,
        None,
    )
    .on_hover_text("Input Phase");
    context(&phase, state);
    if phase.clicked() {
        track.input_phase_inverted = !track.input_phase_inverted;
        changed = true;
    }
    changed |= number(
        ui,
        at(r, 24.0, 96.0, 29.0, 15.0),
        "input-gain-value",
        &mut track.input_gain_db,
        -24.0..=24.0,
        0.1,
        "",
        "Input Gain",
    );
    let knob = at(r, r.width() - 29.0, 91.0, 24.0, 24.0);
    let gain = ui
        .interact(knob, ui.id().with("input-gain"), Sense::click_and_drag())
        .on_hover_text("Input Gain (use shift)");
    context(&gain, state);
    if gain.dragged() {
        let speed = if ui.input(|i| i.modifiers.shift) {
            0.01
        } else {
            0.1
        };
        track.input_gain_db =
            (track.input_gain_db - ui.input(|i| i.pointer.delta().y) * speed).clamp(-24.0, 24.0);
        changed = true;
    }
    mixer::knob_graphic(ui, knob, track.input_gain_db);
    let body = at(r, 1.0, 123.0, r.width() - 2.0, r.height() - 125.0);
    ui.painter().rect_filled(body, 4.0, p.surface_raised);
    ui.painter().rect_stroke(
        body,
        4.0,
        Stroke::new(1.0, p.muted),
        egui::StrokeKind::Inside,
    );
    let pan = at(r, 6.0, 132.0, r.width() - 12.0, 14.0);
    ui.painter().rect_filled(pan, 1.0, p.canvas);
    let response = ui.interact(pan, ui.id().with("pan"), Sense::click_and_drag());
    context(&response, state);
    if response.hovered() {
        ui.painter()
            .rect_filled(pan, 1.0, theme::hover_button_fill(p.canvas, ui.visuals()));
        ui.painter().rect_stroke(
            pan,
            1.0,
            ui.visuals().widgets.hovered.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            track.pan = ((pos.x - pan.left()) / pan.width() * 2.0 - 1.0).clamp(-1.0, 1.0);
            changed = true;
        }
    }
    let x = pan.center().x + track.pan * pan.width() * 0.46;
    ui.painter().line_segment(
        [
            Pos2::new(x, pan.top() + 2.0),
            Pos2::new(x, pan.bottom() - 2.0),
        ],
        Stroke::new(2.0, p.accent),
    );
    let pan_value = at(r, 6.0, 149.0, 28.0, 18.0);
    let mut pan_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("pan-value")
            .max_rect(pan_value),
    );
    changed |= pan_ui
        .add_sized(
            pan_value.size(),
            egui::DragValue::new(&mut track.pan)
                .range(-1.0..=1.0)
                .speed(0.01)
                .custom_formatter(|value, _| {
                    if value.abs() < 0.005 {
                        "C".to_owned()
                    } else {
                        format!(
                            "{}{}",
                            if value < 0.0 { "L" } else { "R" },
                            (value.abs() * 100.0).round()
                        )
                    }
                })
                .custom_parser(|text| {
                    let text = text.trim().to_uppercase();
                    if text == "C" {
                        Some(0.0)
                    } else if let Some(v) = text.strip_prefix('L') {
                        v.parse::<f64>().ok().map(|v| -v / 100.0)
                    } else if let Some(v) = text.strip_prefix('R') {
                        v.parse::<f64>().ok().map(|v| v / 100.0)
                    } else {
                        None
                    }
                }),
        )
        .changed();
    for (i, (label, active, tip, y)) in [
        ("M", track.muted, "Mute", 173.0),
        (
            "S",
            track.solo || track.solo_defeat,
            "Solo State (Solo Defeat with ALT click)",
            190.0,
        ),
        ("L", track.listen, "Listen", 210.0),
        ("R", track.automation_read, "Read Automation", 233.0),
        ("W", track.automation_write, "Write Automation", 250.0),
    ]
    .into_iter()
    .enumerate()
    {
        let response = mixer::control(
            ui,
            at(r, 9.0, y, 21.0, 15.0),
            ("flag", i),
            mixer::Icon::Text(label),
            active,
            false,
            !is_bus || i == 0,
            Some(tip),
        );
        context(&response, state);
        if response.clicked() {
            match i {
                0 => track.muted = !track.muted,
                1 => {
                    if ui.input(|i| i.modifiers.alt) {
                        track.solo_defeat = !track.solo_defeat;
                    } else {
                        track.solo = !track.solo;
                    }
                }
                2 => track.listen = !track.listen,
                3 => track.automation_read = !track.automation_read,
                _ => track.automation_write = !track.automation_write,
            };
            changed = true;
        }
    }
    for (i, icon) in [mixer::Icon::Insert, mixer::Icon::Eq, mixer::Icon::Send]
        .into_iter()
        .enumerate()
    {
        if is_bus && i == 2 {
            continue;
        }
        let response = mixer::control(
            ui,
            at(r, 9.0, 286.0 + i as f32 * 18.0, 21.0, 15.0),
            ("rack-state", i),
            icon,
            match i {
                0 => !track.inserts_bypassed && track.inserts.iter().any(|insert| insert.enabled),
                1 => {
                    !track.equalizer.bypassed
                        && track.equalizer.bands.iter().any(|band| band.enabled)
                }
                _ => {
                    !track.sends_bypassed
                        && (track.send_routing.iter().any(Option::is_some)
                            || state
                                .enabled_sends
                                .iter()
                                .any(|(track_id, _)| *track_id == track.id))
                }
            },
            false,
            !is_bus || i < 2,
            None,
        )
        .on_hover_text(match i {
            0 => "Inserts State (Bypass Inserts with click / Context-menu shows usage)",
            1 => "EQs State (Bypass EQs with click)",
            _ => "Sends State (Bypass Sends with click / Context-menu shows usage)",
        });
        context(&response, state);
        if response.clicked() {
            match i {
                0 => track.inserts_bypassed = !track.inserts_bypassed,
                1 => track.equalizer.bypassed = !track.equalizer.bypassed,
                _ => track.sends_bypassed = !track.sends_bypassed,
            };
            changed = true;
        }
    }
    for (record, y) in [(false, r.height() - 62.0), (true, r.height() - 39.0)] {
        if is_bus {
            continue;
        }
        let response = mixer::control(
            ui,
            at(
                r,
                record.then_some(6.0).unwrap_or(9.0),
                y,
                if record { 27.0 } else { 21.0 },
                if record { 27.0 } else { 21.0 },
            ),
            if record { "record" } else { "monitor" },
            if record {
                mixer::Icon::Record
            } else {
                mixer::Icon::Speaker
            },
            if record { track.armed } else { track.monitor },
            true,
            true,
            None,
        )
        .on_hover_text(if record { "Record Enable" } else { "Monitor" });
        context(&response, state);
        if response.clicked() {
            if record {
                track.armed = !track.armed;
            } else {
                track.monitor = !track.monitor;
            }
            changed = true;
        }
    }
    if fader {
        let virtual_rect = Rect::from_min_max(
            r.min + Vec2::new((r.width() - 82.0) / 2.0, 50.0),
            Pos2::new(r.center().x + 41.0, r.bottom() + 27.0),
        );
        let fader_color = project_controls::track_color(track);
        let (edit, response) = mixer::channel_settings_fader(
            ui,
            virtual_rect,
            &mut track.gain,
            peak,
            track.audio_channels,
            track.id,
            held_peak,
            fader_color,
        );
        changed |= edit;
        context(&response, state);
    }
    changed
}

pub(super) fn setup(ctx: &egui::Context, state: &mut ChannelSettingsUiState) {
    if !state.setup_open {
        return;
    }
    let mut open = true;
    let mut done = false;
    egui::Window::new("Setup...")
        .id(Id::new("channel-settings-view-setup"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_size(Vec2::new(648.0, 320.0))
        .show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(648.0, 320.0), Sense::hover());
            let p = theme::palette(ui);
            let left = at(r, 0.0, 18.0, 266.0, 226.0);
            let right = at(r, 380.0, 18.0, 268.0, 226.0);
            text(
                ui,
                at(r, 0.0, 0.0, 266.0, 16.0),
                "Visible Items",
                11.0,
                Align2::LEFT_CENTER,
            );
            text(
                ui,
                at(r, 380.0, 0.0, 268.0, 16.0),
                "Hidden Items",
                11.0,
                Align2::LEFT_CENTER,
            );
            for list in [left, right] {
                ui.painter().rect_filled(list, 0.0, p.canvas);
                ui.painter().rect_stroke(
                    list,
                    0.0,
                    Stroke::new(1.0, p.grid),
                    egui::StrokeKind::Inside,
                );
            }
            let mut list_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("visible-layout-items")
                    .max_rect(left.shrink(2.0)),
            );
            egui::ScrollArea::vertical()
                .id_salt("visible-scroll")
                .show(&mut list_ui, |ui| {
                    for (i, item) in state.preferences.visible_items.iter().enumerate() {
                        if state.bus_context && !item.available_for_bus() {
                            continue;
                        }
                        if ui
                            .selectable_label(state.setup_selected_visible == Some(i), item.label())
                            .clicked()
                        {
                            state.setup_selected_visible = Some(i);
                        }
                    }
                });
            let hidden = ChannelSettingsViewItem::ALL
                .into_iter()
                .filter(|item| !state.bus_context || item.available_for_bus())
                .filter(|item| !state.preferences.visible_items.contains(item))
                .collect::<Vec<_>>();
            let mut list_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("hidden-layout-items")
                    .max_rect(right.shrink(2.0)),
            );
            egui::ScrollArea::vertical()
                .id_salt("hidden-scroll")
                .show(&mut list_ui, |ui| {
                    for item in hidden {
                        let label = if item.implemented() {
                            item.label().to_owned()
                        } else {
                            format!("{} (not implemented)", item.label())
                        };
                        if ui
                            .add_enabled(
                                item.implemented(),
                                egui::Button::selectable(
                                    state.setup_selected_hidden == Some(item),
                                    label,
                                ),
                            )
                            .clicked()
                        {
                            state.setup_selected_hidden = Some(item);
                        }
                    }
                });
            let selected = state.setup_selected_visible.filter(|i| {
                *i < state.preferences.visible_items.len()
                    && (!state.bus_context
                        || state.preferences.visible_items[*i].available_for_bus())
            });
            if ui
                .put(
                    at(r, 284.0, 18.0, 80.0, 22.0),
                    egui::Button::new(">>>>>").sense(if selected.is_some() {
                        Sense::click()
                    } else {
                        Sense::hover()
                    }),
                )
                .clicked()
            {
                if let Some(i) = selected {
                    state.preferences.visible_items.remove(i);
                    state.setup_selected_visible = None;
                }
            }
            if ui
                .put(at(r, 284.0, 58.0, 80.0, 22.0), egui::Button::new("<<<<<"))
                .clicked()
            {
                if let Some(item) = state.setup_selected_hidden.take() {
                    set_channel_view_item(state, item);
                }
            }
            if ui
                .put(at(r, 284.0, 98.0, 80.0, 22.0), egui::Button::new("Move up"))
                .clicked()
            {
                if let Some(i) = selected {
                    if let Some(previous) = (0..i).rev().find(|j| {
                        !state.bus_context
                            || state.preferences.visible_items[*j].available_for_bus()
                    }) {
                        state.preferences.visible_items.swap(i, previous);
                        state.setup_selected_visible = Some(previous);
                    }
                }
            }
            if ui
                .put(
                    at(r, 284.0, 138.0, 80.0, 22.0),
                    egui::Button::new("Move Down"),
                )
                .clicked()
            {
                if let Some(i) = selected {
                    if let Some(next) = (i + 1..state.preferences.visible_items.len()).find(|j| {
                        !state.bus_context
                            || state.preferences.visible_items[*j].available_for_bus()
                    }) {
                        state.preferences.visible_items.swap(i, next);
                        state.setup_selected_visible = Some(next);
                    }
                }
            }
            text(
                ui,
                at(r, 0.0, 251.0, 110.0, 16.0),
                "Presets",
                11.0,
                Align2::LEFT_CENTER,
            );
            ui.put(
                at(r, 0.0, 269.0, 86.0, 19.0),
                egui::TextEdit::singleline(&mut state.preset_name_draft).hint_text("-"),
            );
            let arrow = button(
                ui,
                at(r, 87.0, 269.0, 14.0, 19.0),
                "preset-selector",
                "",
                false,
            );
            crate::icon_system::paint_app_icon(
                ui,
                arrow.rect,
                crate::icon_system::IconId::ChevronDown,
                p.text,
            );
            menu(&arrow, 160.0, |ui| {
                for (i, preset) in state.preferences.presets.iter().enumerate() {
                    if channel_menu_item(ui, state.selected_preset == Some(i), &preset.name)
                        .clicked()
                    {
                        state.selected_preset = Some(i);
                        state.preferences.visible_items = preset.visible_items.clone();
                        state.preset_name_draft = preset.name.clone();
                        ui.close();
                    }
                }
            });
            let save = button(
                ui,
                at(r, 110.0, 269.0, 18.0, 18.0),
                "save-layout-preset",
                "",
                false,
            );
            let ink = project_controls::label_ink(theme::toolbar_button_fill(ui.visuals()));
            crate::icon_system::paint_app_icon(
                ui,
                save.rect,
                crate::icon_system::IconId::SavePreset,
                ink,
            );
            if save.clicked() {
                let name = state.preset_name_draft.trim();
                if !name.is_empty() {
                    if let Some(i) = state
                        .preferences
                        .presets
                        .iter()
                        .position(|preset| preset.name == name)
                    {
                        state.preferences.presets[i].visible_items =
                            state.preferences.visible_items.clone();
                        state.selected_preset = Some(i);
                    } else {
                        state.preferences.presets.push(ChannelSettingsLayoutPreset {
                            name: name.to_owned(),
                            visible_items: state.preferences.visible_items.clone(),
                        });
                        state.selected_preset = Some(state.preferences.presets.len() - 1);
                    }
                }
            }
            let delete = button(
                ui,
                at(r, 136.0, 269.0, 18.0, 18.0),
                "delete-layout-preset",
                "",
                false,
            );
            ui.painter().rect_stroke(
                delete.rect.shrink2(Vec2::new(5.0, 4.0)),
                1.0,
                Stroke::new(1.0, ink),
                egui::StrokeKind::Inside,
            );
            ui.painter().line_segment(
                [
                    delete.rect.min + Vec2::new(4.0, 4.0),
                    delete.rect.min + Vec2::new(14.0, 4.0),
                ],
                Stroke::new(1.0, ink),
            );
            if delete.clicked() {
                if let Some(i) = state.selected_preset.filter(|i| {
                    state
                        .preferences
                        .presets
                        .get(*i)
                        .is_some_and(|p| p.name != "Default")
                }) {
                    state.preferences.presets.remove(i);
                    state.selected_preset = None;
                    state.preset_name_draft.clear();
                }
            }
            if ui
                .put(at(r, 568.0, 294.0, 80.0, 24.0), egui::Button::new("OK"))
                .clicked()
            {
                done = true;
            }
        });
    state.setup_open = open && !done;
}

#[cfg(test)]
mod bus_tests {
    use super::*;
    #[test]
    fn both_bus_editors_render_with_no_arrange_tracks_or_selected_track() {
        let ctx = egui::Context::default();
        let mut project = Project::default();
        let mut state = ChannelSettingsUiState::default();
        let meters = HashMap::new();
        for input in [true, false] {
            let mut target = Some(input);
            for _ in 0..3 {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(902.0, 430.0))),
                        ..Default::default()
                    },
                    |ui| {
                        assert!(!show(
                            ui,
                            &mut project,
                            &meters,
                            &mut HashMap::new(),
                            &mut state,
                            &mut target,
                            [0.1, 0.2],
                            [0.3, 0.4],
                            &[],
                        ));
                    },
                );
                assert!(
                    !out.shapes.iter().any(
                        |s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Sends")
                    ),
                    "device buses must not render a Sends rack"
                );
                assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Post Inserts/EQ")), "analyzer stage control is rendered in bus editor");
                let _ = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
                out.textures_delta.clear();
            }
        }
        assert!(
            state
                .preferences
                .visible_items
                .contains(&ChannelSettingsViewItem::Sends),
            "bus filtering must preserve normal track preferences"
        );
        project.add_track(TrackKind::Audio);
        let mut target = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(902.0, 430.0))),
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    &mut project,
                    &meters,
                    &mut HashMap::new(),
                    &mut state,
                    &mut target,
                    [0.0; 2],
                    [0.0; 2],
                    &[],
                );
            },
        );
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Sends")),
            "normal tracks retain Sends"
        );
        out.textures_delta.clear();
        assert_eq!(project.tracks.len(), 1);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn eq_parameter_bar_uses_the_channel_color() {
        let color = Color32::from_rgb(76, 142, 210);
        let ctx = egui::Context::default();
        let mut value = 0.0_f32;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(!parameter_bar(
                ui,
                Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 6.0)),
                "color-test",
                &mut value,
                -24.0,
                24.0,
                false,
                true,
                true,
                color,
            ));
        });
        assert!(
            out.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.fill == color
            )),
            "enabled EQ parameter bars must draw in the channel color"
        );
        out.textures_delta.clear();
    }
}
