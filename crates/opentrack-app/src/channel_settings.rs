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

fn group_route_would_cycle(tracks: &[Track], track_id: u64, target_group_id: u64) -> bool {
    let mut current = target_group_id;
    let mut visited = std::collections::HashSet::new();
    loop {
        if current == track_id {
            return true;
        }
        if !visited.insert(current) {
            return true;
        }
        let Some(next) = tracks
            .iter()
            .find(|track| track.id == current)
            .and_then(|track| track.output_routing.as_deref())
            .and_then(|route| route.strip_prefix("Group:"))
            .and_then(|id| id.parse::<u64>().ok())
        else {
            return false;
        };
        current = next;
    }
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
    let selected_track_id = project
        .selected_track
        .map(|index| index as usize)
        .and_then(|index| project.tracks.get(index))
        .map(|track| track.id);
    let group_options = project
        .tracks
        .iter()
        .filter(|candidate| candidate.kind == TrackKind::Group)
        .filter(|candidate| Some(candidate.id) != selected_track_id)
        .filter(|candidate| {
            selected_track_id.is_none_or(|track_id| {
                !group_route_would_cycle(&project.tracks, track_id, candidate.id)
            })
        })
        .map(|candidate| (candidate.id, candidate.name.clone()))
        .collect::<Vec<_>>();
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
                state.eq_solo = None;
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
    let channel_id = track.id;
    if state
        .eq_solo
        .is_some_and(|(solo_channel, _)| solo_channel != channel_id)
    {
        state.eq_solo = None;
    }
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
                    &group_options,
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
                    channel_id,
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

const EQ_FILTER_TYPES: &[(FilterType, &str)] = &[
    (FilterType::ParametricI, "Bell · Constant Q"),
    (FilterType::ParametricII, "Bell · Proportional Q"),
    (FilterType::Notch, "Notch"),
    (FilterType::LowShelf, "Low Shelf · Classic"),
    (FilterType::LowShelfSoft, "Low Shelf · Smooth"),
    (FilterType::LowShelfSteep, "Low Shelf · Steep"),
    (FilterType::LowShelfResonant, "Low Shelf · Resonant"),
    (FilterType::HighShelf, "High Shelf · Classic"),
    (FilterType::HighShelfSoft, "High Shelf · Smooth"),
    (FilterType::HighShelfSteep, "High Shelf · Steep"),
    (FilterType::HighShelfResonant, "High Shelf · Resonant"),
    (FilterType::LowPass, "Low Pass 12 dB/oct"),
    (FilterType::LowPassII, "Low Pass 24 dB/oct"),
    (FilterType::HighPassI, "High Pass 12 dB/oct"),
    (FilterType::HighPassII, "High Pass 24 dB/oct"),
];

fn eq_filter_types(_band_index: usize) -> &'static [(FilterType, &'static str)] {
    EQ_FILTER_TYPES
}

fn eq_filter_selected(current: FilterType, choice: FilterType) -> bool {
    current == choice
        || matches!(
            (current, choice),
            (FilterType::Bell, FilterType::ParametricI)
                | (
                    FilterType::LowShelfI
                        | FilterType::LowShelfII
                        | FilterType::LowShelfIII
                        | FilterType::LowShelfIV,
                    FilterType::LowShelf
                )
                | (FilterType::HighPass, FilterType::HighPassI)
        )
}

fn channel_filter_type_name(filter: FilterType) -> &'static str {
    match filter {
        FilterType::Notch => "Notch",
        FilterType::LowShelf
        | FilterType::LowShelfI
        | FilterType::LowShelfII
        | FilterType::LowShelfIII
        | FilterType::LowShelfIV => "Low Shelf · Classic",
        FilterType::LowShelfSoft => "Low Shelf · Smooth",
        FilterType::LowShelfSteep => "Low Shelf · Steep",
        FilterType::LowShelfResonant => "Low Shelf · Resonant",
        FilterType::HighShelf => "High Shelf · Classic",
        FilterType::HighShelfSoft => "High Shelf · Smooth",
        FilterType::HighShelfSteep => "High Shelf · Steep",
        FilterType::HighShelfResonant => "High Shelf · Resonant",
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
    channel_id: u64,
) -> bool {
    let mut changed = false;
    if header(
        ui,
        r,
        "Equalizers",
        !eq.bypassed && eq.all_bands().any(|b| b.enabled),
        state,
    ) {
        eq.bypassed = !eq.bypassed;
        changed = true;
    }
    let p = theme::palette(ui);
    let preset = at(r, 0.0, 24.0, r.width(), 16.0);
    ui.painter().rect_filled(preset, 0.0, p.surface_raised);
    let bands_height = 24.0;
    let bands_top = if curve {
        r.height() - bands_height
    } else {
        44.0
    };
    if curve {
        let show_eq_bands = !state.analyzer.tonal_view;
        let controls_height = state.analyzer.controls_height(r.width());
        let graph_top = 22.0 + controls_height + 4.0;
        changed |= state
            .analyzer
            .controls(ui, at(r, 0.0, 22.0, r.width(), controls_height), eq);
        changed |= graph(
            ui,
            at(
                r,
                0.0,
                graph_top,
                r.width(),
                (bands_top - graph_top - 4.0).max(80.0),
            ),
            eq,
            state,
            color,
            channel_id,
            show_eq_bands,
        );
    }
    let strip = at(r, 0.0, bands_top, r.width(), bands_height);
    ui.painter().rect_filled(strip, 0.0, p.canvas);
    let add_rect = Rect::from_min_size(
        Pos2::new((strip.right() - 22.0).max(strip.left()), strip.top() + 2.0),
        Vec2::new(20.0_f32.min(strip.width()), 20.0),
    );
    let strip_view =
        Rect::from_min_max(strip.min, Pos2::new(add_rect.left() - 2.0, strip.bottom()));
    let last_auto_scroll = ui.id().with("eq-band-last-auto-scroll");
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(strip_view)
            .id_salt("eq-band-strip-viewport"),
        |ui| {
            egui::ScrollArea::horizontal()
                .id_salt("eq-band-strip-scroll")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .scroll_source(egui::scroll_area::ScrollSource {
                    drag: egui::scroll_area::DragScroll::Always,
                    ..Default::default()
                })
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let visible_bands = eq
                            .all_bands()
                            .enumerate()
                            .filter_map(|(index, band)| band.enabled.then_some(index))
                            .collect::<Vec<_>>();
                        for i in visible_bands {
                            let band = *eq.band(i).expect("valid built-in EQ band index");
                            let (rect, response) =
                                ui.allocate_exact_size(Vec2::new(32.0, 20.0), Sense::click());
                            if response.clicked() {
                                state.selected_eq_band = i;
                            }
                            if response.secondary_clicked() && band.enabled {
                                remove_eq_band(eq, i);
                                if state.eq_solo == Some((channel_id, i)) {
                                    state.eq_solo = None;
                                }
                                state.selected_eq_band = eq
                                    .all_bands()
                                    .position(|candidate| candidate.enabled)
                                    .unwrap_or(0);
                                changed = true;
                            }
                            let selected = state.selected_eq_band == i;
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                if selected { p.surface_raised } else { p.canvas },
                            );
                            let text_color = if band.enabled && !eq.bypassed {
                                color
                            } else {
                                p.muted
                            };
                            let shape_tag = match band.filter {
                                FilterType::Bell
                                | FilterType::ParametricI
                                | FilterType::ParametricII => "B",
                                FilterType::Notch => "N",
                                FilterType::LowShelf
                                | FilterType::LowShelfI
                                | FilterType::LowShelfII
                                | FilterType::LowShelfIII
                                | FilterType::LowShelfIV
                                | FilterType::LowShelfSoft
                                | FilterType::LowShelfSteep
                                | FilterType::LowShelfResonant => "LS",
                                FilterType::HighShelf
                                | FilterType::HighShelfSoft
                                | FilterType::HighShelfSteep
                                | FilterType::HighShelfResonant => "HS",
                                FilterType::LowPass | FilterType::LowPassII => "LP",
                                FilterType::HighPass
                                | FilterType::HighPassI
                                | FilterType::HighPassII => "HP",
                            };
                            text(
                                ui,
                                rect,
                                &format!("{} {shape_tag}", i + 1),
                                9.0,
                                Align2::CENTER_CENTER,
                            );
                            ui.painter().circle_filled(
                                Pos2::new(rect.right() - 5.0, rect.top() + 5.0),
                                2.0,
                                text_color,
                            );
                            response.on_hover_text(format!(
                                "EQ band {}: {}",
                                i + 1,
                                channel_filter_type_name(band.filter)
                            ));
                            if selected {
                                let selection_changed = ui.data(|data| {
                                    data.get_temp::<usize>(last_auto_scroll) != Some(i)
                                });
                                if selection_changed {
                                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                                    ui.data_mut(|data| data.insert_temp(last_auto_scroll, i));
                                }
                            }
                        }
                    });
                });
        },
    );
    let add_response = ui.interact(add_rect, ui.id().with("eq-band-add"), Sense::click());
    ui.painter().rect_filled(add_rect, 2.0, p.surface_raised);
    text(ui, add_rect, "+", 14.0, Align2::CENTER_CENTER);
    add_response.clone().on_hover_text(format!(
        "Add EQ band (up to {})",
        opentrack_core::project::MAX_EQ_BANDS
    ));
    if add_response.clicked() {
        let next_unused_band = next_eq_band(eq);
        if let Some(index) = next_unused_band {
            add_eq_band(eq, index, eq_band_default_frequency(index), 0.0);
            state.selected_eq_band = index;
            changed = true;
        }
    }
    changed
}

