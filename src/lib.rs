use nih_plug::prelude::*;
// use parking_lot::Mutex;
use std::sync::Arc;

const BSM_BUFFER_SIZE: usize = 16;
const BSM_BUFFER_MASK: usize = 15;

struct PuluGrit {
    params: Arc<PuluGritParams>,
    sample_rate: f32,
    bsm_env: f32,
    bsm_buffer: [f32; BSM_BUFFER_SIZE],
    bsm_buffer_pos: usize,
}

impl PuluGrit {
    fn clip_process(
        &mut self,
        buffer: &mut Buffer,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                let drive = self.params.clip_drive.value();
                let val = *sample;
                *sample = (val * (drive + 1.0)).max(-1.0).min(1.0);
            }
        }

        ProcessStatus::Normal
    }

    fn superdirt_shape_process(
        &mut self,
        buffer: &mut Buffer,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                let mut shape = self.params.sds_shape.value();
                let val = *sample;
                shape = shape.min(1.0 - 4e-8); // avoid division by zero
	        //amp = 1.0 - (0.15 * shape / (shape + 2.0)) * amp; // optional gain comp
	        shape = (2.0 * shape) / (1.0 - shape);
                *sample = (1.0 + shape) * val / (1.0 + (shape * val.abs()));
            }
        }

        ProcessStatus::Normal
    }

    fn barrys_satan_maximizer_process(
        &mut self,
        buffer: &mut Buffer,
    ) -> ProcessStatus {
        let env_time = (self.params.bsm_env_time.value() * self.sample_rate)
            .max(2.0);
        let knee = self.params.bsm_knee.value();
        let delay = (env_time * 0.5).round() as usize;
        let env_tr = 1.0 / env_time;

        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                let input = *sample;
                self.bsm_env = if input.abs() > self.bsm_env {
                    input.abs()
                } else {
                    input.abs() * env_tr + self.bsm_env * (1.0 - env_tr)
                };
                let env_sc = if self.bsm_env <= knee {
                    1.0 / knee
                } else {
                    1.0 / self.bsm_env
                };
                self.bsm_buffer[self.bsm_buffer_pos] = input;
                *sample = self.bsm_buffer[(self.bsm_buffer_pos - delay) & BSM_BUFFER_MASK] * env_sc;
                self.bsm_buffer_pos = (self.bsm_buffer_pos + 1) & BSM_BUFFER_MASK;
            }
        }

        // for (pos = 0; pos < sample_count; pos++) {
	//     if (fabs(input[pos]) > env) {
	//         env = fabs(input[pos]);
	//     } else {
	//         env = fabs(input[pos]) * env_tr + env * (1.0f - env_tr);
	//     }
	//     if (env <= knee) {
	//         env_sc = 1.0f / knee;
	//     } else {
	//         env_sc = 1.0f / env;
	//     }
	//     buffer[buffer_pos] = input[pos];
	//     output[pos] = buffer[(buffer_pos - delay) & BUFFER_MASK] * env_sc;
	//     buffer_pos = (buffer_pos + 1) & BUFFER_MASK;
        // }

        // plugin_data->env = env;
        // plugin_data->buffer_pos = buffer_pos;
        ProcessStatus::Normal
    }
}

impl Default for PuluGrit {
    fn default() -> Self {
        Self {
            params: Arc::new(PuluGritParams::default()),
            sample_rate: 1.0,
            bsm_env: 0.0,
            bsm_buffer: [0.0; BSM_BUFFER_SIZE],
            bsm_buffer_pos: 0,
        }
    }
}

pub fn v2s_algorithm_formatter() -> Arc<dyn Fn(i32) -> String + Send + Sync> {
    Arc::new(move |value| {
        let names = [
            "Clip",
            "SuperDirt Shape",
            "Barry's Satan Maximizer"
        ];
        let name = if value >= 0 && (value as usize) < names.len() {
            names[value as usize]
        } else {
            "???"
        };
        format!("{}", name)
    })
}

#[derive(Params)]
struct PuluGritParams {
    #[id = "algorithm"]
    pub algorithm: IntParam,

    #[id = "clip_drive"]
    pub clip_drive: FloatParam,

    #[id = "sds_shape"]
    pub sds_shape: FloatParam,

    #[id = "bsm_env_time"]
    pub bsm_env_time: FloatParam,
    
    #[id = "bsm_knee"]
    pub bsm_knee: FloatParam,
}

impl Default for PuluGritParams {
    fn default() -> Self {
        Self {
            algorithm: IntParam::new(
                "Algorithm",
                0,
                IntRange::Linear {
                    min: 0,
                    max: 2,
                },
            )
                .with_value_to_string(v2s_algorithm_formatter())
            // .with_string_to_value(formatters::s2v_f32_gain_to_db())
                ,
            
            clip_drive: FloatParam::new(
                "Clip: Drive",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),
            
            sds_shape: FloatParam::new(
                "SDS: Shape",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),
            
            bsm_env_time: FloatParam::new(
                "BSM: Env Time",
                1.0e-3,
                FloatRange::Linear {
                    min: 0.1e-3,
                    max: 1.0e-3,
                },
            ),
            
            bsm_knee: FloatParam::new(
                "BSM: Knee",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-90.0),
                    max: util::db_to_gain(0.0),
                    factor: FloatRange::gain_skew_factor(-90.0, 0.0),
                },
            )
                .with_smoother(SmoothingStyle::Logarithmic(50.0))
                .with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db())
                ,
        }
    }
}

impl Plugin for PuluGrit {
    const NAME: &'static str = "pulu-grit";
    const VENDOR: &'static str = "pulusound";
    // You can use `env!("CARGO_PKG_HOMEPAGE")` to reference the homepage field from the
    // `Cargo.toml` file here
    const URL: &'static str = "https://pulusound.fi";
    const EMAIL: &'static str = "miranda@pulusound.fi";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],

            // Individual ports and the layout as a whole can be named here. By default these names
            // are generated as needed. This layout will be called 'Stereo', while the other one is
            // given the name 'Mono' based no the number of input and output channels.
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    // Setting this to `true` will tell the wrapper to split the buffer up into smaller blocks
    // whenever there are inter-buffer parameter changes. This way no changes to the plugin are
    // required to support sample accurate automation and the wrapper handles all of the boring
    // stuff like making sure transport and other timing information stays consistent between the
    // splits.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.reset();

        true
    }

    fn reset(&mut self) {
        self.bsm_env = 0.0;
        self.bsm_buffer.fill(0.0);
        self.bsm_buffer_pos = 0;
    }
    
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let algorithm = self.params.algorithm.value();

        match algorithm {
            0 => self.clip_process(buffer),
            1 => self.superdirt_shape_process(buffer),
            2 => self.barrys_satan_maximizer_process(buffer),
            _ => ProcessStatus::Normal
        }
    }

    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
    fn deactivate(&mut self) {}
}

impl ClapPlugin for PuluGrit {
    const CLAP_ID: &'static str = "fi.pulusound.pulu-grit";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for PuluGrit {
    const VST3_CLASS_ID: [u8; 16] = *b"puluGrit        ";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(PuluGrit);
nih_export_vst3!(PuluGrit);
