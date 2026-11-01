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
    let p = theme::palette(ui);
    let ink = if active {
        project_controls::label_ink(p.accent)
    } else {
        // Off controls stay visible: a muted grey reads as disabled without
        // disappearing into the button fill.
        p.muted
    };
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
    input_meters: &HashMap<u64, [f32; 2]>,
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
    // Device buses meter their own processed output; tracks report the peak of
    // the input stage (after the trim, before the EQ) so the input-gain ring
    // describes the level the trim actually sees.
    let input_peak = if index >= count {
        peak
    } else {
        input_meters
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
    let base_width = item_base_width;
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
                    input_peak,
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

/// Base column width for a Channel Settings view item.
fn item_base_width(item: ChannelSettingsViewItem) -> f32 {
    match item {
        ChannelSettingsViewItem::Inserts | ChannelSettingsViewItem::Sends => 146.0,
        ChannelSettingsViewItem::SendRouting => 154.0,
        ChannelSettingsViewItem::Fader | ChannelSettingsViewItem::ControlStrip => 92.0,
        _ => 474.0,
    }
}

/// Inline rack measurements. Each slot has a control row and a name row, so long
/// plug-in names have the full panel width.
const SLOT_HEIGHT: f32 = 46.0;
const SLOT_PITCH: f32 = 48.0;
const SLOT_FIRST_TOP: f32 = 24.0;
const SLOT_TOP_ROW: f32 = 22.0;
const SLOT_BUTTON: f32 = 13.0;
const SLOT_CHEVRON: f32 = 12.0;
/// Insert and send racks cap at this many rows.
const MAX_SLOTS: usize = 24;

/// Rows shown for one rack: at least one, never fewer than the used slots, and
/// never more than fit the panel.
fn visible_slots(configured: Option<usize>, used: usize, capacity: usize) -> usize {
    configured
        .unwrap_or(0)
        .max(used)
        .clamp(1, MAX_SLOTS)
        .min(capacity.max(1))
}

/// Clickable regions of one insert/send slot: a top control row holding the
/// number, power, bypass and eye, and a bottom row where the chevron and the
/// name form one button.
#[derive(Clone, Copy)]
struct SlotRects {
    index: Rect,
    power: Rect,
    bypass: Rect,
    view: Rect,
    chevron: Rect,
    label: Rect,
    /// One hit region for the chevron and the name: they are the same button.
    list: Rect,
}

impl SlotRects {
    fn new(cell: Rect, has_plugin: bool, has_eye: bool) -> Self {
        let top_height = SLOT_TOP_ROW.min(cell.height());
        let button_top = cell.top() + (top_height - SLOT_BUTTON) * 0.5;
        let index = Rect::from_min_size(
            Pos2::new(cell.left() + 2.0, cell.top()),
            Vec2::new(12.0, top_height),
        );
        let mut x = index.right() + 1.0;
        let mut button = |x: &mut f32| {
            let rect = Rect::from_min_size(Pos2::new(*x, button_top), Vec2::splat(SLOT_BUTTON));
            *x += SLOT_BUTTON + 1.0;
            rect
        };
        let (power, bypass, view) = if has_plugin {
            let power = button(&mut x);
            let bypass = button(&mut x);
            let view = if has_eye {
                button(&mut x)
            } else {
                Rect::NOTHING
            };
            (power, bypass, view)
        } else {
            (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING)
        };
        let name_top = cell.top() + top_height;
        let name_height = (cell.bottom() - name_top).max(1.0);
        let chevron = Rect::from_center_size(
            Pos2::new(cell.left() + 8.0, name_top + name_height * 0.5),
            Vec2::splat(SLOT_CHEVRON),
        );
        let label = Rect::from_min_max(
            Pos2::new(chevron.right() + 4.0, name_top),
            Pos2::new(
                (cell.right() - 3.0).max(chevron.right() + 4.0),
                cell.bottom(),
            ),
        );
        Self {
            index,
            power,
            bypass,
            view,
            chevron,
            label,
            list: Rect::from_min_max(
                Pos2::new(chevron.left(), label.top()),
                Pos2::new(label.right(), label.bottom()),
            ),
        }
    }
}

fn slot_cell(r: Rect, index: usize) -> Rect {
    at(
        r,
        0.0,
        SLOT_FIRST_TOP + index as f32 * SLOT_PITCH,
        r.width(),
        SLOT_HEIGHT,
    )
}

/// Row chrome: background and border. The slot number sits at the right edge.
fn slot_row(ui: &egui::Ui, cell: Rect) {
    let p = theme::palette(ui);
    ui.painter().rect_filled(cell, 1.0, p.canvas);
    ui.painter().rect_stroke(
        cell,
        1.0,
        Stroke::new(1.0, p.grid),
        egui::StrokeKind::Inside,
    );
}

fn slot_number(ui: &egui::Ui, rect: Rect, number: usize) {
    text(ui, rect, &number.to_string(), 8.0, Align2::CENTER_CENTER);
}

/// The "+" row under a rack: one click opens another empty slot.
fn paint_add_slot(ui: &egui::Ui, cell: Rect, label: &str) {
    let p = theme::palette(ui);
    let center = Pos2::new(cell.left() + 18.0, cell.center().y);
    let arm = 4.0;
    let stroke = Stroke::new(1.2, p.muted);
    ui.painter().line_segment(
        [center - Vec2::new(arm, 0.0), center + Vec2::new(arm, 0.0)],
        stroke,
    );
    ui.painter().line_segment(
        [center - Vec2::new(0.0, arm), center + Vec2::new(0.0, arm)],
        stroke,
    );
    text(
        ui,
        Rect::from_min_max(Pos2::new(center.x + arm + 6.0, cell.top()), cell.max),
        label,
        9.0,
        Align2::LEFT_CENTER,
    );
}

/// Hover feedback for the text and chevron that open a slot's list.
fn slot_hover(ui: &egui::Ui, rect: Rect, response: &egui::Response) {
    if response.hovered() {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(1.0, 2.0)),
            2.0,
            theme::hover_button_fill(theme::palette(ui).canvas, ui.visuals()),
        );
    }
}