fn eq_knob(
    ui: &mut egui::Ui,
    rect: Rect,
    selected_index: usize,
    label: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    speed: f64,
    suffix: &str,
    logarithmic: bool,
    tooltip: String,
) -> bool {
    let compact = rect.height() < 58.0;
    let dense = rect.height() < 42.0;
    let label_height = if dense {
        7.0
    } else if compact {
        8.0
    } else {
        10.0
    };
    let dial_size = if dense {
        16.0
    } else if compact {
        18.0
    } else {
        24.0
    };
    let dial_center_y = if dense {
        15.0
    } else if compact {
        20.0
    } else {
        25.0
    };
    let value_top = if dense {
        24.0
    } else if compact {
        31.0
    } else {
        42.0
    };
    let value_height = if dense {
        14.0
    } else if compact {
        16.0
    } else {
        18.0
    };
    text(
        ui,
        at(rect, 0.0, 0.0, rect.width(), label_height),
        label,
        7.0,
        Align2::CENTER_CENTER,
    );
    let dial = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + dial_center_y),
        Vec2::splat(dial_size),
    );
    let mut changed = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(dial), |ui| {
        show_eq_dial(ui, value, minimum, maximum, logarithmic, &mut changed);
    });
    changed
        | number(
            ui,
            at(rect, 0.0, value_top, rect.width(), value_height),
            ("eq-knob-value", selected_index, label),
            value,
            minimum..=maximum,
            speed,
            suffix,
            tooltip,
        )
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

fn band_node_position(plot: Rect, band: &opentrack_core::project::EqualizerBand) -> Pos2 {
    node_position(
        plot,
        band.frequency_hz,
        if band.filter == FilterType::Notch {
            0.0
        } else {
            band.gain_db
        },
    )
}

fn solo_band_frequency_bounds(band: &opentrack_core::project::EqualizerBand) -> (f32, f32) {
    // The band-pass audition uses this same Q. Convert its approximate
    // half-power bandwidth from octaves to frequency bounds for the graph.
    let q = band.q.clamp(0.1, 10.0);
    let half_bandwidth_octaves = (2.0 * (1.0 / (2.0 * q)).asinh()) / std::f32::consts::LN_2;
    let half_ratio = 2.0_f32.powf(half_bandwidth_octaves * 0.5);
    (
        (band.frequency_hz / half_ratio).clamp(20.0, 20_000.0),
        (band.frequency_hz * half_ratio).clamp(20.0, 20_000.0),
    )
}

fn dynamic_overlay_frequency_bounds(
    band: &opentrack_core::project::EqualizerBand,
    range_db: f32,
    sample_rate: u32,
) -> (f32, f32) {
    const RESPONSE_FLOOR_DB: f32 = 0.08;
    const STEP_OCTAVES: f32 = 0.25;
    let mut altered = *band;
    altered.gain_db = (altered.gain_db + range_db).clamp(-24.0, 24.0);
    let response_size = |frequency| {
        let base = opentrack_audio::equalizer_band_response_db(band, frequency, sample_rate);
        let changed = opentrack_audio::equalizer_band_response_db(&altered, frequency, sample_rate);
        base.abs().max(changed.abs())
    };
    let find_edge = |direction: f32| {
        let mut frequency = band.frequency_hz.clamp(20.0, 20_000.0);
        for _ in 0..64 {
            let next = (frequency * 2.0_f32.powf(direction * STEP_OCTAVES)).clamp(20.0, 20_000.0);
            if next == frequency || response_size(next) <= RESPONSE_FLOOR_DB {
                return next;
            }
            frequency = next;
        }
        if direction < 0.0 { 20.0 } else { 20_000.0 }
    };
    (find_edge(-1.0), find_edge(1.0))
}

fn dynamic_overlay_sample_count(low: f32, high: f32) -> usize {
    (((high / low).log2() * 24.0).ceil() as usize).clamp(64, 256)
}

fn curve_band_mesh(upper: &[Pos2], lower: &[Pos2], color: Color32) -> egui::Mesh {
    let point_count = upper.len().min(lower.len());
    let mut mesh = egui::Mesh::default();
    for index in 0..point_count {
        mesh.colored_vertex(upper[index], color);
        mesh.colored_vertex(lower[index], color);
    }
    for index in 0..point_count.saturating_sub(1) {
        let current = (index * 2) as u32;
        let next = current + 2;
        mesh.add_triangle(current, current + 1, next);
        mesh.add_triangle(current + 1, next + 1, next);
    }
    mesh
}

