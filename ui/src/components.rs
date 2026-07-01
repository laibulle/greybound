use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point};

#[derive(Debug, Clone, Copy)]
pub struct KnobSpec<'a> {
    pub label: &'a str,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub skin: KnobSkin,
}

impl<'a> KnobSpec<'a> {
    pub fn normalized(label: &'a str, value: f32) -> Self {
        Self {
            label,
            value,
            min: 0.0,
            max: 1.0,
            step: 0.0,
            skin: KnobSkin::AsatoBlack,
        }
    }

    pub fn display_value(self) -> f32 {
        let span = self.max - self.min;
        self.min + self.value.clamp(0.0, 1.0) * span
    }

    pub fn quantize(self, value: f32) -> f32 {
        let value = value.clamp(0.0, 1.0);
        if self.step <= 0.0 || self.max <= self.min {
            return value;
        }

        let display = self.min + value * (self.max - self.min);
        let stepped = ((display - self.min) / self.step).round() * self.step + self.min;
        ((stepped - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum KnobSkin {
    AsatoBlack,
    Teal,
}

pub fn draw_knob(frame: &mut Frame, center: Point, radius: f32, spec: KnobSpec<'_>) {
    let value = spec.quantize(spec.value);
    draw_ticks(frame, center, radius);
    draw_shadow(frame, center, radius);

    match spec.skin {
        KnobSkin::AsatoBlack => draw_asato_cap(frame, center, radius, value),
        KnobSkin::Teal => draw_teal_cap(frame, center, radius, value),
    }

    if !spec.label.is_empty() {
        draw_text(
            frame,
            spec.label,
            Point::new(center.x, center.y + radius + 30.0),
            14.0,
            Color::from_rgb(0.09, 0.08, 0.08),
            Horizontal::Center,
        );
    }
}

fn draw_ticks(frame: &mut Frame, center: Point, radius: f32) {
    for tick in 0..29 {
        let t = tick as f32 / 28.0;
        let angle = (-135.0 + t * 270.0).to_radians();
        let inner = Point::new(
            center.x + angle.cos() * (radius + 8.0),
            center.y + angle.sin() * (radius + 8.0),
        );
        let outer = Point::new(
            center.x + angle.cos() * (radius + 17.0),
            center.y + angle.sin() * (radius + 17.0),
        );
        frame.stroke(
            &Path::line(inner, outer),
            Stroke::default()
                .with_color(Color::from_rgba(0.08, 0.08, 0.09, 0.58))
                .with_width(if tick % 4 == 0 { 1.8 } else { 1.0 }),
        );
    }
}

fn draw_shadow(frame: &mut Frame, center: Point, radius: f32) {
    frame.fill(
        &Path::circle(Point::new(center.x + 7.0, center.y + 10.0), radius + 6.0),
        Color::from_rgba(0.04, 0.03, 0.03, 0.28),
    );
}

fn draw_asato_cap(frame: &mut Frame, center: Point, radius: f32, value: f32) {
    frame.fill(
        &Path::circle(center, radius),
        Color::from_rgb(0.13, 0.09, 0.09),
    );
    frame.stroke(
        &Path::circle(center, radius),
        Stroke::default()
            .with_color(Color::from_rgba(0.92, 0.76, 0.72, 0.42))
            .with_width(2.0),
    );

    for groove in 0..10 {
        let r = radius - groove as f32 * 2.1;
        frame.stroke(
            &Path::circle(center, r),
            Stroke::default()
                .with_color(Color::from_rgba(0.46, 0.34, 0.34, 0.10))
                .with_width(1.0),
        );
    }

    for lobe in 0..10 {
        let angle = lobe as f32 / 10.0 * std::f32::consts::TAU;
        let p = Point::new(
            center.x + angle.cos() * radius * 0.78,
            center.y + angle.sin() * radius * 0.78,
        );
        frame.fill(
            &Path::circle(p, radius * 0.18),
            Color::from_rgba(0.28, 0.20, 0.20, 0.44),
        );
    }

    let glint = Path::circle(
        Point::new(center.x - radius * 0.28, center.y - radius * 0.30),
        radius * 0.40,
    );
    frame.fill(&glint, Color::from_rgba(1.0, 0.80, 0.78, 0.10));
    draw_pointer(
        frame,
        center,
        radius,
        value,
        Color::from_rgb(0.93, 0.70, 0.34),
        4.0,
    );
}

fn draw_teal_cap(frame: &mut Frame, center: Point, radius: f32, value: f32) {
    frame.fill(
        &Path::circle(center, radius),
        Color::from_rgb(0.31, 0.50, 0.51),
    );
    for tooth in 0..28 {
        let angle = tooth as f32 / 28.0 * std::f32::consts::TAU;
        let a = Point::new(
            center.x + angle.cos() * (radius - 2.0),
            center.y + angle.sin() * (radius - 2.0),
        );
        let b = Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        frame.stroke(
            &Path::line(a, b),
            Stroke::default()
                .with_color(Color::from_rgba(0.03, 0.08, 0.08, 0.36))
                .with_width(2.0),
        );
    }
    frame.fill(
        &Path::circle(
            Point::new(center.x - radius * 0.22, center.y - radius * 0.22),
            radius * 0.42,
        ),
        Color::from_rgba(0.86, 0.98, 0.94, 0.13),
    );
    draw_pointer(
        frame,
        center,
        radius,
        value,
        Color::from_rgb(0.92, 0.88, 0.80),
        5.0,
    );
}

fn draw_pointer(
    frame: &mut Frame,
    center: Point,
    radius: f32,
    value: f32,
    color: Color,
    width: f32,
) {
    let angle = (-130.0 + value.clamp(0.0, 1.0) * 260.0).to_radians();
    let pointer = Path::line(
        Point::new(
            center.x + angle.cos() * radius * 0.18,
            center.y + angle.sin() * radius * 0.18,
        ),
        Point::new(
            center.x + angle.cos() * radius * 0.76,
            center.y + angle.sin() * radius * 0.76,
        ),
    );
    frame.stroke(
        &pointer,
        Stroke::default().with_color(color).with_width(width),
    );
}

fn draw_text(
    frame: &mut Frame,
    content: &str,
    position: Point,
    size: f32,
    color: Color,
    align: Horizontal,
) {
    frame.fill_text(Text {
        content: content.to_string(),
        position,
        color,
        size,
        horizontal_alignment: align,
        vertical_alignment: Vertical::Center,
        ..Text::default()
    });
}
