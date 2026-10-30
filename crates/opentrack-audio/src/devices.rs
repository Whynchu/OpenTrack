//! Enumerates audio devices and the stream configurations exposed by CPAL.

use std::str::FromStr;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, DeviceId, SampleFormat, SupportedBufferSize, SupportedStreamConfigRange};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub host_name: String,
    pub supports_input: bool,
    pub supports_output: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioOutputConfig {
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
    pub default_sample_rate: u32,
    pub default_channels: u16,
    pub default_sample_format: SampleFormat,
    pub min_buffer_frames: Option<u32>,
    pub max_buffer_frames: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct AudioDeviceInventory {
    pub devices: Vec<AudioDevice>,
    pub diagnostics: Vec<String>,
}

impl AudioOutputConfig {
    pub fn sample_format_supported_by_engine(&self) -> bool {
        matches!(
            self.sample_format,
            SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
        )
    }

    fn from_range(
        config: SupportedStreamConfigRange,
        default_config: cpal::SupportedStreamConfig,
    ) -> Self {
        let (min_buffer_frames, max_buffer_frames) = match config.buffer_size() {
            SupportedBufferSize::Range { min, max } => (Some(*min), Some(*max)),
            SupportedBufferSize::Unknown => (None, None),
        };
        Self {
            min_sample_rate: config.min_sample_rate(),
            max_sample_rate: config.max_sample_rate(),
            channels: config.channels(),
            sample_format: config.sample_format(),
            default_sample_rate: default_config.sample_rate(),
            default_channels: default_config.channels(),
            default_sample_format: default_config.sample_format(),
            min_buffer_frames,
            max_buffer_frames,
        }
    }
}

pub fn available_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let inventory = scan_audio_devices();
    if inventory.devices.is_empty() && !inventory.diagnostics.is_empty() {
        return Err(format!(
            "could not enumerate audio devices: {}",
            inventory.diagnostics.join("; ")
        ));
    }
    Ok(inventory.devices)
}

/// One diagnostic line for devices a backend lists but cannot open.
fn unavailable_devices_summary(host_name: &str, count: usize) -> String {
    if count == 1 {
        format!("{host_name}: 1 listed audio device was unavailable or disconnected and was skipped")
    } else {
        format!(
            "{host_name}: {count} listed audio devices were unavailable or disconnected and were skipped"
        )
    }
}

/// Parsed ALSA PCM name, for example `alsa:plughw:CARD=sofhdadsp,DEV=3`.
struct AlsaPcm<'a> {
    pcm: &'a str,
    card: Option<&'a str>,
    dev: Option<&'a str>,
}

impl<'a> AlsaPcm<'a> {
    fn parse(id: &'a str) -> Option<Self> {
        let rest = id.strip_prefix("alsa:")?;
        let (pcm, params) = match rest.split_once(':') {
            Some((pcm, params)) => (pcm, params),
            None => (rest, ""),
        };
        let mut card = None;
        let mut dev = None;
        for pair in params.split(',') {
            match pair.split_once('=') {
                Some(("CARD", value)) => card = Some(value),
                Some(("DEV", value)) => dev = Some(value),
                _ => {}
            }
        }
        Some(Self { pcm, card, dev })
    }

    /// `CARD=0` is an alias of `CARD=sofhdadsp`; both name the same hardware.
    fn card_is_numeric(&self) -> bool {
        self.card
            .is_some_and(|card| card.chars().all(|character| character.is_ascii_digit()))
    }

    fn key(&self) -> Option<(&'a str, &'a str)> {
        Some((self.card?, self.dev?))
    }
}

