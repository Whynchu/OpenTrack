//! Typed, validated channel routing.
//!
//! Routing used to be free-form strings: the arrange plan, the send rack and the
//! project file each parsed `"Group:7"` or `"Left - Stereo In"` on their own, and
//! an unrecognized value silently became the stereo master. That made a wrong
//! route indistinguishable from a missing one and let a project save a route it
//! could not play.
//!
//! This module is the single vocabulary. Persisted strings keep their original
//! spelling so existing projects load unchanged, but they are parsed once,
//! validated against the project, and then handed to the audio plan as values.

use serde::{Deserialize, Serialize};

/// Physical input channels the capture path can address today.
///
/// The input stream is opened as a stereo bus and the recorder writes one or two
/// of its channels, so a mono route may name channel 1 or 2. [`InputRoute`] can
/// express any channel, which is what a wider capture bus would need, but
/// validation rejects anything the engine cannot deliver instead of folding it
/// into the left channel.
pub const MAX_INPUT_CHANNELS: u8 = 2;

/// Why a routing value cannot be used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutingError {
    /// The persisted text is not a route this build understands.
    UnknownRoute(String),
    /// A mono input names a channel the capture bus does not provide.
    UnsupportedInputChannel { channel: u8, available: u8 },
    /// The output names a group that does not exist.
    MissingGroup { group: u64 },
    /// A channel routes to itself.
    SelfGroup { group: u64 },
    /// The output would send the channel's audio back into its own input.
    RoutingCycle { group: u64 },
    /// A send names an FX channel that does not exist.
    MissingFx { fx: u64 },
    /// A send would feed its own FX channel.
    SelfFx { fx: u64 },
    /// An FX send chain would create a feedback loop.
    FxRoutingCycle { fx: u64 },
    /// An FX channel is a return and cannot be a main output destination.
    FxIsSendOnly { fx: u64 },
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRoute(route) => {
                write!(formatter, "unknown routing '{route}'")
            }
            Self::UnsupportedInputChannel { channel, available } => write!(
                formatter,
                "input channel {channel} is outside the {available} channel(s) this input provides"
            ),
            Self::MissingGroup { group } => {
                write!(formatter, "routing target group {group} does not exist")
            }
            Self::SelfGroup { group } => {
                write!(formatter, "channel cannot route to itself as group {group}")
            }
            Self::RoutingCycle { group } => write!(
                formatter,
                "routing to group {group} would create a feedback loop"
            ),
            Self::MissingFx { fx } => {
                write!(formatter, "routing target FX channel {fx} does not exist")
            }
            Self::SelfFx { fx } => {
                write!(
                    formatter,
                    "channel cannot send to itself as FX channel {fx}"
                )
            }
            Self::FxRoutingCycle { fx } => write!(
                formatter,
                "routing to FX channel {fx} would create a feedback loop"
            ),
            Self::FxIsSendOnly { fx } => write!(
                formatter,
                "FX channel {fx} is a return and cannot be a main output"
            ),
        }
    }
}

/// Where a channel takes its audio from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRoute {
    /// No input bus is assigned; nothing reaches this channel.
    NoBus,
    /// Both channels of the selected input device.
    StereoIn,
    /// One physical channel of the selected input device, numbered from 1.
    MonoIn { channel: u8 },
}

impl InputRoute {
    /// Parses the persisted spelling, including the historical mono labels.
    pub fn parse(value: &str) -> Result<Self, RoutingError> {
        match value.trim() {
            "No Bus" => Ok(Self::NoBus),
            "Stereo In" => Ok(Self::StereoIn),
            // Historical labels for the two channels of a stereo input device.
            "Left - Stereo In" => Ok(Self::MonoIn { channel: 1 }),
            "Right - Stereo In" => Ok(Self::MonoIn { channel: 2 }),
            other => other
                .strip_prefix("Input ")
                .and_then(|channel| channel.trim().parse::<u8>().ok())
                .filter(|channel| *channel > 0)
                .map(|channel| Self::MonoIn { channel })
                .ok_or_else(|| RoutingError::UnknownRoute(other.to_owned())),
        }
    }

