use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point};

const KNOB_MIN_ANGLE_DEGREES: f32 = 135.0;
const KNOB_MAX_ANGLE_DEGREES: f32 = 405.0;

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
    HeaderDial,
    Teal,
}

pub fn draw_knob(frame: &mut Frame, center: Point, radius: f32, spec: KnobSpec<'_>) {
    let value = spec.quantize(spec.value);
    draw_ticks(frame, center, radius, spec.skin);
    draw_shadow(frame, center, radius);

    match spec.skin {
        KnobSkin::AsatoBlack => draw_asato_cap(frame, center, radius, value),
        KnobSkin::HeaderDial => draw_header_dial(frame, center, radius, value),
        KnobSkin::Teal => draw_pedal_teal_cap(frame, center, radius, value),
    }

    if !spec.label.is_empty() {
        draw_text(
            frame,
            spec.label,
            Point::new(center.x, center.y + radius + 26.0),
            14.0,
            Color::from_rgb(0.09, 0.08, 0.08),
            Horizontal::Center,
        );
    }
}

pub fn draw_vertical_meter(frame: &mut Frame, top: Point, height: f32, level: f32) {
    let track_width = 12.0;
    let fill_height = (height * level.clamp(0.0, 1.0)).max(3.0);
    let track = Path::rectangle(top, iced::Size::new(track_width, height));
    frame.fill(&track, Color::from_rgba(0.44, 0.52, 0.72, 0.28));

    let fill_top = Point::new(top.x + 2.0, top.y + height - fill_height);
    let fill = Path::rectangle(fill_top, iced::Size::new(track_width - 4.0, fill_height));
    frame.fill(&fill, Color::from_rgb(0.08, 0.13, 0.28));
}