/// Collapses the raw ALSA PCM hints into one entry per endpoint.
///
/// ALSA publishes every physical device several times: once with a numeric card
/// and once with the card name, and again as `hw`, `plughw`, `dmix` and
/// `dsnoop` front ends for the same PCM. On a PipeWire desktop that turns one
/// pair of speakers into dozens of entries, which buries the one a musician
/// needs, so aliases are merged and the system default is listed first.
fn curate_alsa_devices(devices: Vec<AudioDevice>) -> Vec<AudioDevice> {
    let has_named_card = |pcm: &str, dev: &str| {
        devices.iter().any(|device| {
            AlsaPcm::parse(&device.id).is_some_and(|parsed| {
                parsed.pcm == pcm
                    && parsed.dev == Some(dev)
                    && parsed.card.is_some()
                    && !parsed.card_is_numeric()
            })
        })
    };
    let has_default = devices.iter().any(|device| {
        AlsaPcm::parse(&device.id).is_some_and(|parsed| parsed.pcm == "default")
    });

    let mut kept: Vec<AudioDevice> = devices
        .iter()
        .filter(|device| {
            let Some(parsed) = AlsaPcm::parse(&device.id) else {
                return true;
            };
            match parsed.pcm {
                // The null sink discards audio and sysdefault aliases the default.
                "null" | "sysdefault" => false,
                // `default` already points at the running sound server.
                "pulse" | "pipewire" if has_default => false,
                "hw" | "dmix" | "dsnoop" | "plughw" => {
                    !(parsed.card_is_numeric()
                        && parsed
                            .dev
                            .is_some_and(|dev| has_named_card(parsed.pcm, dev)))
                }
                _ => true,
            }
        })
        .cloned()
        .collect();

    // `plughw` converts formats and rates, so prefer it and take over the
    // capabilities reported by the other front ends for the same PCM.
    let plug_keys: Vec<(String, String)> = kept
        .iter()
        .filter_map(|device| {
            let parsed = AlsaPcm::parse(&device.id)?;
            (parsed.pcm == "plughw").then(|| {
                let (card, dev) = parsed.key()?;
                Some((card.to_owned(), dev.to_owned()))
            })?
        })
        .collect();
    for (card, dev) in &plug_keys {
        let mut supports_input = false;
        let mut supports_output = false;
        for device in &kept {
            let Some(parsed) = AlsaPcm::parse(&device.id) else {
                continue;
            };
            if parsed.card == Some(card.as_str()) && parsed.dev == Some(dev.as_str()) {
                supports_input |= device.supports_input;
                supports_output |= device.supports_output;
            }
        }
        if let Some(target) = kept.iter_mut().find(|device| {
            AlsaPcm::parse(&device.id)
                .is_some_and(|parsed| parsed.pcm == "plughw" && parsed.card == Some(card.as_str()) && parsed.dev == Some(dev.as_str()))
        }) {
            target.supports_input = supports_input;
            target.supports_output = supports_output;
        }
    }
    kept.retain(|device| {
        let Some(parsed) = AlsaPcm::parse(&device.id) else {
            return true;
        };
        if !matches!(parsed.pcm, "hw" | "dmix" | "dsnoop") {
            return true;
        }
        let Some((card, dev)) = parsed.key() else {
            return true;
        };
        !plug_keys
            .iter()
            .any(|(plug_card, plug_dev)| plug_card == card && plug_dev == dev)
    });

    // Prefer ALSA's own PCM names ("HDA Analog", "HDMI 1") over the card-only
    // description CPAL reports.
    for device in &mut kept {
        let Some(parsed) = AlsaPcm::parse(&device.id) else {
            continue;
        };
        let (pcm, card, dev) = (
            parsed.pcm.to_owned(),
            parsed.card.map(str::to_owned),
            parsed.dev.map(str::to_owned),
        );
        if pcm == "default" {
            device.name = if device.name.contains("PipeWire") {
                "System Default (PipeWire)".to_owned()
            } else {
                "System Default".to_owned()
            };
        } else if let (Some(card), Some(dev)) = (card, dev) {
            display_alsa_device(device, &card, &dev);
        }
    }

    // The internal microphone array can expose several PCMs ("DMIC",
    // "DMIC16kHz"); one friendly entry is enough.
    let mut seen_friendly = std::collections::HashSet::new();
    kept.retain(|device| {
        let friendly =
            device.name == "Laptop Speakers" || device.name == "Laptop Microphone";
        !friendly || seen_friendly.insert(device.name.clone())
    });

    // Names such as "sof-hda-dsp, " lose the empty sub-device, which leaves
    // several devices sharing one name; keep the device number to tell them apart.
    let mut duplicated: Vec<String> = Vec::new();
    for (index, device) in kept.iter().enumerate() {
        if kept[..index].iter().any(|other| other.name == device.name) {
            duplicated.push(device.name.clone());
        }
    }
    for device in &mut kept {
        if duplicated.contains(&device.name)
            && let Some(parsed) = AlsaPcm::parse(&device.id)
            && let Some(dev) = parsed.dev
        {
            device.name = format!("{} (DEV {dev})", clean_alsa_name(&device.name));
        }
    }

    kept.sort_by_key(|device| {
        let rank = match AlsaPcm::parse(&device.id).map(|parsed| parsed.pcm) {
            Some("default") => 0,
            Some("pipewire" | "pulse") => 1,
            _ => 2,
        };
        (rank, device.id.clone())
    });
    kept
}