fn sample_equalizer_curve(curve_db: &[f32], frequency_hz: f32) -> f32 {
    match curve_db {
        [] => 0.0,
        [only] => *only,
        _ => {
            let normalized = (frequency_hz.clamp(20.0, 20_000.0) / 20.0).log10() / 3.0;
            let position = normalized * (curve_db.len() - 1) as f32;
            let lower = (position.floor() as usize).min(curve_db.len() - 2);
            let fraction = position - lower as f32;
            curve_db[lower] + (curve_db[lower + 1] - curve_db[lower]) * fraction
        }
    }
}

fn dynamic_edge_is_shaped(points: &[Pos2], baseline_y: f32) -> bool {
    points
        .iter()
        .any(|point| (point.y - baseline_y).abs() > 1.0)
}

fn dynamic_overlay_response_db(
    aggregate_response_db: f32,
    static_band_response_db: f32,
    altered_band_response_db: f32,
) -> f32 {
    aggregate_response_db + altered_band_response_db - static_band_response_db
}

fn nearest_eq_node(plot: Rect, eq: &ChannelEqualizer, pointer: Pos2) -> Option<(usize, Pos2)> {
    eq.all_bands()
        .enumerate()
        .filter(|(_, band)| band.enabled)
        .map(|(index, band)| (index, band_node_position(plot, band)))
        .filter(|(_, node)| node.distance(pointer) <= 12.0)
        .min_by(|(_, a), (_, b)| a.distance_sq(pointer).total_cmp(&b.distance_sq(pointer)))
}

fn next_eq_band(eq: &ChannelEqualizer) -> Option<usize> {
    eq.all_bands().position(|band| !band.enabled)
}

fn eq_band_default_frequency(index: usize) -> f32 {
    const INITIAL_FREQUENCIES: [f32; 8] = [
        100.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 14_000.0,
    ];
    if let Some(frequency) = INITIAL_FREQUENCIES.get(index) {
        *frequency
    } else {
        let extra_index = index.saturating_sub(INITIAL_FREQUENCIES.len()) as f32;
        let extra_count = opentrack_core::project::MAX_EQ_BANDS - INITIAL_FREQUENCIES.len();
        100.0 * 180.0_f32.powf(extra_index / (extra_count - 1) as f32)
    }
}

fn add_eq_band(eq: &mut ChannelEqualizer, index: usize, frequency_hz: f32, gain_db: f32) -> bool {
    let Some(band) = eq.band_mut(index) else {
        return false;
    };
    *band = opentrack_core::project::EqualizerBand {
        enabled: true,
        filter: FilterType::Bell,
        frequency_hz: frequency_hz.clamp(20.0, 20_000.0),
        gain_db: gain_db.clamp(-24.0, 24.0),
        q: 0.707,
    };
    eq.dynamic_band_mut(index).unwrap().enabled = false;
    true
}

fn remove_eq_band(eq: &mut ChannelEqualizer, index: usize) -> bool {
    let Some(band) = eq.band_mut(index) else {
        return false;
    };
    if !band.enabled {
        return false;
    }
    band.enabled = false;
    eq.dynamic_band_mut(index).unwrap().enabled = false;
    true
}