fn draw_ticks(frame: &mut Frame, center: Point, radius: f32, skin: KnobSkin) {
    let is_header = matches!(skin, KnobSkin::HeaderDial);
    let is_pedal = matches!(skin, KnobSkin::AsatoBlack | KnobSkin::Teal);

    for tick in 0..29 {
        let is_major = tick % 4 == 0;
        let t = tick as f32 / 28.0;
        let angle = knob_angle(t);
        let inner_offset = if is_pedal { radius + 6.0 } else { radius + 8.0 };
        let outer_offset = if is_pedal && !is_major {
            radius + 10.0
        } else if is_pedal {
            radius + 14.0
        } else if is_header && !is_major {
            radius + 13.0
        } else {
            radius + 17.0
        };
        let alpha = if is_header && !is_major { 0.44 } else { 0.58 };
        let width = if is_major { 1.8 } else { 1.0 };
        let inner = Point::new(
            center.x + angle.cos() * inner_offset,
            center.y + angle.sin() * inner_offset,
        );
        let outer = Point::new(
            center.x + angle.cos() * outer_offset,
            center.y + angle.sin() * outer_offset,
        );
        frame.stroke(
            &Path::line(inner, outer),
            Stroke::default()
                .with_color(Color::from_rgba(0.08, 0.08, 0.09, alpha))
                .with_width(width),
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
    let rim_outer = Path::circle(Point::new(center.x + 1.5, center.y + 2.5), radius + 5.0);
    frame.fill(&rim_outer, Color::from_rgb(0.10, 0.08, 0.085));
    frame.stroke(
        &rim_outer,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.78, 0.74, 0.36))
            .with_width(2.0),
    );

    let rim_inner = Path::circle(Point::new(center.x - 1.0, center.y - 1.0), radius + 1.0);
    frame.stroke(
        &rim_inner,
        Stroke::default()
            .with_color(Color::from_rgba(0.02, 0.015, 0.018, 0.78))
            .with_width(5.0),
    );

    let cap_shadow = scalloped_knob_path(Point::new(center.x + 2.5, center.y + 3.5), radius * 0.91);
    frame.fill(&cap_shadow, Color::from_rgba(0.02, 0.014, 0.016, 0.58));

    let cap = scalloped_knob_path(center, radius * 0.90);
    frame.fill(&cap, Color::from_rgb(0.12, 0.075, 0.075));
    frame.stroke(
        &cap,
        Stroke::default()
            .with_color(Color::from_rgba(0.72, 0.40, 0.40, 0.54))
            .with_width(2.0),
    );

    let inner_cap = scalloped_knob_path(Point::new(center.x - 1.5, center.y - 1.5), radius * 0.76);
    frame.fill(&inner_cap, Color::from_rgba(0.22, 0.15, 0.15, 0.20));

    for groove in 0..7 {
        let r = radius * (0.32 + groove as f32 * 0.07);
        frame.stroke(
            &scalloped_knob_path(center, r),
            Stroke::default()
                .with_color(Color::from_rgba(0.55, 0.40, 0.38, 0.08))
                .with_width(1.0),
        );
    }

    for grain in 0..18 {
        let y = center.y - radius * 0.48 + grain as f32 * radius * 0.052;
        let x0 = center.x - radius * 0.42 + (grain as f32 * 1.7).sin() * 4.0;
        let x1 = center.x + radius * 0.38 + (grain as f32 * 2.1).cos() * 5.0;
        frame.stroke(
            &Path::line(Point::new(x0, y), Point::new(x1, y + 2.0)),
            Stroke::default()
                .with_color(Color::from_rgba(0.50, 0.35, 0.32, 0.055))
                .with_width(1.0),
        );
    }

    frame.stroke(
        &arc_path(
            center,
            radius * 0.95,
            128.0_f32.to_radians(),
            212.0_f32.to_radians(),
            24,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.58, 0.92, 0.96, 0.58))
            .with_width(4.0),
    );
    frame.stroke(
        &arc_path(
            center,
            radius * 0.83,
            130.0_f32.to_radians(),
            185.0_f32.to_radians(),
            18,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.84, 0.80, 0.42))
            .with_width(2.0),
    );
    frame.stroke(
        &arc_path(
            center,
            radius * 0.90,
            300.0_f32.to_radians(),
            380.0_f32.to_radians(),
            22,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.02, 0.015, 0.018, 0.46))
            .with_width(4.0),
    );

    draw_pointer(
        frame,
        Point::new(center.x + 1.2, center.y + 1.4),
        radius * 0.90,
        value,
        Color::from_rgba(0.12, 0.07, 0.04, 0.46),
        1.5,
    );
    draw_pointer(
        frame,
        center,
        radius * 0.92,
        value,
        Color::from_rgb(0.93, 0.70, 0.34),
        4.0,
    );
}