    /// The spelling stored in the project file.
    ///
    /// Channels 1 and 2 keep their historical labels so an existing project
    /// round-trips byte for byte.
    pub fn persisted(self) -> String {
        match self {
            Self::NoBus => "No Bus".to_owned(),
            Self::StereoIn => "Stereo In".to_owned(),
            Self::MonoIn { channel: 1 } => "Left - Stereo In".to_owned(),
            Self::MonoIn { channel: 2 } => "Right - Stereo In".to_owned(),
            Self::MonoIn { channel } => format!("Input {channel}"),
        }
    }

    /// Short label for the routing field and its menu.
    pub fn label(self) -> String {
        match self {
            Self::NoBus => "No Bus".to_owned(),
            Self::StereoIn => "Stereo In (1+2)".to_owned(),
            Self::MonoIn { channel: 1 } => "Input 1 (Left)".to_owned(),
            Self::MonoIn { channel: 2 } => "Input 2 (Right)".to_owned(),
            Self::MonoIn { channel } => format!("Input {channel}"),
        }
    }

    /// Channels of the input device this route needs.
    pub const fn channels_required(self) -> u8 {
        match self {
            Self::NoBus => 0,
            Self::StereoIn => 2,
            Self::MonoIn { channel } => channel,
        }
    }

    /// Checks the route against the channels the input device provides.
    pub fn validate(self, available_channels: u8) -> Result<(), RoutingError> {
        let needed = self.channels_required();
        if needed > available_channels {
            return Err(RoutingError::UnsupportedInputChannel {
                channel: needed,
                available: available_channels,
            });
        }
        Ok(())
    }
}

/// Where a channel sends its audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputRoute {
    /// Nothing reaches the output.
    NoBus,
    /// Both channels of the stereo output.
    StereoOut,
    /// The left channel only, as a speaker assignment.
    Left,
    /// The right channel only, as a speaker assignment.
    Right,
    /// A group channel, summed into that group's bus.
    Group(u64),
    /// A shared effect return, used by sends only.
    Fx(u64),
}

impl OutputRoute {
    pub fn parse(value: &str) -> Result<Self, RoutingError> {
        match value.trim() {
            "No Bus" => Ok(Self::NoBus),
            "Stereo Out" => Ok(Self::StereoOut),
            "Left" => Ok(Self::Left),
            "Right" => Ok(Self::Right),
            other => {
                if let Some(id) = other
                    .strip_prefix("Group:")
                    .and_then(|id| id.trim().parse::<u64>().ok())
                {
                    return Ok(Self::Group(id));
                }
                other
                    .strip_prefix("FX:")
                    .and_then(|id| id.trim().parse::<u64>().ok())
                    .map(Self::Fx)
                    .ok_or_else(|| RoutingError::UnknownRoute(other.to_owned()))
            }
        }
    }

    pub fn persisted(self) -> String {
        match self {
            Self::NoBus => "No Bus".to_owned(),
            Self::StereoOut => "Stereo Out".to_owned(),
            Self::Left => "Left".to_owned(),
            Self::Right => "Right".to_owned(),
            Self::Group(id) => format!("Group:{id}"),
            Self::Fx(id) => format!("FX:{id}"),
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::NoBus => "No Bus".to_owned(),
            Self::StereoOut => "Stereo Out (1+2)".to_owned(),
            Self::Left => "Left only".to_owned(),
            Self::Right => "Right only".to_owned(),
            Self::Group(id) => format!("Group:{id}"),
            Self::Fx(id) => format!("FX:{id}"),
        }
    }

    /// The group channel this route feeds, when it feeds one.
    pub const fn group(self) -> Option<u64> {
        match self {
            Self::Group(id) => Some(id),
            _ => None,
        }
    }