/// Drops the trailing separator CPAL leaves for a PCM without a sub-device.
fn clean_alsa_name(name: &str) -> String {
    name.trim().trim_end_matches(',').trim_end().to_owned()
}

/// Name of the ALSA PCM behind `CARD`/`DEV`, for example `HDA Analog` or `HDMI 1`.
///
/// CPAL describes a device as the card name with an empty PCM sub-name, which
/// makes every endpoint of a laptop look identical. ALSA publishes the real
/// name in `/proc/asound`, so use it when that file is readable.
fn alsa_pcm_name(card: &str, dev: &str, playback: bool) -> Option<String> {
    let index = alsa_card_index(card)?;
    let direction = if playback { "p" } else { "c" };
    let info = std::fs::read_to_string(format!(
        "/proc/asound/card{index}/pcm{dev}{direction}/info"
    ))
    .ok()?;
    parse_alsa_pcm_info_name(&info)
}

fn parse_alsa_pcm_info_name(info: &str) -> Option<String> {
    let field = |prefix: &str| {
        info.lines()
            .find_map(|line| line.strip_prefix(prefix))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    // `name` is frequently empty for the analog PCM; `id` still names it.
    field("name:")
        .map(str::to_owned)
        .or_else(|| {
            field("id:").map(|id| id.trim_end_matches(" (*)").to_owned())
        })
}

fn alsa_card_index(card: &str) -> Option<String> {
    if !card.is_empty() && card.chars().all(|character| character.is_ascii_digit()) {
        return Some(card.to_owned());
    }
    let cards = std::fs::read_to_string("/proc/asound/cards").ok()?;
    parse_alsa_card_index(&cards, card)
}

/// Parses lines such as ` 0 [sofhdadsp      ]: sof-hda-dsp - sof-hda-dsp`.
fn parse_alsa_card_index(cards: &str, name: &str) -> Option<String> {
    cards.lines().find_map(|line| {
        let open = line.find('[')?;
        let close = line[open..].find(']').map(|offset| offset + open)?;
        (line[open + 1..close].trim() == name).then(|| line[..open].trim().to_owned())
    })
}

/// Friendly name for a built-in endpoint, if this PCM is one.
///
/// A laptop's analog codec drives the internal speakers, and a DMIC is the
/// internal microphone array; naming them after the codec (`sof-hda-dsp`) tells
/// a musician nothing. Desktop machines keep the PCM name because their analog
/// output is usually a line output or headphone jack.
fn friendly_pcm_name(pcm: &str, laptop: bool) -> Option<&'static str> {
    let pcm = pcm.to_ascii_lowercase();
    if pcm.contains("dmic") {
        return Some("Laptop Microphone");
    }
    (laptop && pcm.contains("analog")).then_some("Laptop Speakers")
}

/// True on a laptop chassis, where the internal codec feeds the built-in speakers.
fn is_laptop_chassis() -> bool {
    static LAPTOP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LAPTOP.get_or_init(|| {
        const LAPTOP_CHASSIS_TYPES: [&str; 5] = ["8", "9", "10", "14", "31"];
        if std::fs::read_to_string("/sys/class/dmi/id/chassis_type")
            .is_ok_and(|value| LAPTOP_CHASSIS_TYPES.contains(&value.trim()))
        {
            return true;
        }
        std::fs::read_dir("/sys/class/power_supply").is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("BAT")
            })
        })
    })
}

