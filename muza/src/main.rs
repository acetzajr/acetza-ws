use std::{fs, io::Result, path::Path, thread};

pub type WaveForm = fn(x: f64) -> f64;

pub mod constants {
    pub const FRAME_RATE: usize = 48_000;
    pub const CHANNELS: usize = 2;
}

pub mod wave_forms {
    use std::f64::consts::PI;

    pub fn sqr(x: f64) -> f64 {
        if x < 0.5 {
            1.0
        } else {
            -1.0
        }
    }

    pub fn saw(x: f64) -> f64 {
        1.0 - 2.0 * x
    }

    pub fn tri(x: f64) -> f64 {
        if x < 0.25 {
            return 4.0 * x;
        }
        if x < 0.75 {
            return 2.0 - 4.0 * x;
        }
        4.0 * x - 4.0
    }

    pub fn sin(x: f64) -> f64 {
        (2.0 * PI * x).sin()
    }
}

pub struct WaveFormerBuilder {
    waveform: Option<WaveForm>,
    duration: Option<f64>,
    frequency: Option<f64>,
}

pub struct WaveFormer {
    pub waveform: WaveForm,
    pub duration: f64,
    pub frequency: f64,
}

pub fn duration_to_frame(duration: f64) -> usize {
    (duration * constants::FRAME_RATE as f64) as usize
}
pub fn frame_to_duration(frame: usize) -> f64 {
    frame as f64 / constants::FRAME_RATE as f64
}

impl WaveFormer {
    pub fn render<S: AsRef<Path>>(&mut self, path: S) {
        use hound;
        let spec = hound::WavSpec {
            channels: constants::CHANNELS as u16,
            sample_rate: constants::FRAME_RATE as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create::<S>(path, spec).unwrap();
        for _t in (0..44100).map(|x| x as f32 / 44100.0) {}
        let frames_count = duration_to_frame(self.duration);
        for frame in 0..frames_count {
            let seconds = frame_to_duration(frame);
            let part = seconds * self.frequency % 1.0;
            let sample = (self.waveform)(part) * 0.5;
            for _channel in 0..constants::CHANNELS {
                writer.write_sample(sample as f32).unwrap();
            }
        }
    }
}

impl Default for WaveFormerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveFormerBuilder {
    pub fn new() -> Self {
        Self {
            waveform: None,
            duration: None,
            frequency: None,
        }
    }
    pub fn waveform(mut self, waveform: WaveForm) -> Self {
        self.waveform = Some(waveform);
        self
    }
    pub fn duration(mut self, duration: f64) -> Self {
        self.duration = Some(duration);
        self
    }
    pub fn frequency(mut self, frequency: f64) -> Self {
        self.frequency = Some(frequency);
        self
    }
    pub fn build(self) -> WaveFormer {
        WaveFormer {
            waveform: self.waveform.unwrap_or(wave_forms::sin),
            duration: self.duration.unwrap_or(1.0),
            frequency: self.duration.unwrap_or(360.0),
        }
    }
}

#[derive(Clone)]
struct Ruler {
    frequency: f64,
    bpm: f64,
    rations: [f64; 12],
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            frequency: 440.0,
            bpm: 120.0,
            rations: [
                1.0,            // 0
                256.0 / 243.0,  // 1
                9.0 / 8.0,      // 2
                32.0 / 27.0,    // 3
                81.0 / 64.0,    // 4
                4.0 / 3.0,      // 5
                2.0_f64.sqrt(), // 6
                3.0 / 2.0,      // 7
                128.0 / 81.0,   // 8
                27.0 / 16.0,    // 9
                16.0 / 9.0,     // 10
                256.0 / 128.0,  // 11
            ],
        }
    }
}

impl Ruler {
    pub fn ration(&self, note: i64) -> f64 {
        self.rations[note.rem_euclid(self.rations.len() as i64) as usize]
    }
    pub fn power(&self, note: i64) -> f64 {
        let length = self.rations.len() as i32;
        let note = note as i32;
        2.0_f64.powi(if note < 0 {
            (note + 1) / length - 1
        } else {
            note / length
        })
    }
    pub fn frequency(&self, note: i64) -> f64 {
        self.frequency * self.ration(note) * self.power(note)
    }
    pub fn duration(&self, ration: f64) -> f64 {
        self.bpm / 60.0 * ration
    }
}
fn main() -> Result<()> {
    fs::remove_dir_all("out")?;
    fs::create_dir("out")?;
    let ruler = Ruler {
        frequency: 440.0,
        ..Default::default()
    };
    let octaves = 8;
    let offset = 36;
    println!("{}", ruler.frequency(-offset));
    println!("{}", ruler.frequency(-offset + octaves * 12 - 1));
    let mut handles = Vec::with_capacity(octaves as usize);
    for job in 0..octaves {
        let ruler = ruler.clone();
        handles.push(thread::spawn(move || {
            let lengths = [1, 2, 4, 8];
            fs::create_dir(format!("out/o[{}]", job)).unwrap();
            let mut waveformer = WaveFormerBuilder::new().build();
            let start = job * 12 - offset;
            for note in start..start + 12 {
                let abs = note - start;
                //let sign = if note < 0 { "-" } else { "+" };
                fs::create_dir(format!("out/o[{}]/n[{}]", job, abs)).unwrap();
                for length in lengths {
                    waveformer.frequency = ruler.frequency(note);
                    waveformer.duration = ruler.duration(length as f64);
                    waveformer.render(format!(
                        "out/o[{}]/n[{}]/o[{}] n[{}] l[{}].wav",
                        job, abs, job, abs, length
                    ));
                }
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(())
}
