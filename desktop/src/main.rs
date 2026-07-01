use greybound::{amp::VoxAmp, AmpControls, Greybound};
use greybound_ui::{GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};
use std::time::Duration;

fn main() -> iced::Result {
    Desktop::run(Settings {
        window: iced::window::Settings {
            size: (DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32),
            min_size: Some(((DESIGN_WIDTH * 0.55) as u32, (DESIGN_HEIGHT * 0.55) as u32)),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

struct Desktop {
    ui: GreyboundUi,
    plugin: Greybound,
    meter_probe: MeterProbe,
}

struct MeterProbe {
    amp: VoxAmp,
    phase: f32,
    modulation_phase: f32,
    input_level: f32,
    output_level: f32,
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Desktop {
                ui: GreyboundUi::default(),
                plugin: Greybound::default(),
                meter_probe: MeterProbe::new(48_000.0),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Desktop")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if let Message::MeterProbeTick(_) = message {
            let controls = amp_controls(&self.ui);
            let (input, output) = self.meter_probe.tick(controls);
            self.ui
                .update(Message::MeterLevelsChanged { input, output });
            return Command::none();
        }

        self.ui.update(message);
        self.plugin.set_ui_controls(amp_controls(&self.ui));
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.ui.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::subscription::events_with(|event, _status| match event {
                iced::Event::Window(iced::window::Event::Resized { width, height }) => {
                    Some(Message::WindowResized { width, height })
                }
                _ => None,
            }),
            iced::time::every(Duration::from_millis(33)).map(Message::MeterProbeTick),
        ])
    }
}

impl MeterProbe {
    fn new(sample_rate: f32) -> Self {
        Self {
            amp: VoxAmp::new(sample_rate),
            phase: 0.0,
            modulation_phase: 0.0,
            input_level: 0.0,
            output_level: 0.0,
        }
    }

    fn tick(&mut self, controls: AmpControls) -> (f32, f32) {
        const SAMPLE_RATE: f32 = 48_000.0;
        const BLOCK: usize = 768;

        let mut input_energy = 0.0;
        let mut output_energy = 0.0;

        for _ in 0..BLOCK {
            let input = self.next_probe_sample();
            let output = self.amp.process(input, controls);
            input_energy += input * input;
            output_energy += output * output;
        }

        let input_rms = (input_energy / BLOCK as f32).sqrt();
        let output_rms = (output_energy / BLOCK as f32).sqrt();
        self.input_level = smooth_meter(self.input_level, rms_to_meter(input_rms));
        self.output_level = smooth_meter(self.output_level, rms_to_meter(output_rms));

        self.phase = self.phase.rem_euclid(std::f32::consts::TAU);
        self.modulation_phase = (self.modulation_phase
            + std::f32::consts::TAU * 1.1 * BLOCK as f32 / SAMPLE_RATE)
            .rem_euclid(std::f32::consts::TAU);

        (self.input_level, self.output_level)
    }

    fn next_probe_sample(&mut self) -> f32 {
        const SAMPLE_RATE: f32 = 48_000.0;

        let envelope = 0.42 + self.modulation_phase.sin().max(0.0) * 0.58;
        let fundamental = self.phase.sin();
        let harmonic = (self.phase * 2.01).sin() * 0.18;
        let pick_grain = (self.phase * 5.97).sin() * 0.035;
        let sample = (fundamental + harmonic + pick_grain) * envelope * 0.075;

        self.phase += std::f32::consts::TAU * 110.0 / SAMPLE_RATE;
        self.modulation_phase += std::f32::consts::TAU * 1.1 / SAMPLE_RATE;

        sample
    }
}

fn amp_controls(ui: &GreyboundUi) -> AmpControls {
    let device = &ui.amp;
    AmpControls {
        volume: device.gain,
        bass: device.bass,
        cut: device.cut,
        treble: device.treble,
        output: 0.58,
        drive: 0.0,
        presence: 0.0,
        sag: device.master,
    }
}

fn rms_to_meter(rms: f32) -> f32 {
    let db = 20.0 * rms.max(0.000_001).log10();
    ((db + 54.0) / 54.0).clamp(0.0, 1.0)
}

fn smooth_meter(previous: f32, target: f32) -> f32 {
    let coefficient = if target > previous { 0.62 } else { 0.10 };
    previous + (target - previous) * coefficient
}