    /// The FX return this route feeds, when it feeds one.
    pub const fn fx(self) -> Option<u64> {
        match self {
            Self::Fx(id) => Some(id),
            _ => None,
        }
    }

    /// The bus channel this route feeds, group or FX.
    pub const fn bus(self) -> Option<u64> {
        match self {
            Self::Group(id) | Self::Fx(id) => Some(id),
            _ => None,
        }
    }

    /// Speaker assignment for the main stereo output, if this is one.
    pub const fn speaker(self) -> Option<Speaker> {
        match self {
            Self::StereoOut => Some(Speaker::Stereo),
            Self::Left => Some(Speaker::Left),
            Self::Right => Some(Speaker::Right),
            Self::NoBus => Some(Speaker::None),
            Self::Group(_) | Self::Fx(_) => None,
        }
    }
}

/// Which speaker of the stereo output a channel is assigned to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Speaker {
    None,
    Left,
    Right,
    Stereo,
}

impl Speaker {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "No speaker",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Stereo => "Stereo",
        }
    }
}

/// Every output destination a channel may pick, in menu order.
///
/// `groups` is `(id, name)` for the group channels that would not create a
/// cycle, so the caller decides which groups are legal and the menu cannot
/// offer a route that validation would reject.
pub fn output_destinations(groups: &[(u64, String)]) -> Vec<OutputRoute> {
    let mut destinations = vec![
        OutputRoute::StereoOut,
        OutputRoute::Left,
        OutputRoute::Right,
        OutputRoute::NoBus,
    ];
    destinations.extend(groups.iter().map(|(id, _)| OutputRoute::Group(*id)));
    destinations
}

/// Every send destination a channel may pick, in menu order.
///
/// Group and FX lists contain only channels that would not create a cycle, so
/// the caller decides which buses are legal and the menu cannot offer a route
/// that validation would reject. Main output menus intentionally use only
/// [`output_destinations`], because an FX channel is a send return.
pub fn send_destinations(groups: &[(u64, String)], fx: &[(u64, String)]) -> Vec<OutputRoute> {
    let mut destinations = output_destinations(groups);
    destinations.extend(fx.iter().map(|(id, _)| OutputRoute::Fx(*id)));
    destinations
}

