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
    for host_id in host_ids {
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
                    failures.push(format!(
                        "{host_name}: input capability query for {id} failed: {error}"
                    ));
                    false
                }
            };
            let supports_output = match device.supported_output_configs() {
                Ok(mut configs) => configs.next().is_some(),
                Err(error) => {
                    failures.push(format!(
                        "{host_name}: output capability query for {id} failed: {error}"
                    ));
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