/// Plug-in editor toggle. The eye is lit while that insert's editor is open.
fn view_button(
    ui: &mut egui::Ui,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    open: bool,
) -> egui::Response {
    let response = ui.interact(r, ui.id().with(id), Sense::click());
    let p = theme::palette(ui);
    let base = if open { p.accent } else { p.surface_raised };
    let ink = theme::paint_control(ui, r, &response, base, true, false);
    let ink = if open {
        project_controls::label_ink(base)
    } else {
        ink
    };
    paint_eye(ui, r.shrink(2.0), ink, open);
    response
}

/// Bypass keeps the plug-in loaded but removes it from the audio chain.
fn bypass_button(
    ui: &mut egui::Ui,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    active: bool,
) -> egui::Response {
    let response = ui.interact(r, ui.id().with(id), Sense::click());
    let p = theme::palette(ui);
    let base = if active { p.accent } else { p.surface_raised };
    let ink = theme::paint_control(ui, r, &response, base, true, false);
    let ink = if active {
        project_controls::label_ink(base)
    } else {
        ink
    };
    let center = r.center();
    let radius = r.width() * 0.30;
    ui.painter()
        .circle_stroke(center, radius, Stroke::new(1.0, ink));
    let arm = radius * 0.72;
    ui.painter().line_segment(
        [center - Vec2::splat(arm), center + Vec2::splat(arm)],
        Stroke::new(1.0, ink),
    );
    response
}

fn paint_eye(ui: &egui::Ui, r: Rect, ink: Color32, open: bool) {
    let center = r.center();
    let radius = Vec2::new(r.width() * 0.46, r.height() * 0.30);
    ui.painter().add(egui::Shape::ellipse_stroke(
        center,
        radius,
        Stroke::new(1.0, ink),
    ));
    ui.painter()
        .circle_filled(center, (r.height() * 0.13).max(1.0), ink);
    if !open {
        ui.painter().line_segment(
            [
                center + Vec2::new(-radius.x * 0.8, radius.y * 0.9),
                center + Vec2::new(radius.x * 0.8, -radius.y * 0.9),
            ],
            Stroke::new(1.0, ink),
        );
    }
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
    let capacity = ((r.height() - SLOT_FIRST_TOP) / SLOT_PITCH)
        .floor()
        .max(1.0) as usize;
    let rows = visible_slots(
        state.insert_rows.get(&track.id).copied(),
        track.inserts.len(),
        capacity,
    );
    for i in 0..rows {
        let cell = slot_cell(r, i);
        slot_row(ui, cell);
        let has_plugin = track.inserts.get(i).is_some();
        let enabled = track.inserts.get(i).is_some_and(|effect| effect.enabled);
        let bypassed = track.inserts.get(i).is_some_and(|effect| effect.bypassed);
        let is_legacy_compressor = track.inserts.get(i).is_some_and(|effect| {
            matches!(
                &effect.kind,
                InsertEffectKind::LegacyUnavailableCompressor { .. }
            )
        });
        let is_vst3 = track
            .inserts
            .get(i)
            .is_some_and(|effect| matches!(&effect.kind, InsertEffectKind::Vst3 { .. }));
        let rects = SlotRects::new(cell, has_plugin, is_vst3);
        slot_number(ui, rects.index, i + 1);
        let p = theme::palette(ui);
        if has_plugin {
            let power = power(
                ui,
                rects.power,
                ("insert-power", i),
                enabled && !is_legacy_compressor,
            )
            .on_hover_text(if is_legacy_compressor {
                "Removed built-in effect; replace it with a VST3 plug-in or remove the slot."
                    .to_owned()
            } else {
                format!(
                    "{} Insert {} (powered off plug-ins are unloaded)",
                    if enabled { "Turn off" } else { "Turn on" },
                    i + 1
                )
            });
            context(&power, state);
            if power.clicked() && !is_legacy_compressor {
                if let Some(effect) = track.inserts.get_mut(i) {
                    effect.enabled = !effect.enabled;
                    if !effect.enabled && is_vst3 {
                        state.vst3_power_off = Some((track.id, i));
                    }
                    changed = true;
                }
            }
            let bypass = bypass_button(ui, rects.bypass, ("insert-bypass", i), bypassed)
                .on_hover_text(if bypassed {
                    "Bypassed: removed from the chain, plug-in stays loaded"
                } else {
                    "Bypass: remove from the chain but keep the plug-in loaded"
                });
            if bypass.clicked() {
                if let Some(effect) = track.inserts.get_mut(i) {
                    effect.bypassed = !effect.bypassed;
                    changed = true;
                }
            }
            if is_vst3 {
                let editor_open = state.vst3_editor == Some((track.id, i));
                let view = view_button(ui, rects.view, ("insert-view", i), editor_open)
                    .on_hover_text(if !enabled {
                        "Power the insert on to open its editor"
                    } else if editor_open {
                        "Hide Plug-in Editor"
                    } else {
                        "Show Plug-in Editor"
                    });
                if view.clicked() && enabled {
                    if editor_open {
                        state.vst3_editor_close = Some((track.id, i));
                    } else {
                        state.vst3_parameter_slot = Some((track.id, i));
                    }
                }
            }
        }
        // The chevron and the name are one button: both always open the list.
        // The editor moved to its own button so the two actions are not confused.
        let list = ui
            .interact(rects.list, ui.id().with(("insert-list", i)), Sense::click())
            .on_hover_text(if has_plugin {
                "Choose or Replace Plug-in (right-click removes the row)"
            } else {
                "Add Plug-in"
            });
        slot_hover(ui, rects.list, &list);
        if list.secondary_clicked() {
            if has_plugin {
                if i + 1 == rows && rows > 1 {
                    state.insert_rows.insert(track.id, rows - 1);
                }
                state.remove_insert = Some((track.id, i));
            } else if rows > 1 {
                state.insert_rows.insert(track.id, rows - 1);
            }
        }
        crate::icon_system::paint_app_icon(
            ui,
            rects.chevron.shrink(1.0),
            crate::icon_system::IconId::ChevronRight,
            p.muted,
        );
        if let Some(effect) = track.inserts.get(i) {
            text(
                ui,
                rects.label,
                &insert_effect_name(&effect.kind),
                10.0,
                Align2::RIGHT_CENTER,
            );
        }
        let mut selected = None;
        menu(&list, 200.0, |ui| {
            show_insert_effect_picker(ui, &mut selected, vst3_plugin_catalog)
        });
        match selected {
            Some(InsertSelection::Add(kind)) => {
                let wants_editor = matches!(kind, InsertEffectKind::Vst3 { .. });
                if let Some(effect) = track.inserts.get_mut(i) {
                    effect.kind = kind;
                    if wants_editor {
                        state.vst3_parameter_slot = Some((track.id, i));
                        effect.enabled = true;
                    }
                } else {
                    if wants_editor {
                        state.vst3_parameter_slot = Some((track.id, i));
                    }
                    track.inserts.insert(
                        i.min(track.inserts.len()),
                        InsertEffect {
                            enabled: true,
                            bypassed: false,
                            kind,
                        },
                    );
                }
                changed = true;
            }
            Some(InsertSelection::Remove) => {
                if i < track.inserts.len() {
                    track.inserts.remove(i);
                    changed = true;
                }
            }
            None => {}
        }
    }
    if rows < MAX_SLOTS {
        let cell = slot_cell(r, rows);
        slot_row(ui, cell);
        let response = ui
            .interact(cell, ui.id().with("insert-add-slot"), Sense::click())
            .on_hover_text("Add Insert Slot");
        slot_hover(ui, cell, &response);
        paint_add_slot(ui, cell, "Add Insert");
        if response.clicked() {
            state.insert_rows.insert(track.id, rows + 1);
        }
    }
    changed
}