/// Every input route the capture bus can deliver.
pub fn input_destinations(available_channels: u8) -> Vec<InputRoute> {
    let mut destinations = vec![InputRoute::StereoIn];
    destinations.extend(
        (1..=available_channels.min(MAX_INPUT_CHANNELS))
            .map(|channel| InputRoute::MonoIn { channel }),
    );
    destinations.push(InputRoute::NoBus);
    destinations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_spellings_round_trip_without_migrating_projects() {
        for (text, route) in [
            ("No Bus", InputRoute::NoBus),
            ("Stereo In", InputRoute::StereoIn),
            ("Left - Stereo In", InputRoute::MonoIn { channel: 1 }),
            ("Right - Stereo In", InputRoute::MonoIn { channel: 2 }),
            ("Input 5", InputRoute::MonoIn { channel: 5 }),
        ] {
            assert_eq!(InputRoute::parse(text), Ok(route));
            assert_eq!(route.persisted(), text);
        }
        for (text, route) in [
            ("No Bus", OutputRoute::NoBus),
            ("Stereo Out", OutputRoute::StereoOut),
            ("Left", OutputRoute::Left),
            ("Right", OutputRoute::Right),
            ("Group:7", OutputRoute::Group(7)),
            ("FX:9", OutputRoute::Fx(9)),
        ] {
            assert_eq!(OutputRoute::parse(text), Ok(route));
            assert_eq!(route.persisted(), text);
        }
    }

    #[test]
    fn unknown_routing_is_reported_instead_of_becoming_the_master() {
        assert_eq!(
            OutputRoute::parse("Group:"),
            Err(RoutingError::UnknownRoute("Group:".to_owned()))
        );
        assert_eq!(
            OutputRoute::parse("Surround 5.1"),
            Err(RoutingError::UnknownRoute("Surround 5.1".to_owned()))
        );
        assert!(InputRoute::parse("Input 0").is_err());
        assert!(InputRoute::parse("Adat 9").is_err());
    }

    #[test]
    fn mono_inputs_are_validated_against_the_capture_bus_width() {
        assert_eq!(InputRoute::MonoIn { channel: 1 }.validate(2), Ok(()));
        assert_eq!(InputRoute::StereoIn.validate(2), Ok(()));
        assert_eq!(
            InputRoute::MonoIn { channel: 2 }.validate(1),
            Err(RoutingError::UnsupportedInputChannel {
                channel: 2,
                available: 1
            })
        );
        // A wider model is expressible even though the bus is stereo today.
        assert_eq!(
            InputRoute::MonoIn { channel: 3 }.validate(2),
            Err(RoutingError::UnsupportedInputChannel {
                channel: 3,
                available: 2
            })
        );
    }

    #[test]
    fn speaker_assignment_only_covers_the_stereo_output() {
        assert_eq!(OutputRoute::Left.speaker(), Some(Speaker::Left));
        assert_eq!(OutputRoute::Right.speaker(), Some(Speaker::Right));
        assert_eq!(OutputRoute::StereoOut.speaker(), Some(Speaker::Stereo));
        assert_eq!(OutputRoute::NoBus.speaker(), Some(Speaker::None));
        assert_eq!(OutputRoute::Group(3).speaker(), None);
        assert_eq!(OutputRoute::Group(3).group(), Some(3));
        assert_eq!(OutputRoute::Fx(4).speaker(), None);
        assert_eq!(OutputRoute::Fx(4).fx(), Some(4));
        assert_eq!(OutputRoute::Fx(4).group(), None);
        assert_eq!(OutputRoute::Fx(4).bus(), Some(4));
        assert_eq!(OutputRoute::Group(3).bus(), Some(3));
        assert_eq!(OutputRoute::Left.group(), None);
    }

    #[test]
    fn destination_lists_match_what_validation_accepts() {
        let groups = [(4_u64, "Drums".to_owned())];
        let fx = [(5_u64, "Hall".to_owned())];
        assert_eq!(
            output_destinations(&groups),
            vec![
                OutputRoute::StereoOut,
                OutputRoute::Left,
                OutputRoute::Right,
                OutputRoute::NoBus,
                OutputRoute::Group(4),
            ]
        );
        assert_eq!(
            send_destinations(&groups, &fx),
            vec![
                OutputRoute::StereoOut,
                OutputRoute::Left,
                OutputRoute::Right,
                OutputRoute::NoBus,
                OutputRoute::Group(4),
                OutputRoute::Fx(5),
            ]
        );
        assert_eq!(
            input_destinations(2),
            vec![
                InputRoute::StereoIn,
                InputRoute::MonoIn { channel: 1 },
                InputRoute::MonoIn { channel: 2 },
                InputRoute::NoBus,
            ]
        );
        assert_eq!(
            input_destinations(1),
            vec![
                InputRoute::StereoIn,
                InputRoute::MonoIn { channel: 1 },
                InputRoute::NoBus,
            ]
        );
    }

    #[test]
    fn labels_name_the_physical_channel() {
        assert_eq!(InputRoute::MonoIn { channel: 1 }.label(), "Input 1 (Left)");
        assert_eq!(InputRoute::MonoIn { channel: 2 }.label(), "Input 2 (Right)");
        assert_eq!(InputRoute::StereoIn.label(), "Stereo In (1+2)");
        assert_eq!(OutputRoute::StereoOut.label(), "Stereo Out (1+2)");
        assert_eq!(Speaker::Left.label(), "Left");
    }
}