fn graph(
    ui: &mut egui::Ui,
    r: Rect,
    eq: &mut ChannelEqualizer,
    state: &mut ChannelSettingsUiState,
    color: Color32,
    channel_id: u64,
    show_eq_bands: bool,
) -> bool {
    let p = theme::palette(ui);
    let plot = r.shrink2(Vec2::new(30.0, 18.0));
    let response = ui.interact(r, ui.id().with("eq-graph"), Sense::click_and_drag());
    let hover_position = response.hover_pos();
    let drag_id = response.id.with("drag");
    let mut changed = false;
    let next_unused_band = next_eq_band(eq);
    if show_eq_bands
        && response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && nearest_eq_node(plot, eq, pos).is_none()
        && let Some(index) = next_unused_band
    {
        let frequencies = [
            100.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 14_000.0,
        ];
        let frequency =
            20.0 * 1000.0_f32.powf(((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0));
        let gain = ((plot.center().y - pos.y) / plot.height() * 48.0).clamp(-24.0, 24.0);
        if !add_eq_band(eq, index, frequency, gain) {
            add_eq_band(eq, index, frequencies[index], 0.0);
        }
        state.selected_eq_band = index;
        changed = true;
    }
    if show_eq_bands
        && response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && let Some((index, _)) = nearest_eq_node(plot, eq, pos)
    {
        remove_eq_band(eq, index);
        if state.eq_solo == Some((channel_id, index)) {
            state.eq_solo = None;
        }
        state.selected_eq_band = eq.all_bands().position(|band| band.enabled).unwrap_or(0);
        changed = true;
    }
    let pressed = show_eq_bands
        && response.is_pointer_button_down_on()
        && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    if show_eq_bands
        && (pressed
            || response.drag_started_by(egui::PointerButton::Primary)
            || response.clicked_by(egui::PointerButton::Primary))
    {
        if let Some(pos) = ui
            .input(|i| i.pointer.press_origin())
            .or(response.interact_pointer_pos())
        {
            if let Some((index, node)) = nearest_eq_node(plot, eq, pos) {
                ui.data_mut(|d| {
                    d.insert_temp(
                        drag_id,
                        GraphDrag {
                            band: index,
                            offset: node - pos,
                        },
                    )
                });
                state.selected_eq_band = index;
            } else {
                ui.data_mut(|d| d.remove::<GraphDrag>(drag_id));
            }
        }
    }
    if show_eq_bands
        && (pressed
            || response.dragged_by(egui::PointerButton::Primary)
            || response.clicked_by(egui::PointerButton::Primary))
    {
        if let (Some(pos), Some(drag)) = (
            response.interact_pointer_pos(),
            ui.data(|d| d.get_temp::<GraphDrag>(drag_id)),
        ) {
            let pos = pos + drag.offset;
            let band = eq.band_mut(drag.band).unwrap();
            let frequency =
                20.0 * 1000.0_f32.powf(((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0));
            let gain = ((plot.center().y - pos.y) / plot.height() * 48.0).clamp(-24.0, 24.0);
            changed |= (band.frequency_hz - frequency).abs() > 0.001
                || (band.filter != FilterType::Notch && (band.gain_db - gain).abs() > 0.001);
            band.frequency_hz = frequency;
            if band.filter != FilterType::Notch {
                band.gain_db = gain;
            }
        }
    }
    if !show_eq_bands || !response.is_pointer_button_down_on() {
        ui.data_mut(|d| d.remove::<GraphDrag>(drag_id));
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
    // Keep the solo audition frequency window behind the live spectrum and EQ
    // curves. This makes the audible band visible without hiding the data.
    if show_eq_bands {
        if let Some((solo_channel, solo_index)) = state.eq_solo {
            if solo_channel == channel_id {
                if let Some(band) = eq.band(solo_index).filter(|band| band.enabled) {
                    let (low, high) = solo_band_frequency_bounds(band);
                    let left = node_position(plot, low, 0.0).x;
                    let right = node_position(plot, high, 0.0).x;
                    let range = Rect::from_min_max(
                        Pos2::new(left.min(right), plot.top()),
                        Pos2::new(left.max(right), plot.bottom()),
                    );
                    let solo_color = color.gamma_multiply(0.10);
                    ui.painter().rect_filled(range, 0.0, solo_color);
                    for x in [range.left(), range.right()] {
                        ui.painter().line_segment(
                            [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                            Stroke::new(1.0, color.gamma_multiply(0.58)),
                        );
                    }
                    text(
                        ui,
                        Rect::from_min_size(
                            range.min + Vec2::new(3.0, 3.0),
                            Vec2::new((range.width() - 6.0).max(30.0), 12.0),
                        ),
                        "SOLO RANGE",
                        7.0,
                        Align2::LEFT_TOP,
                    );
                }
            }
        }
    }
    state.analyzer.paint(ui, plot, color);
    let has_dynamic_bands = eq.all_bands().enumerate().any(|(index, band)| {
        band.enabled
            && band.filter.supports_dynamic_eq()
            && eq
                .dynamic_band(index)
                .is_some_and(|dynamic| dynamic.enabled)
    });
    let mut response_eq = eq.clone();
    response_eq.bypassed = false;
    let curve_response_eq: &ChannelEqualizer = if eq.bypassed && has_dynamic_bands {
        &response_eq
    } else {
        &*eq
    };
    let curve_gains: Vec<f32> = (0..=256)
        .map(|i| {
            let frequency = 20.0 * 1000.0_f32.powf(i as f32 / 256.0);
            opentrack_audio::channel_equalizer_response_db(
                curve_response_eq,
                frequency,
                state.analyzer.rate(None),
            )
        })
        .collect();
    let points = curve_gains
        .iter()
        .enumerate()
        .map(|(i, gain)| {
            let frequency = 20.0 * 1000.0_f32.powf(i as f32 / 256.0);
            node_position(
                plot,
                frequency,
                if eq.bypassed {
                    0.0
                } else {
                    gain.clamp(-24.0, 24.0)
                },
            )
        })
        .collect();
    // Tonal mode owns the graph's frequency-balance trace. The aggregate EQ
    // response can contain narrow peaks/notches that look like analyzer spikes
    // and obscure that smoother tonal profile, so only draw it in EQ mode.
    if show_eq_bands {
        ui.painter()
            .add(egui::Shape::line(points, Stroke::new(1.5, color)));
    }
    let dynamic_gains = (show_eq_bands && !eq.bypassed)
        .then(|| state.analyzer.dynamic_gains_db(channel_id))
        .flatten();
    for (index, band) in eq.all_bands().enumerate() {
        let dynamic = *eq.dynamic_band(index).unwrap();
        if !show_eq_bands || !band.enabled || !dynamic.enabled || !band.filter.supports_dynamic_eq()
        {
            continue;
        }
        // Anchor the envelope to the combined static EQ curve so it joins the
        // response at both ends. Its width and dynamic delta still come only
        // from this band's own filter and Q.
        let sample_rate = state.analyzer.rate(None);
        let (low, high) = dynamic_overlay_frequency_bounds(band, dynamic.range_db, sample_rate);
        let sample_count = dynamic_overlay_sample_count(low, high);
        let mut static_points = Vec::with_capacity(sample_count + 1);
        let mut dynamic_points = Vec::with_capacity(sample_count + 1);
        for sample in 0..=sample_count {
            let t = sample as f32 / sample_count as f32;
            let frequency = low * (high / low).powf(t);
            let static_band =
                opentrack_audio::equalizer_band_response_db(band, frequency, sample_rate);
            let mut dynamic_band = *band;
            dynamic_band.gain_db = (dynamic_band.gain_db + dynamic.range_db).clamp(-24.0, 24.0);
            let dynamic_response =
                opentrack_audio::equalizer_band_response_db(&dynamic_band, frequency, sample_rate);
            let aggregate_response = if eq.bypassed {
                0.0
            } else {
                sample_equalizer_curve(&curve_gains, frequency)
            };
            static_points.push(node_position(
                plot,
                frequency,
                aggregate_response.clamp(-24.0, 24.0),
            ));
            dynamic_points.push(node_position(
                plot,
                frequency,
                dynamic_overlay_response_db(aggregate_response, static_band, dynamic_response)
                    .clamp(-24.0, 24.0),
            ));
        }
        ui.painter().add(egui::Shape::mesh(curve_band_mesh(
            &static_points,
            &dynamic_points,
            color.gamma_multiply(0.19),
        )));
        // Keep the combined static EQ curve visible over the fill. If it is
        // flat, leave the full-width graph baseline as the only line.
        if dynamic_edge_is_shaped(&static_points, plot.center().y) {
            ui.painter().add(egui::Shape::line(
                static_points,
                Stroke::new(1.5, color.gamma_multiply(0.9)),
            ));
        }
        ui.painter().add(egui::Shape::line(
            dynamic_points,
            Stroke::new(1.5, color.gamma_multiply(0.82)),
        ));
    }
    if response.hovered() {
        ui.painter().rect_stroke(
            r.shrink(0.5),
            1.0,
            ui.visuals().widgets.hovered.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    for (i, band) in eq.all_bands().enumerate() {
        if !show_eq_bands || !band.enabled {
            continue;
        }
        let pos = band_node_position(plot, band);
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
    if let Some(dynamic_gains) = dynamic_gains {
        for (index, band) in eq.all_bands().enumerate() {
            let dynamic = *eq.dynamic_band(index).unwrap();
            if !show_eq_bands
                || !band.enabled
                || !dynamic.enabled
                || !band.filter.supports_dynamic_eq()
            {
                continue;
            }
            let mut actual_band = *band;
            actual_band.gain_db = (actual_band.gain_db + dynamic_gains[index]).clamp(-24.0, 24.0);
            let sample_rate = state.analyzer.rate(None);
            let (low, high) = dynamic_overlay_frequency_bounds(band, dynamic.range_db, sample_rate);
            let sample_count = dynamic_overlay_sample_count(low, high);
            let mut static_points = Vec::with_capacity(sample_count + 1);
            let mut actual_points = Vec::with_capacity(sample_count + 1);
            for sample in 0..=sample_count {
                let t = sample as f32 / sample_count as f32;
                let frequency = low * (high / low).powf(t);
                let static_band =
                    opentrack_audio::equalizer_band_response_db(band, frequency, sample_rate);
                let actual_band_response = opentrack_audio::equalizer_band_response_db(
                    &actual_band,
                    frequency,
                    sample_rate,
                );
                let aggregate_response = sample_equalizer_curve(&curve_gains, frequency);
                static_points.push(node_position(
                    plot,
                    frequency,
                    aggregate_response.clamp(-24.0, 24.0),
                ));
                actual_points.push(node_position(
                    plot,
                    frequency,
                    dynamic_overlay_response_db(
                        aggregate_response,
                        static_band,
                        actual_band_response,
                    )
                    .clamp(-24.0, 24.0),
                ));
            }
            ui.painter().add(egui::Shape::mesh(curve_band_mesh(
                &static_points,
                &actual_points,
                color.gamma_multiply(0.42),
            )));
            // The movement fill is painted over the graph's static curve. Draw
            // the same combined response again so the envelope stays visibly
            // connected at its upper edge, including when a neighbor shapes it.
            if dynamic_edge_is_shaped(&static_points, plot.center().y) {
                ui.painter().add(egui::Shape::line(
                    static_points,
                    Stroke::new(1.5, color.gamma_multiply(0.9)),
                ));
            }
            ui.painter().add(egui::Shape::line(
                actual_points,
                Stroke::new(1.5, color.gamma_multiply(0.95)),
            ));
            let actual_gain = dynamic_overlay_response_db(
                sample_equalizer_curve(&curve_gains, band.frequency_hz),
                opentrack_audio::equalizer_band_response_db(band, band.frequency_hz, sample_rate),
                opentrack_audio::equalizer_band_response_db(
                    &actual_band,
                    band.frequency_hz,
                    sample_rate,
                ),
            );
            let actual_node = node_position(plot, band.frequency_hz, actual_gain);
            ui.painter().circle_filled(actual_node, 4.2, p.text);
            ui.painter()
                .circle_stroke(actual_node, 4.2, Stroke::new(1.5, color));
        }
    }
    let selected_index = state
        .selected_eq_band
        .min(opentrack_core::project::MAX_EQ_BANDS - 1);
    let selected_node = eq
        .band(selected_index)
        .filter(|_| show_eq_bands)
        .map(|band| band_node_position(plot, band));
    if let Some(node) =
        selected_node.filter(|_| eq.band(selected_index).is_some_and(|band| band.enabled))
    {
        ui.painter()
            .circle_filled(node, 9.0, color.gamma_multiply(0.24));
        ui.painter()
            .circle_stroke(node, 8.0, Stroke::new(1.25, color));
        ui.painter().circle_filled(node, 5.1, p.text);
        ui.painter()
            .circle_stroke(node, 5.1, Stroke::new(2.0, color));
    }
    if let (Some(node), Some((band, dynamic))) = (
        selected_node.filter(|_| eq.band(selected_index).is_some_and(|band| band.enabled)),
        eq.band_and_dynamic_mut(selected_index),
    ) {
        let popup_size = Vec2::new(344.0_f32.min(plot.width()), 144.0_f32.min(r.height()));
        let mut popup_pos = Pos2::new(
            plot.left() + 8.0 + state.eq_detail_offset.x,
            plot.top() + 6.0 + state.eq_detail_offset.y,
        );
        popup_pos.x = popup_pos
            .x
            .clamp(plot.left(), (plot.right() - popup_size.x).max(plot.left()));
        popup_pos.y = popup_pos
            .y
            .clamp(r.top(), (r.bottom() - popup_size.y).max(r.top()));
        let popup = Rect::from_min_size(popup_pos, popup_size);
        ui.painter().rect_filled(popup, 4.0, p.surface_raised);
        ui.painter().rect_stroke(
            popup,
            4.0,
            Stroke::new(1.0, p.grid),
            egui::StrokeKind::Inside,
        );
        let inner = popup.shrink2(Vec2::new(7.0, 4.0));
        let header = at(inner, 0.0, 0.0, inner.width(), 16.0);
        ui.painter().rect_filled(header, 3.0, p.canvas);
        let title_rect = at(inner, 24.0, 0.0, (inner.width() - 78.0).max(40.0), 16.0);
        let title_response = ui.interact(
            title_rect,
            ui.id().with(("eq-popup-title", selected_index)),
            Sense::drag(),
        );
        if title_response.dragged() {
            state.eq_detail_offset += title_response.drag_delta();
        }
        text(
            ui,
            title_rect,
            &format!("Band {}  ·  drag to move", selected_index + 1),
            8.0,
            Align2::LEFT_CENTER,
        );
        let enabled_rect = at(inner, 0.0, 0.0, 20.0, 16.0);
        if power(
            ui,
            enabled_rect,
            ("eq-popup-power", selected_index),
            band.enabled,
        )
        .clicked()
        {
            band.enabled = !band.enabled;
            changed = true;
        }
        let solo_supported = channel_id != opentrack_core::project::INPUT_BUS_ID;
        let solo_active = state.eq_solo == Some((channel_id, selected_index));
        let solo_rect = at(inner, inner.width() - 20.0, 0.0, 20.0, 16.0);
        let solo_response = ui.interact(
            solo_rect,
            ui.id().with(("eq-popup-solo", selected_index)),
            if solo_supported {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let palette = theme::palette(ui);
        ui.painter().rect_filled(
            solo_rect,
            2.0,
            if solo_active {
                palette.accent.gamma_multiply(0.3)
            } else {
                palette.canvas
            },
        );
        text(ui, solo_rect, "S", 8.0, Align2::CENTER_CENTER);
        if solo_response.clicked() {
            state.eq_solo = if solo_active {
                None
            } else {
                Some((channel_id, selected_index))
            };
        }
        solo_response.on_hover_text(if solo_supported {
            "Solo this band as a frequency-isolated audition"
        } else {
            "Band audition is unavailable for the input bus"
        });
        let section_title = at(inner, 0.0, 15.0, inner.width(), 7.0);
        text(ui, section_title, "STATIC EQ", 7.0, Align2::LEFT_CENTER);
        let static_width = ((inner.width() - 8.0) / 3.0).max(1.0);
        let static_top = 20.0;
        if band.filter == FilterType::Notch {
            let gain_rect = at(inner, 0.0, static_top, static_width, 38.0);
            text(
                ui,
                at(gain_rect, 0.0, 0.0, gain_rect.width(), 8.0),
                "GAIN",
                7.0,
                Align2::CENTER_CENTER,
            );
            text(
                ui,
                at(gain_rect, 0.0, 24.0, gain_rect.width(), 14.0),
                "N/A",
                8.0,
                Align2::CENTER_CENTER,
            );
        } else {
            changed |= eq_knob(
                ui,
                at(inner, 0.0, static_top, static_width, 38.0),
                selected_index,
                "GAIN",
                &mut band.gain_db,
                -24.0,
                24.0,
                0.1,
                " dB",
                false,
                "EQ band gain".to_owned(),
            );
        }
        changed |= eq_knob(
            ui,
            at(inner, static_width + 4.0, static_top, static_width, 38.0),
            selected_index,
            "FREQ",
            &mut band.frequency_hz,
            20.0,
            20000.0,
            1.0,
            " Hz",
            true,
            "EQ band center or cutoff frequency".to_owned(),
        );
        changed |= eq_knob(
            ui,
            at(
                inner,
                (static_width + 4.0) * 2.0,
                static_top,
                static_width,
                38.0,
            ),
            selected_index,
            "Q",
            &mut band.q,
            0.1,
            10.0,
            0.01,
            "",
            true,
            if matches!(
                band.filter,
                FilterType::LowShelf
                    | FilterType::LowShelfI
                    | FilterType::LowShelfII
                    | FilterType::LowShelfIII
                    | FilterType::LowShelfIV
                    | FilterType::HighShelf
                    | FilterType::LowShelfSoft
                    | FilterType::LowShelfSteep
                    | FilterType::LowShelfResonant
                    | FilterType::HighShelfSoft
                    | FilterType::HighShelfSteep
                    | FilterType::HighShelfResonant
            ) {
                "Shelf transition width / resonance".to_owned()
            } else {
                "EQ band Q / resonance".to_owned()
            },
        );
        let filter_label = at(inner, 0.0, 58.0, 40.0, 18.0);
        text(ui, filter_label, "FILTER", 7.0, Align2::LEFT_CENTER);
        let filter_rect = at(inner, 42.0, 58.0, (inner.width() - 42.0).max(1.0), 18.0);
        let filter_response = ui.interact(
            filter_rect,
            ui.id().with(("eq-popup-filter", selected_index)),
            Sense::click(),
        );
        ui.painter().rect_filled(
            filter_rect,
            2.0,
            if filter_response.hovered() {
                p.surface_raised
            } else {
                p.canvas
            },
        );
        ui.painter().rect_stroke(
            filter_rect,
            2.0,
            Stroke::new(1.0, p.grid),
            egui::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                Pos2::new(filter_rect.left() + 1.0, filter_rect.top() + 3.0),
                Pos2::new(filter_rect.left() + 1.0, filter_rect.bottom() - 3.0),
            ],
            Stroke::new(2.0, color),
        );
        text(
            ui,
            filter_rect.shrink2(Vec2::new(8.0, 0.0)),
            channel_filter_type_name(band.filter),
            10.0,
            Align2::LEFT_CENTER,
        );
        crate::icon_system::paint_app_icon(
            ui,
            Rect::from_center_size(
                Pos2::new(filter_rect.right() - 10.0, filter_rect.center().y),
                Vec2::splat(10.0),
            ),
            crate::icon_system::IconId::ChevronDown,
            p.text,
        );
        let filter_response = filter_response.on_hover_text("Choose this band's filter type");
        menu(&filter_response, 218.0, |ui| {
            for (kind, label) in eq_filter_types(selected_index).iter().copied() {
                if channel_menu_item(ui, eq_filter_selected(band.filter, kind), label).clicked() {
                    band.filter = kind;
                    changed = true;
                    ui.close();
                }
            }
        });
        let dynamic_eligible = band.filter.supports_dynamic_eq();
        let divider_y = 78.0;
        ui.painter().line_segment(
            [
                Pos2::new(inner.left(), inner.top() + divider_y),
                Pos2::new(inner.right(), inner.top() + divider_y),
            ],
            Stroke::new(1.0, p.grid),
        );
        let dynamic_title = at(inner, 24.0, 80.0, inner.width() - 48.0, 14.0);
        text(ui, dynamic_title, "DYNAMIC EQ", 8.0, Align2::LEFT_CENTER);
        let dynamic_power_rect = at(inner, 0.0, 79.0, 20.0, 16.0);
        if dynamic_eligible {
            if power(
                ui,
                dynamic_power_rect,
                ("eq-popup-dynamic-power", selected_index),
                dynamic.enabled,
            )
            .clicked()
            {
                dynamic.enabled = !dynamic.enabled;
                changed = true;
            }
            let dynamic_width = ((inner.width() - 6.0) / 4.0).max(1.0);
            let dynamic_top = 93.0;
            for (slot, (label, value, minimum, maximum, speed, suffix, log, help)) in [
                (
                    "THRESH",
                    &mut dynamic.threshold_db,
                    -90.0,
                    0.0,
                    0.5,
                    " dB",
                    false,
                    "Dynamic detector threshold",
                ),
                (
                    "RANGE",
                    &mut dynamic.range_db,
                    -18.0,
                    18.0,
                    0.5,
                    " dB",
                    false,
                    "Maximum dynamic gain change",
                ),
                (
                    "ATTACK",
                    &mut dynamic.attack_ms,
                    0.1,
                    2000.0,
                    0.1,
                    " ms",
                    true,
                    "Dynamic attack time",
                ),
                (
                    "RELEASE",
                    &mut dynamic.release_ms,
                    1.0,
                    5000.0,
                    1.0,
                    " ms",
                    true,
                    "Dynamic release time",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let x = slot as f32 * (dynamic_width + 2.0);
                changed |= eq_knob(
                    ui,
                    at(inner, x, dynamic_top, dynamic_width, 38.0),
                    selected_index,
                    label,
                    value,
                    minimum,
                    maximum,
                    speed,
                    suffix,
                    log,
                    help.to_owned(),
                );
            }
        } else {
            text(
                ui,
                dynamic_title,
                "DYNAMIC EQ  ·  unavailable for this shape",
                8.0,
                Align2::LEFT_CENTER,
            );
        }
    }
    changed
}

fn strip(
    ui: &mut egui::Ui,
    r: Rect,
    track: &mut Track,
    names: &[String],
    group_options: &[(u64, String)],
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
        let group_route_label = if input || is_bus {
            None
        } else {
            label
                .strip_prefix("Group:")
                .and_then(|value| value.parse::<u64>().ok())
                .and_then(|group_id| {
                    group_options
                        .iter()
                        .find(|(candidate_id, _)| *candidate_id == group_id)
                        .map(|(_, name)| format!("Group: {name}"))
                })
        };
        let label = group_route_label.as_deref().unwrap_or(label);
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
                    if !group_options.is_empty() {
                        ui.separator();
                        for (group_id, group_name) in group_options {
                            if ui.button(format!("Group: {group_name}")).clicked() {
                                selected = Some(format!("Group:{group_id}"));
                                ui.close();
                            }
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
                track.group_output_auto = false;
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
        .on_hover_text("Input Gain (use shift); lights show live peak level");
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
    mixer::knob_graphic_with_signal(
        ui,
        knob,
        track.input_gain_db,
        Some((peak, project_controls::track_color(track))),
    );
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
                        && track.equalizer.all_bands().any(|band| band.enabled)
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

    fn render_equalizer_frame(
        context: &egui::Context,
        rect: Rect,
        time: f64,
        events: Vec<egui::Event>,
        eq_state: &mut ChannelEqualizer,
        state: &mut ChannelSettingsUiState,
    ) {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                equalizer(
                    ui,
                    rect,
                    eq_state,
                    true,
                    false,
                    state,
                    Color32::from_rgb(60, 170, 220),
                    1,
                );
            },
        );
        output.textures_delta.clear();
    }

    fn render_graph_frame(
        context: &egui::Context,
        rect: Rect,
        time: f64,
        events: Vec<egui::Event>,
        eq_state: &mut ChannelEqualizer,
        state: &mut ChannelSettingsUiState,
    ) -> bool {
        let mut changed = false;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                changed = graph(
                    ui,
                    rect,
                    eq_state,
                    state,
                    Color32::from_rgb(60, 170, 220),
                    1,
                    true,
                );
            },
        );
        output.textures_delta.clear();
        changed
    }

    fn pointer_button(position: Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: position,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn click_equalizer_point(
        context: &egui::Context,
        rect: Rect,
        equalizer: &mut ChannelEqualizer,
        state: &mut ChannelSettingsUiState,
        position: Pos2,
        button: egui::PointerButton,
        start_frame: usize,
    ) {
        let time = |frame: usize| frame as f64 * 0.05;
        render_equalizer_frame(
            context,
            rect,
            time(start_frame),
            vec![
                egui::Event::PointerMoved(position),
                pointer_button(position, button, true),
            ],
            equalizer,
            state,
        );
        render_equalizer_frame(
            context,
            rect,
            time(start_frame + 1),
            vec![pointer_button(position, button, false)],
            equalizer,
            state,
        );
    }

    #[test]
    fn eq_graph_hit_testing_ignores_blank_space_and_anchors_notches_at_zero_gain() {
        let plot = Rect::from_min_max(Pos2::new(30.0, 20.0), Pos2::new(630.0, 220.0));
        let mut equalizer = ChannelEqualizer::default();
        equalizer.bands[0].filter = FilterType::Notch;
        equalizer.bands[0].gain_db = 18.0;
        let notch_node = band_node_position(plot, &equalizer.bands[0]);
        assert_eq!(notch_node.y, plot.center().y);
        assert!(nearest_eq_node(plot, &equalizer, notch_node).is_none());
        equalizer.bands[0].enabled = true;
        assert_eq!(nearest_eq_node(plot, &equalizer, notch_node).unwrap().0, 0);
        assert!(nearest_eq_node(plot, &equalizer, plot.right_top() - Vec2::splat(20.0)).is_none());
    }

    #[test]
    fn eq_band_add_remove_and_capacity_follow_the_strip_workflow() {
        let mut eq = ChannelEqualizer::default();
        assert_eq!(next_eq_band(&eq), Some(0));
        assert!(add_eq_band(&mut eq, 0, 2_400.0, 4.0));
        assert!(eq.band(0).unwrap().enabled);
        assert_eq!(eq.band(0).unwrap().frequency_hz, 2_400.0);
        assert_eq!(eq.band(0).unwrap().gain_db, 4.0);
        eq.dynamic_bands[0].enabled = true;
        assert!(remove_eq_band(&mut eq, 0));
        assert!(!eq.band(0).unwrap().enabled);
        assert!(!eq.dynamic_bands[0].enabled);
        for index in 0..opentrack_core::project::MAX_EQ_BANDS {
            assert!(add_eq_band(&mut eq, index, 100.0 * (index + 1) as f32, 0.0));
        }
        assert_eq!(next_eq_band(&eq), None);
        assert!(!add_eq_band(
            &mut eq,
            opentrack_core::project::MAX_EQ_BANDS,
            1_000.0,
            0.0
        ));
    }

    #[test]
    fn eq_strip_plus_adds_bands_and_right_click_removes_only_the_target() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
        let plus = Pos2::new(388.0, 288.0);
        let first_band = Pos2::new(20.0, 286.0);
        let mut equalizer = ChannelEqualizer::default();
        let mut state = ChannelSettingsUiState::default();

        render_equalizer_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            plus,
            egui::PointerButton::Primary,
            1,
        );
        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            plus,
            egui::PointerButton::Primary,
            3,
        );
        assert!(equalizer.band(0).unwrap().enabled);
        assert!(equalizer.band(1).unwrap().enabled);

        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            first_band,
            egui::PointerButton::Secondary,
            5,
        );
        assert!(!equalizer.band(0).unwrap().enabled);
        assert!(equalizer.band(1).unwrap().enabled);
    }

    #[test]
    fn dynamic_power_is_independent_from_band_power() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
        let plus = Pos2::new(388.0, 288.0);
        let mut equalizer = ChannelEqualizer::default();
        let mut state = ChannelSettingsUiState::default();

        render_equalizer_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            plus,
            egui::PointerButton::Primary,
            1,
        );
        assert!(equalizer.band(0).unwrap().enabled);
        assert!(!equalizer.dynamic_band(0).unwrap().enabled);

        let mut changed = false;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                time: Some(0.15),
                ..Default::default()
            },
            |ui| {
                changed = super::equalizer(
                    ui,
                    rect,
                    &mut equalizer,
                    true,
                    false,
                    &mut state,
                    Color32::from_rgb(60, 170, 220),
                    1,
                );
            },
        );
        let labels = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for label in [
            "STATIC EQ",
            "GAIN",
            "FREQ",
            "Q",
            "FILTER",
            "DYNAMIC EQ",
            "THRESH",
            "RANGE",
            "ATTACK",
            "RELEASE",
        ] {
            assert!(
                labels.iter().any(|rendered| rendered == label),
                "missing {label} knob label"
            );
        }
        output.textures_delta.clear();
        assert!(!changed);
        let filter_control = context
            .read_response(
                egui::Id::new((egui::ViewportId::ROOT, "__top_ui")).with(("eq-popup-filter", 0)),
            )
            .expect("filter type is an interactive control");
        assert!(filter_control.rect.height() >= 18.0);
        let dynamic_power = context
            .read_response(
                egui::Id::new((egui::ViewportId::ROOT, "__top_ui"))
                    .with(("eq-popup-dynamic-power", 0)),
            )
            .expect("dynamic EQ has a separate power button")
            .rect
            .center();

        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            dynamic_power,
            egui::PointerButton::Primary,
            3,
        );
        assert!(equalizer.dynamic_band(0).unwrap().enabled);
        assert!(equalizer.band(0).unwrap().enabled);

        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            dynamic_power,
            egui::PointerButton::Primary,
            5,
        );
        assert!(!equalizer.dynamic_band(0).unwrap().enabled);
        assert!(equalizer.band(0).unwrap().enabled);
    }

    #[test]
    fn filter_picker_exposes_distinct_pass_slopes_and_legacy_aliases() {
        assert!(
            EQ_FILTER_TYPES.iter().any(|(kind, label)| {
                *kind == FilterType::LowPass && label.contains("12 dB/oct")
            })
        );
        assert!(EQ_FILTER_TYPES.iter().any(|(kind, label)| {
            *kind == FilterType::LowPassII && label.contains("24 dB/oct")
        }));
        assert!(EQ_FILTER_TYPES.iter().any(|(kind, label)| {
            *kind == FilterType::HighPassI && label.contains("12 dB/oct")
        }));
        assert!(EQ_FILTER_TYPES.iter().any(|(kind, label)| {
            *kind == FilterType::HighPassII && label.contains("24 dB/oct")
        }));
        assert!(eq_filter_selected(
            FilterType::HighPass,
            FilterType::HighPassI
        ));
        assert!(!eq_filter_selected(
            FilterType::HighPassII,
            FilterType::HighPassI
        ));
        assert!(eq_filter_selected(
            FilterType::Bell,
            FilterType::ParametricI
        ));
        assert!(eq_filter_selected(
            FilterType::LowShelfIII,
            FilterType::LowShelf
        ));
    }

    #[test]
    fn tonal_mode_hides_band_editor_and_disables_graph_band_gestures() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
        let mut equalizer = ChannelEqualizer::default();
        equalizer.bands[0].enabled = true;
        equalizer.dynamic_bands[0].enabled = true;
        let mut state = ChannelSettingsUiState::default();
        state.analyzer.tonal_view = true;

        render_equalizer_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        let root = egui::Id::new((egui::ViewportId::ROOT, "__top_ui"));
        assert!(
            context
                .read_response(root.with(("eq-popup-power", 0)))
                .is_none()
        );
        assert!(
            context
                .read_response(root.with(("eq-popup-dynamic-power", 0)))
                .is_none()
        );

        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            Pos2::new(350.0, 120.0),
            egui::PointerButton::Primary,
            1,
        );
        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            Pos2::new(350.0, 120.0),
            egui::PointerButton::Primary,
            3,
        );
        assert!(equalizer.band(0).unwrap().enabled);
        assert_eq!(next_eq_band(&equalizer), Some(1));
    }

    #[test]
    fn dynamic_overlay_width_tracks_its_own_band_q() {
        let broad = opentrack_core::project::EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 6.0,
            q: 0.7,
        };
        let mut narrow = broad;
        narrow.q = 4.0;
        let broad_bounds = dynamic_overlay_frequency_bounds(&broad, -6.0, 48_000);
        let narrow_bounds = dynamic_overlay_frequency_bounds(&narrow, -6.0, 48_000);
        assert!(broad_bounds.1 / broad_bounds.0 > narrow_bounds.1 / narrow_bounds.0);
        for (band, (low, high)) in [(broad, broad_bounds), (narrow, narrow_bounds)] {
            let mut altered = band;
            altered.gain_db = (altered.gain_db - 6.0).clamp(-24.0, 24.0);
            for frequency in [low, high] {
                assert!(
                    opentrack_audio::equalizer_band_response_db(&band, frequency, 48_000).abs()
                        <= 0.08
                );
                assert!(
                    opentrack_audio::equalizer_band_response_db(&altered, frequency, 48_000).abs()
                        <= 0.08
                );
            }
        }
    }

    #[test]
    fn dynamic_envelope_uses_a_single_connected_ribbon_mesh() {
        let upper = [
            Pos2::new(0.0, 10.0),
            Pos2::new(5.0, 4.0),
            Pos2::new(10.0, 10.0),
        ];
        let lower = [
            Pos2::new(0.0, 12.0),
            Pos2::new(5.0, 18.0),
            Pos2::new(10.0, 12.0),
        ];
        let mesh = curve_band_mesh(&upper, &lower, Color32::WHITE);

        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices.len(), 12);
        assert_eq!(mesh.vertices[0].pos, upper[0]);
        assert_eq!(mesh.vertices[2].pos, upper[1]);
        assert_eq!(mesh.vertices[4].pos, upper[2]);
    }

    #[test]
    fn equalizer_curve_sampling_matches_its_log_frequency_polyline() {
        let curve = [0.0, 6.0, 12.0];

        assert_eq!(sample_equalizer_curve(&curve, 20.0), 0.0);
        assert_eq!(
            sample_equalizer_curve(&curve, 20.0 * 1000.0_f32.sqrt()),
            6.0
        );
        assert_eq!(sample_equalizer_curve(&curve, 20_000.0), 12.0);
        assert_eq!(sample_equalizer_curve(&[], 1_000.0), 0.0);
    }

    #[test]
    fn dynamic_flat_static_edge_does_not_draw_a_bounded_baseline() {
        let flat = [Pos2::new(0.0, 20.0), Pos2::new(5.0, 20.0)];
        let shaped = [Pos2::new(0.0, 20.0), Pos2::new(5.0, 8.0)];

        assert!(!dynamic_edge_is_shaped(&flat, 20.0));
        assert!(dynamic_edge_is_shaped(&shaped, 20.0));
    }

    #[test]
    fn dynamic_overlay_anchors_to_aggregate_but_keeps_its_own_band_delta() {
        let aggregate_db = -4.0;
        let static_band_db = 0.0;
        let altered_band_db = -6.0;

        let upper_edge = dynamic_overlay_response_db(aggregate_db, static_band_db, static_band_db);
        let lower_edge = dynamic_overlay_response_db(aggregate_db, static_band_db, altered_band_db);

        assert_eq!(upper_edge, aggregate_db);
        assert_eq!(lower_edge - upper_edge, altered_band_db - static_band_db);
    }

    #[test]
    fn empty_graph_double_click_adds_a_band_at_pointer_frequency_and_gain() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
        let position = Pos2::new(350.0, 150.0);
        let mut equalizer = ChannelEqualizer::default();
        let mut state = ChannelSettingsUiState::default();

        render_equalizer_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            position,
            egui::PointerButton::Primary,
            1,
        );
        click_equalizer_point(
            &context,
            rect,
            &mut equalizer,
            &mut state,
            position,
            egui::PointerButton::Primary,
            3,
        );
        assert!(equalizer.band(0).unwrap().enabled);
        assert!(equalizer.band(0).unwrap().frequency_hz > 5_000.0);
        assert!(equalizer.band(0).unwrap().gain_db > 0.0);
    }

    #[test]
    fn right_clicking_graph_node_removes_band_and_clears_its_dynamic_solo_state() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 300.0));
        let plot = rect.shrink2(Vec2::new(30.0, 18.0));
        let mut equalizer = ChannelEqualizer::default();
        equalizer.additional_bands[3] = opentrack_core::project::EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 18_000.0,
            gain_db: 0.0,
            q: 0.707,
        };
        equalizer.dynamic_bands[7].enabled = true;
        let node = band_node_position(plot, &equalizer.additional_bands[3]);
        let mut state = ChannelSettingsUiState::default();
        state.eq_solo = Some((1, 7));
        assert_eq!(nearest_eq_node(plot, &equalizer, node).unwrap().0, 7);

        render_graph_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        let time = |frame| frame as f64 * 0.05;
        render_graph_frame(
            &context,
            rect,
            time(1),
            vec![
                egui::Event::PointerMoved(node),
                pointer_button(node, egui::PointerButton::Secondary, true),
            ],
            &mut equalizer,
            &mut state,
        );
        let changed = render_graph_frame(
            &context,
            rect,
            time(2),
            vec![pointer_button(node, egui::PointerButton::Secondary, false)],
            &mut equalizer,
            &mut state,
        );
        assert!(changed, "graph-node right-click should report an EQ edit");
        assert!(!equalizer.additional_bands[3].enabled);
        assert!(!equalizer.dynamic_bands[7].enabled);
        assert_eq!(state.eq_solo, None);
    }

    #[test]
    fn eq_detail_panel_moves_from_its_title_without_following_the_selected_node() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 300.0));
        let mut equalizer = ChannelEqualizer::default();
        equalizer.bands[0].enabled = true;
        equalizer.bands[0].frequency_hz = 1_000.0;
        equalizer.bands[1].enabled = true;
        equalizer.bands[1].frequency_hz = 4_000.0;
        let mut state = ChannelSettingsUiState::default();
        render_graph_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        let start = Pos2::new(100.0, 32.0);
        let end = Pos2::new(140.0, 52.0);
        render_graph_frame(
            &context,
            rect,
            0.05,
            vec![
                egui::Event::PointerMoved(start),
                pointer_button(start, egui::PointerButton::Primary, true),
            ],
            &mut equalizer,
            &mut state,
        );
        render_graph_frame(
            &context,
            rect,
            0.10,
            vec![egui::Event::PointerMoved(end)],
            &mut equalizer,
            &mut state,
        );
        render_graph_frame(
            &context,
            rect,
            0.15,
            vec![pointer_button(end, egui::PointerButton::Primary, false)],
            &mut equalizer,
            &mut state,
        );
        assert!(state.eq_detail_offset.x > 30.0);
        assert!(state.eq_detail_offset.y > 10.0);

        let panel_offset = state.eq_detail_offset;
        state.selected_eq_band = 1;
        render_graph_frame(&context, rect, 0.20, Vec::new(), &mut equalizer, &mut state);
        assert_eq!(state.eq_detail_offset, panel_offset);
    }

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