fn send_destination_menu(ui: &mut egui::Ui, selected: &mut Option<Option<String>>) {
    if ui.button("No Bus").clicked() {
        *selected = Some(None);
        ui.close();
    }
    ui.menu_button("Outputs", |ui| {
        for destination in ["Stereo Out", "Left", "Right"] {
            if ui.button(destination).clicked() {
                *selected = Some(Some(destination.to_owned()));
                ui.close();
            }
        }
    });
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
    let capacity = ((r.height() - SLOT_FIRST_TOP) / SLOT_PITCH)
        .floor()
        .max(1.0) as usize;
    let rows = visible_slots(
        state.send_rows.get(&track.id).copied(),
        track.send_routing.len(),
        capacity,
    );
    for i in 0..rows {
        let cell = slot_cell(r, i);
        slot_row(ui, cell);
        let has_target = track
            .send_routing
            .get(i)
            .is_some_and(|destination| destination.is_some());
        let rects = SlotRects::new(cell, has_target, false);
        slot_number(ui, rects.index, i + 1);
        let key = (track.id, i);
        let enabled = state.enabled_sends.contains(&key);
        if has_target {
            let power = power(ui, rects.power, ("send-power", i), enabled).on_hover_text(format!(
                "{} Send {}",
                if enabled { "Deactivate" } else { "Activate" },
                i + 1
            ));
            context(&power, state);
            if power.clicked() && !state.enabled_sends.remove(&key) {
                state.enabled_sends.insert(key);
            }
        }
        // Chevron and destination name are one button.
        let list = ui
            .interact(rects.list, ui.id().with(("send-list", i)), Sense::click())
            .on_hover_text("Choose Send Destination (right-click removes the row)");
        slot_hover(ui, rects.list, &list);
        if list.secondary_clicked() {
            if i < track.send_routing.len() {
                track.send_routing.remove(i);
                changed = true;
            }
            state.enabled_sends = state
                .enabled_sends
                .iter()
                .filter_map(|(track_id, slot)| {
                    if *track_id != track.id || *slot < i {
                        Some((*track_id, *slot))
                    } else if *slot > i {
                        Some((*track_id, *slot - 1))
                    } else {
                        None
                    }
                })
                .collect();
            if i + 1 == rows && rows > 1 {
                state.send_rows.insert(track.id, rows - 1);
            }
        }
        crate::icon_system::paint_app_icon(
            ui,
            rects.chevron.shrink(1.0),
            crate::icon_system::IconId::ChevronRight,
            theme::palette(ui).muted,
        );
        if let Some(destination) = track.send_routing.get(i).and_then(Option::as_deref) {
            text(ui, rects.label, destination, 10.0, Align2::RIGHT_CENTER);
        }
        let mut selected = None;
        menu(&list, 140.0, |ui| send_destination_menu(ui, &mut selected));
        if let Some(destination) = selected {
            track
                .send_routing
                .resize(track.send_routing.len().max(i + 1), None);
            track.send_routing[i] = destination;
            changed = true;
        }
    }
    if rows < MAX_SLOTS {
        let cell = slot_cell(r, rows);
        slot_row(ui, cell);
        let response = ui
            .interact(cell, ui.id().with("send-add-slot"), Sense::click())
            .on_hover_text("Add Send Slot");
        slot_hover(ui, cell, &response);
        paint_add_slot(ui, cell, "Add Send");
        if response.clicked() {
            state.send_rows.insert(track.id, rows + 1);
        }
    }
    changed
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

/// Bottom of the level window used by live-level knob rings (the threshold
/// ring). Matches the channel meters' 60 dB span so the ring reads like a meter
/// instead of saturating over the knob's much wider -90..0 dB value range.
const INPUT_LIGHT_FLOOR_DBFS: f32 = -60.0;

fn eq_knob(
    ui: &mut egui::Ui,
    rect: Rect,
    _selected_index: usize,
    label: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    speed: f64,
    suffix: &str,
    logarithmic: bool,
    tooltip: String,
    live_input_dbfs: Option<f32>,
    light_color: Color32,
) -> bool {
    let reset = (match label {
        "GAIN" => 0.0_f32,
        "FREQ" => 1000.0,
        "Q" => 0.707,
        "THRESH" => -24.0,
        "RANGE" => 0.0,
        "ATTACK" => 10.0,
        "RELEASE" => 100.0,
        _ => 0.0_f32,
    })
    .clamp(minimum, maximum);
    // Keep the parameter name below the dial; the editable number lives in
    // an interactive hover panel so the controls can nest diagonally.
    let label_rect = at(rect, 0.0, 48.0, 48.0, 11.0);
    text(ui, label_rect, label, 8.5, Align2::CENTER_CENTER);
    let tooltip = if let Some(db) = live_input_dbfs {
        format!(
            "{tooltip}\nLights: pre-EQ input RMS {db:.1} dBFS on a -60..0 dBFS ring; pointer: threshold"
        )
    } else {
        tooltip
    };
    let dial = eq_knob_dial_rect(rect);
    let mut changed = false;
    let (reset_requested, dial_response) = ui
        .scope_builder(egui::UiBuilder::new().max_rect(dial), |ui| {
            show_eq_dial(ui, value, minimum, maximum, logarithmic, &mut changed)
        })
        .inner;
    let mut value_popup = egui::Tooltip::for_enabled(&dial_response)
        .gap(0.0)
        .width(86.0);
    value_popup.popup = value_popup.popup.align(egui::RectAlign::RIGHT);
    value_popup.show(|ui| {
        ui.style_mut().explanation_tooltips = false;
        let response = ui
            .add_sized(
                Vec2::new(86.0, 20.0),
                egui::DragValue::new(value)
                    .range(minimum..=maximum)
                    .speed(speed)
                    .fixed_decimals(1)
                    .suffix(suffix),
            )
            .on_hover_text(format!(
                "{tooltip}\nDrag to adjust; double-click the value to type"
            ));
        if response.double_clicked() {
            response.request_focus();
        }
        changed |= response.changed();
    });
    if reset_requested && (*value - reset).abs() > f32::EPSILON {
        *value = reset;
        changed = true;
    }
    let normalized = if logarithmic {
        ((*value).ln() - minimum.ln()) / (maximum.ln() - minimum.ln())
    } else {
        (*value - minimum) / (maximum - minimum)
    }
    .clamp(0.0, 1.0);
    let knob_rect = Rect::from_center_size(dial.center(), Vec2::splat(48.0));
    let light_fraction = live_input_dbfs
        .map(|db| {
            // The threshold ring is a level meter, not a value indicator. The
            // knob's own -90..0 dB range is far wider than the useful span of
            // program material, so lighting it over that range left the ring
            // pinned near full. Use the channel meters' 60 dB window instead:
            // -60 dBFS is dark, 0 dBFS is full.
            ((db - INPUT_LIGHT_FLOOR_DBFS) / -INPUT_LIGHT_FLOOR_DBFS).clamp(0.0, 1.0)
        })
        .unwrap_or(normalized);
    mixer::knob_graphic_with_value(
        ui,
        knob_rect,
        normalized * 48.0 - 24.0,
        light_fraction,
        light_color,
    );
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
    let response_size = |frequency| {
        let base = opentrack_audio::equalizer_band_response_db(band, frequency, sample_rate);
        let changed = opentrack_audio::dynamic_equalizer_band_response_db(
            band,
            range_db,
            frequency,
            sample_rate,
        );
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

// Keep the graph and editor in distinct input regions, including narrow layouts.
fn eq_graph_rect(r: Rect, show_editor: bool) -> Rect {
    if !show_editor {
        return r;
    }
    // Keep enough graph width for direct band editing beside the compact
    // diagonal control cluster.
    let sidebar_width = 130.0_f32.min((r.width() - 320.0).max(0.0));
    Rect::from_min_max(
        r.min,
        Pos2::new(r.right() - sidebar_width - 8.0, r.bottom()),
    )
}

fn eq_knob_dial_rect(row: Rect) -> Rect {
    Rect::from_min_size(row.min, Vec2::splat(48.0))
}

fn eq_editor_knob_rect(inner: Rect, section: usize, slot: usize) -> Rect {
    let (x, y) = match section {
        0 => match slot {
            0 => (6.0, 70.0),
            1 => (62.0, 106.0),
            _ => (6.0, 142.0),
        },
        _ => match slot {
            0 => (62.0, 242.0),
            1 => (6.0, 278.0),
            2 => (62.0, 314.0),
            _ => (6.0, 350.0),
        },
    };
    at(inner, x, y, 48.0, 59.0)
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
    let graph_rect = eq_graph_rect(r, show_eq_bands);
    let sidebar = show_eq_bands
        .then(|| Rect::from_min_max(Pos2::new(graph_rect.right() + 8.0, r.top()), r.max));
    let plot = graph_rect.shrink2(Vec2::new(30.0, 18.0));
    let response = ui.interact(
        graph_rect,
        ui.id().with("eq-graph"),
        Sense::click_and_drag(),
    );
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
    ui.painter().rect_filled(graph_rect, 1.0, p.canvas);
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
            let dynamic_response = opentrack_audio::dynamic_equalizer_band_response_db(
                band,
                dynamic.range_db,
                frequency,
                sample_rate,
            );
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
            graph_rect.shrink(0.5),
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
                let actual_band_response = opentrack_audio::dynamic_equalizer_band_response_db(
                    band,
                    dynamic_gains[index],
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
                opentrack_audio::dynamic_equalizer_band_response_db(
                    band,
                    dynamic_gains[index],
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
    if show_eq_bands && eq.all_bands().all(|band| !band.enabled) {
        if let Some(sidebar) = sidebar {
            ui.painter().rect_filled(sidebar, 3.0, p.surface_raised);
            text(
                ui,
                sidebar.shrink(12.0),
                "ADD OR SELECT AN EQ BAND",
                10.0,
                Align2::CENTER_CENTER,
            );
        }
    }
    if let (Some(popup), Some((band, dynamic))) = (sidebar, eq.band_and_dynamic_mut(selected_index))
    {
        ui.painter().rect_filled(popup, 2.0, p.surface_raised);
        ui.painter().rect_stroke(
            popup,
            2.0,
            Stroke::new(1.0, p.grid),
            egui::StrokeKind::Inside,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(popup.shrink2(Vec2::new(7.0, 4.0)))
                .id_salt(("eq-editor", channel_id)),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("eq-editor-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (inner, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 414.0),
                            Sense::hover(),
                        );
                        let header = at(inner, 0.0, 0.0, inner.width(), 16.0);
                        ui.painter().rect_filled(header, 3.0, p.canvas);
                        let title_rect =
                            at(inner, 24.0, 0.0, (inner.width() - 78.0).max(40.0), 16.0);
                        text(
                            ui,
                            title_rect,
                            &format!("BAND {}", selected_index + 1),
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
                        let section_title = at(inner, 0.0, 55.0, inner.width(), 14.0);
                        text(ui, section_title, "STATIC EQ", 9.0, Align2::LEFT_CENTER);
                        let static_width = inner.width();
                        if band.filter == FilterType::Notch {
                            let gain_rect = eq_editor_knob_rect(inner, 0, 0);
                            let gain_label = at(gain_rect, 0.0, 48.0, 48.0, 11.0);
                            text(ui, gain_label, "GAIN", 8.0, Align2::CENTER_CENTER);
                            ui.interact(
                                gain_label,
                                ui.id().with(("eq-knob-not-available", selected_index)),
                                Sense::hover(),
                            )
                            .on_hover_ui(|ui| {
                                ui.label("Gain is unavailable for a notch filter");
                            });
                        } else {
                            changed |= eq_knob(
                                ui,
                                eq_editor_knob_rect(inner, 0, 0),
                                selected_index,
                                "GAIN",
                                &mut band.gain_db,
                                -24.0,
                                24.0,
                                0.1,
                                " dB",
                                false,
                                "EQ band gain".to_owned(),
                                None,
                                color,
                            );
                        }
                        changed |= eq_knob(
                            ui,
                            eq_editor_knob_rect(inner, 0, 1),
                            selected_index,
                            "FREQ",
                            &mut band.frequency_hz,
                            20.0,
                            20000.0,
                            1.0,
                            " Hz",
                            true,
                            "EQ band center or cutoff frequency".to_owned(),
                            None,
                            color,
                        );
                        changed |= eq_knob(
                            ui,
                            eq_editor_knob_rect(inner, 0, 2),
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
                            None,
                            color,
                        );
                        let filter_label = at(inner, 0.0, 18.0, static_width, 12.0);
                        text(ui, filter_label, "FILTER", 7.0, Align2::LEFT_CENTER);
                        let filter_rect = at(inner, 0.0, 31.0, static_width, 20.0);
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
                        let filter_text = Rect::from_min_max(
                            filter_rect.min + Vec2::new(8.0, 0.0),
                            filter_rect.max - Vec2::new(22.0, 0.0),
                        );
                        ui.painter().with_clip_rect(filter_text).text(
                            filter_text.left_center(),
                            Align2::LEFT_CENTER,
                            channel_filter_type_name(band.filter),
                            FontId::proportional(10.0),
                            p.text,
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
                        let filter_response = filter_response.on_hover_text(format!(
                            "{} — choose this band's filter type",
                            channel_filter_type_name(band.filter)
                        ));
                        menu(&filter_response, 132.0, |ui| {
                            for (kind, label) in eq_filter_types(selected_index).iter().copied() {
                                if channel_menu_item(
                                    ui,
                                    eq_filter_selected(band.filter, kind),
                                    label,
                                )
                                .clicked()
                                {
                                    band.filter = kind;
                                    changed = true;
                                    ui.close();
                                }
                            }
                        });
                        let dynamic_eligible = band.filter.supports_dynamic_eq();
                        // A smooth shallow step separates sections without competing with the dials.
                        let mut divider = vec![Pos2::new(inner.left(), inner.top() + 212.0)];
                        for step in 0..=12 {
                            let t = step as f32 / 12.0;
                            let eased = t * t * (3.0 - 2.0 * t);
                            divider.push(Pos2::new(
                                inner.left() + inner.width() * (0.34 + 0.22 * t),
                                inner.top() + 212.0 - 14.0 * eased,
                            ));
                        }
                        divider.push(Pos2::new(inner.right(), inner.top() + 198.0));
                        ui.painter().add(egui::Shape::line(
                            divider,
                            Stroke::new(1.0, p.text.gamma_multiply(0.55)),
                        ));
                        let dynamic_title = at(inner, 0.0, 220.0, inner.width(), 14.0);
                        text(ui, dynamic_title, "DYNAMIC EQ", 9.0, Align2::LEFT_CENTER);
                        // Use the open left column beside Threshold for the section switch.
                        let dynamic_power_rect = at(inner, 20.0, 258.0, 20.0, 16.0);
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
                            for (
                                slot,
                                (label, value, minimum, maximum, speed, suffix, log, help),
                            ) in [
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
                                changed |= eq_knob(
                                    ui,
                                    eq_editor_knob_rect(inner, 1, slot),
                                    selected_index,
                                    label,
                                    value,
                                    minimum,
                                    maximum,
                                    speed,
                                    suffix,
                                    log,
                                    help.to_owned(),
                                    (label == "THRESH")
                                        .then(|| state.analyzer.input_level_dbfs(channel_id))
                                        .flatten(),
                                    color,
                                );
                            }
                        } else {
                            text(
                                ui,
                                dynamic_title,
                                "DYNAMIC EQ unavailable",
                                8.0,
                                Align2::LEFT_CENTER,
                            );
                        }
                    });
            },
        );
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
    input_peak: [f32; 2],
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
    let (input_changed, input_responses) = mixer::input_gain_row(
        ui,
        r,
        79.0,
        track,
        input_peak,
        project_controls::track_color(track),
    );
    changed |= input_changed;
    for response in &input_responses {
        context(response, state);
    }
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

    fn render_inserts_frame(
        context: &egui::Context,
        rect: Rect,
        time: f64,
        events: Vec<egui::Event>,
        track: &mut Track,
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
                inserts(ui, rect, track, state, &[]);
            },
        );
        output.textures_delta.clear();
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
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 750.0));
        let plus = Pos2::new(888.0, 738.0);
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
        let dynamic_power = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "DYNAMIC EQ" => Some(Pos2::new(
                    text.pos.x + 30.0,
                    text.pos.y + text.galley.size().y * 0.5 + 39.0,
                )),
                _ => None,
            })
            .expect("dynamic EQ has a separate power button next to its heading");

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
    fn shelf_envelope_mesh_never_moves_the_opposite_q_flank_against_the_requested_range() {
        let color = Color32::from_rgb(155, 110, 190);
        let fill = color.gamma_multiply(0.19);
        for filter in [FilterType::LowShelf, FilterType::HighShelf] {
            for range in [-9.2_f32, 9.2] {
                let context = egui::Context::default();
                let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 550.0));
                let mut eq = ChannelEqualizer::default();
                *eq.band_mut(0).unwrap() = opentrack_core::project::EqualizerBand {
                    enabled: true,
                    filter,
                    gain_db: -3.0,
                    frequency_hz: 500.0,
                    q: 0.3,
                };
                *eq.band_mut(1).unwrap() = opentrack_core::project::EqualizerBand {
                    enabled: true,
                    filter: FilterType::Bell,
                    gain_db: 3.0,
                    frequency_hz: 1200.0,
                    q: 1.0,
                };
                eq.dynamic_band_mut(0).unwrap().enabled = true;
                eq.dynamic_band_mut(0).unwrap().range_db = range;
                let mut state = ChannelSettingsUiState::default();
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(rect),
                        ..Default::default()
                    },
                    |ui| {
                        graph(ui, rect, &mut eq, &mut state, color, 1, true);
                    },
                );
                output.textures_delta.clear();
                let mesh = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Mesh(mesh)
                            if mesh
                                .vertices
                                .first()
                                .is_some_and(|vertex| vertex.color == fill) =>
                        {
                            Some(mesh)
                        }
                        _ => None,
                    })
                    .expect("the dynamic shelf must paint its range envelope");
                assert!(mesh.vertices.len() > 100);
                for pair in mesh.vertices.chunks_exact(2) {
                    assert!(
                        (pair[1].pos.y - pair[0].pos.y) * range.signum() <= 0.01,
                        "{filter:?}: range envelope must not cross the aggregate reference into opposite movement"
                    );
                }
            }
        }
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
        let position = Pos2::new(eq_graph_rect(rect, true).right() - 35.0, 150.0);
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
        let plot = eq_graph_rect(rect, true).shrink2(Vec2::new(30.0, 18.0));
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
    fn threshold_lights_use_the_channel_meter_scale_and_ignore_the_threshold_value() {
        let color = Color32::from_rgb(181, 87, 143);
        for (input, threshold, expected) in [
            (-90.0, -24.0, 0),
            (-60.0, -24.0, 0),
            (-45.0, -24.0, 3),
            (-30.0, -24.0, 6),
            (0.0, -24.0, 11),
            (-45.0, -80.0, 3),
        ] {
            let ctx = egui::Context::default();
            let mut value = threshold;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0))),
                    ..Default::default()
                },
                |ui| {
                    eq_knob(
                        ui,
                        Rect::from_min_size(Pos2::ZERO, Vec2::new(154.0, 56.0)),
                        0,
                        "THRESH",
                        &mut value,
                        -90.0,
                        0.0,
                        0.5,
                        " dB",
                        false,
                        "Threshold".into(),
                        Some(input),
                        color,
                    );
                },
            );
            let lit = output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Circle(circle) if circle.fill == mixer::knob_led_color(color))).count();
            output.textures_delta.clear();
            assert_eq!(
                lit, expected,
                "input {input} dB must determine track-colored LED fill independently of the threshold value"
            );
        }
    }

    #[test]
    fn editor_diagonal_cluster_keeps_dials_separate_and_inside_the_content() {
        for width in [120.0, 136.0, 150.0] {
            let inner = Rect::from_min_size(Pos2::new(13.0, 17.0), Vec2::new(width, 414.0));
            let controls = [(0, 0), (0, 1), (0, 2)]
                .into_iter()
                .map(|(section, slot)| eq_editor_knob_rect(inner, section, slot))
                .chain((0..4).map(|slot| eq_editor_knob_rect(inner, 1, slot)))
                .collect::<Vec<_>>();
            let dials = controls
                .iter()
                .map(|control| eq_knob_dial_rect(*control))
                .collect::<Vec<_>>();
            for (i, dial) in dials.iter().enumerate() {
                assert!(inner.contains_rect(*dial), "dial {i} is outside the editor");
                assert_eq!(dial.width(), 48.0, "knob artwork keeps its intended size");
                for other in dials.iter().skip(i + 1) {
                    assert!(!dial.intersects(*other), "diagonal knobs overlap");
                }
            }
            let graph = eq_graph_rect(
                Rect::from_min_size(Pos2::ZERO, Vec2::new(width + 500.0, 400.0)),
                true,
            );
            // 130 px editor column plus its 8 px gutter.
            assert_eq!(graph.width(), width + 500.0 - 138.0);
        }
    }

    #[test]
    fn eq_knob_shows_its_label_without_an_always_visible_value() {
        fn render(
            context: &egui::Context,
            time: f64,
            events: Vec<egui::Event>,
            value: &mut f32,
        ) -> egui::FullOutput {
            let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 200.0));
            let knob = Rect::from_min_size(Pos2::new(30.0, 30.0), Vec2::new(100.0, 48.0));
            context.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    eq_knob(
                        ui,
                        knob,
                        0,
                        "GAIN",
                        value,
                        -24.0,
                        24.0,
                        0.1,
                        " dB",
                        false,
                        "EQ band gain".to_owned(),
                        None,
                        Color32::from_rgb(60, 170, 220),
                    );
                },
            )
        }

        let context = egui::Context::default();
        let mut value = 6.0;
        let mut output = render(&context, 0.0, Vec::new(), &mut value);
        let visible_text = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(visible_text(&output).iter().all(|text| text != "6.0 dB"));
        output.textures_delta.clear();

        let visible_text = visible_text(&output);
        assert!(visible_text.iter().any(|text| text == "GAIN"));
        assert!(visible_text.iter().all(|text| !text.contains("6.0")));
    }

    #[test]
    fn insert_and_send_slots_put_controls_on_top_and_the_name_below() {
        for has_eye in [false, true] {
            let cell = Rect::from_min_size(Pos2::new(12.0, 40.0), Vec2::new(146.0, SLOT_HEIGHT));
            let rects = SlotRects::new(cell, true, has_eye);
            let control_bottom = cell.top() + SLOT_TOP_ROW;
            assert!(
                rects.index.right() <= rects.power.left(),
                "number leads the control row"
            );
            assert!(rects.power.bottom() <= control_bottom);
            assert!(rects.power.right() <= rects.bypass.left());
            if has_eye {
                assert!(rects.bypass.right() <= rects.view.left());
            }
            // The name row starts below the controls and uses the full width.
            assert!(rects.label.top() >= control_bottom);
            assert!(rects.chevron.right() < rects.label.left());
            assert!(rects.label.right() <= cell.right());
            assert!(
                rects.label.width() >= 100.0,
                "the name row keeps the full panel width"
            );
            assert_eq!(
                rects.chevron.center().y,
                rects.label.center().y,
                "the chevron shares the name row"
            );
            assert!(
                rects.list.contains_rect(rects.chevron) && rects.list.contains_rect(rects.label),
                "the chevron and the name must be one hit region"
            );
            assert!(cell.contains_rect(rects.chevron));
        }
        let cell = Rect::from_min_size(Pos2::new(12.0, 40.0), Vec2::new(146.0, SLOT_HEIGHT));
        let empty = SlotRects::new(cell, false, false);
        assert_eq!(
            empty.power,
            Rect::NOTHING,
            "empty slots hide the power button"
        );
        assert_eq!(empty.bypass, Rect::NOTHING);
        assert_eq!(empty.view, Rect::NOTHING);
        assert!(
            SLOT_HEIGHT >= 45.0,
            "slots are about twice the original one-line height"
        );
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(146.0, 500.0));
        assert_eq!(
            slot_cell(area, 1).top() - slot_cell(area, 0).top(),
            SLOT_PITCH
        );
    }

    #[test]
    fn racks_start_with_one_row_and_grow_to_the_capacity() {
        assert_eq!(visible_slots(None, 0, 10), 1, "a rack starts with one slot");
        assert_eq!(visible_slots(None, 3, 10), 3, "used slots are always shown");
        assert_eq!(visible_slots(Some(5), 0, 10), 5);
        assert_eq!(visible_slots(Some(1), 7, 10), 7);
        assert_eq!(visible_slots(Some(99), 0, 10), 10);
        assert_eq!(visible_slots(None, 99, 10), 10);
        assert_eq!(visible_slots(Some(4), 0, 0), 1, "capacity shows one row");
    }

    #[test]
    fn a_vst3_insert_eye_opens_and_closes_the_editor() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(146.0, 500.0));
        let mut track = opentrack_core::project::BusChannel::output().strip;
        track.inserts = vec![InsertEffect {
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "0123456789abcdef0123456789abcdef".to_owned(),
                module_path: std::path::PathBuf::from("test.vst3"),
                name: "Test Plug-in".to_owned(),
                vendor: String::new(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        }];
        let mut state = ChannelSettingsUiState::default();
        let track_id = track.id;
        let eye = SlotRects::new(slot_cell(rect, 0), true, true).view.center();
        render_inserts_frame(&context, rect, -0.1, Vec::new(), &mut track, &mut state);
        render_inserts_frame(
            &context,
            rect,
            0.0,
            vec![
                egui::Event::PointerMoved(eye),
                pointer_button(eye, egui::PointerButton::Primary, true),
            ],
            &mut track,
            &mut state,
        );
        render_inserts_frame(
            &context,
            rect,
            0.1,
            vec![pointer_button(eye, egui::PointerButton::Primary, false)],
            &mut track,
            &mut state,
        );
        assert_eq!(
            state.vst3_parameter_slot,
            Some((track_id, 0)),
            "the eye must request the plug-in editor"
        );
        state.vst3_editor = Some((track_id, 0));
        render_inserts_frame(
            &context,
            rect,
            0.2,
            vec![
                egui::Event::PointerMoved(eye),
                pointer_button(eye, egui::PointerButton::Primary, true),
            ],
            &mut track,
            &mut state,
        );
        render_inserts_frame(
            &context,
            rect,
            0.3,
            vec![pointer_button(eye, egui::PointerButton::Primary, false)],
            &mut track,
            &mut state,
        );
        assert_eq!(
            state.vst3_editor_close,
            Some((track_id, 0)),
            "a lit eye must close the editor it opened"
        );
    }

    #[test]
    fn insert_row_power_bypass_and_right_click_actions() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(146.0, 500.0));
        let mut track = opentrack_core::project::BusChannel::output().strip;
        track.inserts = vec![InsertEffect {
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "0123456789abcdef0123456789abcdef".to_owned(),
                module_path: std::path::PathBuf::from("test.vst3"),
                name: "Test Plug-in".to_owned(),
                vendor: String::new(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        }];
        let mut state = ChannelSettingsUiState::default();
        let track_id = track.id;
        let rects = SlotRects::new(slot_cell(rect, 0), true, true);
        let click = |time: f64, point: Pos2, button: egui::PointerButton| {
            (
                time,
                vec![
                    egui::Event::PointerMoved(point),
                    pointer_button(point, button, true),
                ],
            )
        };
        render_inserts_frame(&context, rect, -0.1, Vec::new(), &mut track, &mut state);
        let (time, events) = click(0.0, rects.bypass.center(), egui::PointerButton::Primary);
        render_inserts_frame(&context, rect, time, events, &mut track, &mut state);
        render_inserts_frame(
            &context,
            rect,
            0.1,
            vec![pointer_button(
                rects.bypass.center(),
                egui::PointerButton::Primary,
                false,
            )],
            &mut track,
            &mut state,
        );
        assert!(
            track.inserts[0].bypassed,
            "bypass removes it from the chain"
        );
        assert!(track.inserts[0].enabled, "bypass keeps the plug-in loaded");

        let (time, events) = click(0.2, rects.power.center(), egui::PointerButton::Primary);
        render_inserts_frame(&context, rect, time, events, &mut track, &mut state);
        render_inserts_frame(
            &context,
            rect,
            0.3,
            vec![pointer_button(
                rects.power.center(),
                egui::PointerButton::Primary,
                false,
            )],
            &mut track,
            &mut state,
        );
        assert!(
            !track.inserts[0].enabled,
            "power off deactivates the insert"
        );
        assert_eq!(
            state.vst3_power_off,
            Some((track_id, 0)),
            "the app must unload the powered-off plug-in"
        );

        let (time, events) = click(0.4, rects.list.center(), egui::PointerButton::Secondary);
        render_inserts_frame(&context, rect, time, events, &mut track, &mut state);
        render_inserts_frame(
            &context,
            rect,
            0.5,
            vec![pointer_button(
                rects.list.center(),
                egui::PointerButton::Secondary,
                false,
            )],
            &mut track,
            &mut state,
        );
        assert_eq!(
            state.remove_insert,
            Some((track_id, 0)),
            "right-clicking the row removes the insert"
        );
    }

    #[test]
    fn condensed_panels_leave_more_room_for_the_graph() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 500.0));
        let graph = eq_graph_rect(area, true);
        assert_eq!(graph.right(), 1000.0 - 138.0);
        assert!(item_base_width(ChannelSettingsViewItem::Inserts) < 154.0);
        assert!(item_base_width(ChannelSettingsViewItem::Sends) < 154.0);
    }

    #[test]
    fn double_clicking_sidebar_gain_resets_it_without_creating_or_moving_a_graph_band() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 500.0));
        let mut equalizer = ChannelEqualizer::default();
        add_eq_band(&mut equalizer, 0, 1000.0, 6.0);
        let mut state = ChannelSettingsUiState::default();
        let editor = Rect::from_min_max(
            Pos2::new(eq_graph_rect(rect, true).right() + 8.0, rect.top()),
            rect.max,
        )
        .shrink2(Vec2::new(7.0, 4.0));
        let gain = eq_editor_knob_rect(editor, 0, 0);
        let point = eq_knob_dial_rect(gain).center();
        render_graph_frame(&context, rect, 0.0, Vec::new(), &mut equalizer, &mut state);
        for click in 0..2 {
            let time = 0.05 + click as f64 * 0.10;
            render_graph_frame(
                &context,
                rect,
                time,
                vec![
                    egui::Event::PointerMoved(point),
                    pointer_button(point, egui::PointerButton::Primary, true),
                ],
                &mut equalizer,
                &mut state,
            );
            render_graph_frame(
                &context,
                rect,
                time + 0.05,
                vec![pointer_button(point, egui::PointerButton::Primary, false)],
                &mut equalizer,
                &mut state,
            );
        }
        assert_eq!(equalizer.band(0).unwrap().gain_db, 0.0);
        assert_eq!(equalizer.band(0).unwrap().frequency_hz, 1000.0);
        assert_eq!(equalizer.all_bands().filter(|band| band.enabled).count(), 1);
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
                            &HashMap::new(),
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
                    &HashMap::new(),
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