fn draw_pedal_teal_cap(frame: &mut Frame, center: Point, radius: f32, value: f32) {
    draw_text(
        frame,
        "min",
        Point::new(center.x - radius * 1.58, center.y + radius * 1.08),
        9.0,
        Color::from_rgba(0.09, 0.08, 0.08, 0.72),
        Horizontal::Center,
    );
    draw_text(
        frame,
        "max",
        Point::new(center.x + radius * 1.58, center.y + radius * 1.08),
        9.0,
        Color::from_rgba(0.09, 0.08, 0.08, 0.72),
        Horizontal::Center,
    );

    frame.fill(
        &Path::circle(
            Point::new(center.x + radius * 0.16, center.y + radius * 0.22),
            radius * 1.02,
        ),
        Color::from_rgba(0.10, 0.09, 0.07, 0.24),
    );
    frame.fill(
        &Path::circle(
            Point::new(center.x, center.y + radius * 0.13),
            radius * 0.98,
        ),
        Color::from_rgb(0.25, 0.42, 0.41),
    );

    for tooth in 0..34 {
        let angle = tooth as f32 / 34.0 * std::f32::consts::TAU;
        let start = Point::new(
            center.x + angle.cos() * radius * 0.74,
            center.y + radius * 0.10 + angle.sin() * radius * 0.74,
        );
        let end = Point::new(
            center.x + angle.cos() * radius * 0.96,
            center.y + radius * 0.10 + angle.sin() * radius * 0.96,
        );
        frame.stroke(
            &Path::line(start, end),
            Stroke::default()
                .with_color(Color::from_rgba(0.07, 0.17, 0.17, 0.46))
                .with_width(3.0),
        );
    }

    frame.fill(
        &Path::circle(
            Point::new(center.x, center.y - radius * 0.08),
            radius * 0.82,
        ),
        Color::from_rgb(0.47, 0.67, 0.66),
    );
    frame.fill(
        &Path::circle(
            Point::new(center.x - radius * 0.22, center.y - radius * 0.30),
            radius * 0.36,
        ),
        Color::from_rgba(0.90, 1.0, 0.96, 0.16),
    );
    frame.stroke(
        &Path::circle(
            Point::new(center.x, center.y - radius * 0.08),
            radius * 0.82,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.84, 0.96, 0.92, 0.32))
            .with_width(2.0),
    );

    draw_pointer(
        frame,
        Point::new(center.x + radius * 0.03, center.y - radius * 0.08),
        radius * 0.76,
        value,
        Color::from_rgba(0.18, 0.24, 0.22, 0.20),
        2.0,
    );
    draw_pointer(
        frame,
        Point::new(center.x + radius * 0.03, center.y - radius * 0.08),
        radius * 0.76,
        value,
        Color::from_rgb(0.88, 0.86, 0.78),
        6.0,
    );
}

fn draw_header_dial(frame: &mut Frame, center: Point, radius: f32, value: f32) {
    frame.stroke(
        &arc_path(
            center,
            radius + 6.0,
            KNOB_MIN_ANGLE_DEGREES.to_radians(),
            KNOB_MAX_ANGLE_DEGREES.to_radians(),
            48,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.47, 0.55, 0.72, 0.36))
            .with_width(5.0),
    );

    let active_end = knob_angle(value);
    frame.stroke(
        &arc_path(
            center,
            radius + 6.0,
            KNOB_MIN_ANGLE_DEGREES.to_radians(),
            active_end,
            36,
        ),
        Stroke::default()
            .with_color(Color::from_rgb(0.10, 0.14, 0.29))
            .with_width(4.0),
    );

    frame.fill(
        &Path::circle(Point::new(center.x + 2.0, center.y + 3.0), radius),
        Color::from_rgba(0.50, 0.58, 0.76, 0.10),
    );
    frame.fill(
        &Path::circle(center, radius),
        Color::from_rgb(0.88, 0.91, 1.0),
    );

    let angle = knob_angle(value);
    frame.stroke(
        &Path::line(
            Point::new(center.x, center.y),
            Point::new(
                center.x + angle.cos() * radius * 0.74,
                center.y + angle.sin() * radius * 0.74,
            ),
        ),
        Stroke::default()
            .with_color(Color::from_rgb(0.10, 0.14, 0.29))
            .with_width(3.0),
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
    let angle = knob_angle(value);
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

fn knob_angle(value: f32) -> f32 {
    (KNOB_MIN_ANGLE_DEGREES
        + value.clamp(0.0, 1.0) * (KNOB_MAX_ANGLE_DEGREES - KNOB_MIN_ANGLE_DEGREES))
        .to_radians()
}

fn scalloped_knob_path(center: Point, radius: f32) -> Path {
    Path::new(|path| {
        let steps = 96;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let angle = t * std::f32::consts::TAU;
            let scallop = 1.0 + 0.070 * (angle * 10.0).sin() + 0.018 * (angle * 20.0 + 0.8).sin();
            let r = radius * scallop;
            let point = Point::new(center.x + angle.cos() * r, center.y + angle.sin() * r);

            if step == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        path.close();
    })
}

fn arc_path(center: Point, radius: f32, start: f32, end: f32, steps: usize) -> Path {
    Path::new(|path| {
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let angle = start + (end - start) * t;
            let point = Point::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            );

            if step == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
    })
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