fn display_alsa_device(device: &mut AudioDevice, card: &str, dev: &str) {
    if let Some(pcm) = alsa_pcm_name(card, dev, device.supports_output) {
        if let Some(friendly) = friendly_pcm_name(&pcm, is_laptop_chassis()) {
            // Only the playback side of the analog PCM is the built-in
            // speakers; on a laptop its input side is the headset jack.
            if device.supports_output || friendly == "Laptop Microphone" {
                device.name = friendly.to_owned();
                return;
            }
        }
        // CPAL reports "<card>, <pcm>"; the card alone is the useful suffix.
        let card_label = clean_alsa_name(&device.name);
        let card_label = card_label.split(',').next().unwrap_or_default().trim();
        device.name = if card_label.is_empty() || card_label == pcm {
            pcm
        } else {
            format!("{pcm} ({card_label})")
        };
    }
}

pub fn scan_audio_devices() -> AudioDeviceInventory {
    let host_ids = cpal::available_hosts();
    if host_ids.is_empty() {
        return AudioDeviceInventory {
            devices: Vec::new(),
            diagnostics: vec!["no audio backends are available in this build".to_owned()],
        };
    }

    let mut devices = Vec::new();
    let mut failures = Vec::new();
    // ALSA lists PCM nodes for hardware that is present but not connected (for
    // example HDMI outputs with no display). Probing each one fails, and a bug
    // report full of one warning per node hides the devices that do work, so the
    // per-device detail is kept only when nothing usable was found.
    let mut unavailable_details = Vec::new();
    let mut unavailable_summaries = Vec::new();
    for host_id in host_ids {
        let mut unavailable_in_host = 0usize;
        let host_name = host_id.name().to_owned();
        let host = match cpal::host_from_id(host_id) {
            Ok(host) => host,
            Err(error) => {
                failures.push(format!("{host_name}: {error}"));
                continue;
            }
        };
        let host_devices = match host.devices() {
            Ok(devices) => devices,
            Err(error) => {
                failures.push(format!("{host_name}: {error}"));
                continue;
            }
        };
        for device in host_devices {
            let id = match device.id() {
                Ok(id) => id.to_string(),
                Err(error) => {
                    failures.push(format!(
                        "{host_name}: a device ID could not be read: {error}"
                    ));
                    continue;
                }
            };
            let supports_input = match device.supported_input_configs() {
                Ok(mut configs) => configs.next().is_some(),
                Err(error) => {
                    unavailable_details.push(format!(
                        "{host_name}: input capability query for {id} failed: {error}"
                    ));
                    unavailable_in_host += 1;
                    false
                }
            };
            let supports_output = match device.supported_output_configs() {
                Ok(mut configs) => configs.next().is_some(),
                Err(error) => {
                    unavailable_details.push(format!(
                        "{host_name}: output capability query for {id} failed: {error}"
                    ));
                    unavailable_in_host += 1;
                    false
                }
            };
            if !supports_input && !supports_output {
                continue;
            }
            let name = device
                .description()
                .map(|description| description.name().to_owned())
                .unwrap_or_else(|_| device.to_string());
            devices.push(AudioDevice {
                id,
                name,
                host_name: host_name.clone(),
                supports_input,
                supports_output,
            });
        }
        if unavailable_in_host > 0 {
            unavailable_summaries.push(unavailable_devices_summary(&host_name, unavailable_in_host));
        }
    }
    // Without any usable device the individual probe errors are the only clue
    // about what went wrong, so keep them; otherwise one line per backend.
    if devices.is_empty() {
        failures.extend(unavailable_details);
    } else {
        failures.extend(unavailable_summaries);
    }
    #[cfg(windows)]
    match crate::wasapi_backend::enumerate_active_endpoints() {
        Ok((endpoints, diagnostics)) => {
            failures.extend(diagnostics);
            for (endpoint_id, name, supports_input, supports_output) in endpoints {
                let id = format!("wasapi:{endpoint_id}");
                if let Some(existing) = devices.iter_mut().find(|device| device.id == id) {
                    existing.supports_input |= supports_input;
                    existing.supports_output |= supports_output;
                    if existing.name.is_empty() {
                        existing.name = name;
                    }
                } else {
                    devices.push(AudioDevice {
                        id,
                        name,
                        host_name: "WASAPI".to_owned(),
                        supports_input,
                        supports_output,
                    });
                }
            }
        }
        Err(error) => failures.push(format!("WASAPI endpoint enumeration: {error}")),
    }
    #[cfg(all(windows, feature = "local-asio"))]
    match maolan_asio::drivers() {
        Ok(drivers) => {
            for driver in drivers {
                devices.push(AudioDevice {
                    id: crate::asio_backend::driver_id(&driver),
                    name: driver.name,
                    host_name: "ASIO".into(),
                    supports_input: true,
                    supports_output: true,
                });
            }
        }
        Err(error) => failures.push(format!("ASIO registry: {error}")),
    }
    devices.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.host_name.cmp(&right.host_name))
            .then_with(|| left.id.cmp(&right.id))
    });
    // ALSA publishes each endpoint under several aliases; keep the device list
    // to one entry per endpoint so the usable output is easy to find.
    let devices = if devices.iter().any(|device| device.host_name == "ALSA") {
        curate_alsa_devices(devices)
    } else {
        devices
    };
    AudioDeviceInventory {
        devices,
        diagnostics: failures,
    }
}

