//! An original Rust DSP prototype for Cubase 5's TestGenerator.
//!
//! The control surface and broad waveform list follow the Cubase 5.0 Plug-in
//! Reference. The reference algorithm has not been measured. Waveform and
//! noise details here are original approximations, not a claim of Cubase-
//! compatible output.
//!
//! Noise uses a fixed seed for repeatability. The reference noise algorithms,
//! seeding, frequency-control behavior in noise modes, and anti-aliasing have
//! not been measured.

const MIN_FREQUENCY_HZ: f32 = 1.0;
const MAX_FREQUENCY_HZ: f32 = 20_000.0;
const INITIAL_NOISE_SEED: u64 = 0x4f70_656e_5472_6163;

/// Waveforms documented for the Cubase 5 TestGenerator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestWaveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    WhiteNoise,
    BrownNoise,
    PinkNoise,
}

/// User-facing controls described by the Cubase 5 Plug-in Reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TestGeneratorParameters {
    pub waveform: TestWaveform,
    pub frequency_hz: f32,
    /// Gain in decibels. The reference specifies a maximum of 0 dB but no
    /// minimum, so this model accepts any finite value at or below 0 dB.
    pub gain_db: f32,
}

/// Allocation-free mono signal generator for offline or real-time callers.
#[derive(Clone, Debug)]
pub struct TestGenerator {
    sample_rate: u32,
    parameters: TestGeneratorParameters,
    linear_gain: f32,
    phase: f64,
    random_state: u64,
    brown_state: f32,
    pink_state: [f32; 7],
}

impl TestGenerator {
    /// Creates a generator. The documented frequency range is 1 Hz through
    /// 20 kHz; the reference does not specify a lower gain limit.
    pub fn new(
        sample_rate: u32,
        parameters: TestGeneratorParameters,
    ) -> Result<Self, &'static str> {
        if sample_rate == 0 {
            return Err("sample rate must be nonzero");
        }
        let linear_gain = validate_parameters(parameters)?;
        Ok(Self {
            sample_rate,
            parameters,
            linear_gain,
            phase: 0.0,
            random_state: INITIAL_NOISE_SEED,
            brown_state: 0.0,
            pink_state: [0.0; 7],
        })
    }

    pub fn parameters(&self) -> TestGeneratorParameters {
        self.parameters
    }

    /// Updates controls without allocating. The phase and noise-filter state
    /// continue across parameter changes.
    pub fn set_parameters(
        &mut self,
        parameters: TestGeneratorParameters,
    ) -> Result<(), &'static str> {
        let linear_gain = validate_parameters(parameters)?;
        self.parameters = parameters;
        self.linear_gain = linear_gain;
        Ok(())
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<(), &'static str> {
        if sample_rate == 0 {
            return Err("sample rate must be nonzero");
        }
        self.sample_rate = sample_rate;
        Ok(())
    }

    /// Fills a mono output slice without allocating or locking.
    pub fn render_mono(&mut self, output: &mut [f32]) {
        for sample in output {
            *sample = self.next_sample();
        }
    }

    /// Produces one sample without allocating or locking.
    pub fn next_sample(&mut self) -> f32 {
        let phase_step = f64::from(self.parameters.frequency_hz) / f64::from(self.sample_rate);
        let value = match self.parameters.waveform {
            TestWaveform::Sine => (self.phase * std::f64::consts::TAU).sin() as f32,
            TestWaveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            TestWaveform::Sawtooth => (self.phase * 2.0 - 1.0) as f32,
            TestWaveform::Triangle => (1.0 - 4.0 * (self.phase - 0.5).abs()) as f32,
            TestWaveform::WhiteNoise => self.next_white_noise(),
            TestWaveform::BrownNoise => self.next_brown_noise(),
            TestWaveform::PinkNoise => self.next_pink_noise(),
        };
        self.phase = (self.phase + phase_step).fract();
        value * self.linear_gain
    }

    /// Resets oscillator phase and colored-noise history to the initial seed.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.random_state = INITIAL_NOISE_SEED;
        self.brown_state = 0.0;
        self.pink_state = [0.0; 7];
    }

    fn next_white_noise(&mut self) -> f32 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value;
        let unit = (value >> 40) as f32 / (1_u32 << 24) as f32;
        unit * 2.0 - 1.0
    }

    fn next_brown_noise(&mut self) -> f32 {
        let white = self.next_white_noise();
        self.brown_state = (self.brown_state + 0.02 * white) / 1.02;
        self.brown_state * 3.5
    }

    fn next_pink_noise(&mut self) -> f32 {
        let white = self.next_white_noise();
        self.pink_state[0] = 0.99886 * self.pink_state[0] + white * 0.0555179;
        self.pink_state[1] = 0.99332 * self.pink_state[1] + white * 0.0750759;
        self.pink_state[2] = 0.96900 * self.pink_state[2] + white * 0.1538520;
        self.pink_state[3] = 0.86650 * self.pink_state[3] + white * 0.3104856;
        self.pink_state[4] = 0.55000 * self.pink_state[4] + white * 0.5329522;
        self.pink_state[5] = -0.7616 * self.pink_state[5] - white * 0.0168980;
        let pink = self.pink_state.iter().sum::<f32>() + white * 0.5362;
        self.pink_state[6] = white * 0.115926;
        pink * 0.11
    }
}

fn validate_parameters(parameters: TestGeneratorParameters) -> Result<f32, &'static str> {
    if !parameters.frequency_hz.is_finite()
        || !(MIN_FREQUENCY_HZ..=MAX_FREQUENCY_HZ).contains(&parameters.frequency_hz)
    {
        return Err("frequency must be between 1 Hz and 20,000 Hz");
    }
    if !parameters.gain_db.is_finite() || parameters.gain_db > 0.0 {
        return Err("gain must be finite and no greater than 0 dB");
    }
    Ok(10.0_f32.powf(parameters.gain_db / 20.0))
}