pub fn output_configurations(device_id: Option<&str>) -> Result<Vec<AudioOutputConfig>, String> {
    if device_id.is_some_and(|id| id.starts_with("asio:")) {
        return Ok(Vec::new());
    }
    let device = match device_id {
        Some(id) => device_by_id(id)?,
        None => cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no default audio output device is available".to_owned())?,
    };
    if !device.supports_output() {
        return Err("the selected device does not support audio output".to_owned());
    }
    let default_config = device
        .default_output_config()
        .map_err(|error| format!("could not read the default output configuration: {error}"))?;
    let mut configs = device
        .supported_output_configs()
        .map_err(|error| format!("could not list output configurations: {error}"))?
        .map(|config| AudioOutputConfig::from_range(config, default_config))
        .collect::<Vec<_>>();
    configs.sort_by_key(|config| {
        (
            config.channels,
            config.min_sample_rate,
            config.max_sample_rate,
            config.sample_format,
        )
    });
    Ok(configs)
}

pub(crate) fn device_by_id(device_id: &str) -> Result<Device, String> {
    let id = DeviceId::from_str(device_id)
        .map_err(|error| format!("invalid audio device identifier: {error}"))?;
    let host = cpal::host_from_id(id.host())
        .map_err(|error| format!("the selected audio backend is unavailable: {error}"))?;
    host.device_by_id(&id)
        .ok_or_else(|| "the selected audio device is no longer available".to_owned())
}

#[cfg(not(windows))]
pub(crate) fn input_device(device_id: Option<&str>) -> Result<Device, String> {
    match device_id {
        Some(id) => device_by_id(id),
        None => cpal::default_host()
            .default_input_device()
            .ok_or_else(|| "no default audio input device is available".to_owned()),
    }
}

#[cfg(not(windows))]
pub(crate) fn output_device(device_id: Option<&str>) -> Result<Device, String> {
    match device_id {
        Some(id) => device_by_id(id),
        None => cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no default audio output device is available".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioDevice, curate_alsa_devices, friendly_pcm_name, parse_alsa_card_index,
        parse_alsa_pcm_info_name, unavailable_devices_summary,
    };

    fn alsa_device(id: &str, name: &str, input: bool, output: bool) -> AudioDevice {
        AudioDevice {
            id: id.to_owned(),
            name: name.to_owned(),
            host_name: "ALSA".to_owned(),
            supports_input: input,
            supports_output: output,
        }
    }

    #[test]
    fn alsa_card_and_pcm_names_are_read_from_proc_asound() {
        let cards = " 0 [sofhdadsp      ]: sof-hda-dsp - sof-hda-dsp\n                      Intel Corporation\n 1 [HDMI           ]: HDA-Intel - HDA Intel HDMI\n";
        assert_eq!(parse_alsa_card_index(cards, "sofhdadsp").as_deref(), Some("0"));
        assert_eq!(parse_alsa_card_index(cards, "HDMI").as_deref(), Some("1"));
        assert_eq!(parse_alsa_card_index(cards, "missing"), None);
        let info = "card: 0\ndevice: 0\nsubdevice: 0\nname: HDMI 1\n";
        assert_eq!(parse_alsa_pcm_info_name(info).as_deref(), Some("HDMI 1"));
        // The analog PCM leaves `name` empty and only carries an `id`.
        let analog = "card: 0\ndevice: 0\nsubdevice: 0\nstream: PLAYBACK\nid: HDA Analog (*)\nname: \n";
        assert_eq!(parse_alsa_pcm_info_name(analog).as_deref(), Some("HDA Analog"));
    }

    #[test]
    fn built_in_laptop_endpoints_get_readable_names() {
        assert_eq!(
            friendly_pcm_name("HDA Analog", true),
            Some("Laptop Speakers")
        );
        assert_eq!(
            friendly_pcm_name("DMIC16kHz", false),
            Some("Laptop Microphone")
        );
        assert_eq!(friendly_pcm_name("DMIC", false), Some("Laptop Microphone"));
        // A desktop analog output is a line output, not built-in speakers.
        assert_eq!(friendly_pcm_name("HDA Analog", false), None);
        assert_eq!(friendly_pcm_name("HDMI 1", true), None);
    }

    #[test]
    fn one_unavailable_device_is_reported_in_the_singular() {
        assert_eq!(
            unavailable_devices_summary("ALSA", 1),
            "ALSA: 1 listed audio device was unavailable or disconnected and was skipped"
        );
    }

    #[test]
    fn several_unavailable_devices_are_reported_once() {
        assert_eq!(
            unavailable_devices_summary("ALSA", 6),
            "ALSA: 6 listed audio devices were unavailable or disconnected and were skipped"
        );
    }

    #[test]
    fn alsa_aliases_collapse_to_the_default_and_one_entry_per_pcm() {
        let devices = vec![
            alsa_device(
                "alsa:default",
                "Default ALSA Output (currently PipeWire Media Server)",
                true,
                true,
            ),
            alsa_device(
                "alsa:null",
                "Discard all samples (playback) or generate zero samples (capture)",
                true,
                true,
            ),
            alsa_device("alsa:pipewire", "PipeWire Sound Server", true, true),
            alsa_device("alsa:sysdefault:CARD=testcard", "sof-hda-dsp, ", true, true),
            alsa_device("alsa:hw:CARD=0,DEV=0", "sof-hda-dsp, ", true, true),
            alsa_device("alsa:hw:CARD=testcard,DEV=0", "sof-hda-dsp, ", true, true),
            alsa_device("alsa:plughw:CARD=0,DEV=0", "sof-hda-dsp, ", false, true),
            alsa_device("alsa:plughw:CARD=testcard,DEV=0", "sof-hda-dsp, ", false, true),
            alsa_device("alsa:dmix:CARD=testcard,DEV=0", "sof-hda-dsp, ", false, true),
            alsa_device(
                "alsa:plughw:CARD=testcard,DEV=3",
                "sof-hda-dsp, HDMI 1",
                false,
                true,
            ),
            alsa_device(
                "alsa:hw:CARD=testcard,DEV=3",
                "sof-hda-dsp, HDMI 1",
                false,
                true,
            ),
        ];
        let curated = curate_alsa_devices(devices);
        let ids: Vec<&str> = curated.iter().map(|device| device.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "alsa:default",
                "alsa:plughw:CARD=testcard,DEV=0",
                "alsa:plughw:CARD=testcard,DEV=3",
            ]
        );
        // The merged `plughw` entry keeps the input capability `hw` reported.
        assert!(curated[1].supports_input && curated[1].supports_output);
        assert!(!curated[2].supports_input);
    }

    #[test]
    fn endpoints_sharing_a_name_keep_their_device_number() {
        let curated = curate_alsa_devices(vec![
            alsa_device("alsa:plughw:CARD=testcard,DEV=0", "sof-hda-dsp, ", true, true),
            alsa_device(
                "alsa:plughw:CARD=testcard,DEV=31",
                "sof-hda-dsp, ",
                false,
                true,
            ),
        ]);
        assert_eq!(curated[0].name, "sof-hda-dsp (DEV 0)");
        assert_eq!(curated[1].name, "sof-hda-dsp (DEV 31)");
    }
}
