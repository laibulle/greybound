use anyhow::{bail, Context};
use greybound_ui::{
    AppProfile, ModelRenderSpec, RenderControlRole, RenderControlWidget, RenderSurfaceKind,
};
use image::GenericImageView;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const PEDAL_SOURCE_WIDTH: u32 = 1200;
const PEDAL_SOURCE_HEIGHT: u32 = 2172;
const PEDAL_BODY_LEFT: f32 = 72.0;
const PEDAL_BODY_TOP: f32 = 72.0;
const PEDAL_BODY_RIGHT: f32 = 1128.0;
const PEDAL_BODY_BOTTOM: f32 = 2100.0;
const LABEL_BASELINE_OFFSET: f32 = 177.0;
const LABEL_HEIGHT: f32 = 32.0;
const LABEL_SIDE_PADDING: f32 = 18.0;
const COMPONENT_CLEARANCE: f32 = 12.0;

fn main() -> nih_plug_xtask::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        anyhow::bail!("Usage: cargo xtask bundle <package> [cargo build args]");
    };

    match command.as_str() {
        "pedal-assets" => pedal_assets(args.collect()),
        "bundle" => {
            let package = args.next().ok_or_else(|| {
                anyhow::anyhow!("Usage: cargo xtask bundle <package> [cargo build args]")
            })?;
            let cargo_args = args.collect::<Vec<_>>();
            let packages = vec![package.clone()];
            let target_dir = std::env::current_dir()?.join("target");

            nih_plug_xtask::build(&packages, &cargo_args)?;
            nih_plug_xtask::bundle(&target_dir, &package, &cargo_args, false)
        }
        _ => bail!("Unknown command '{command}'. Usage: cargo xtask bundle <package> | cargo xtask pedal-assets <export|prompt|validate> ..."),
    }
}

fn pedal_assets(args: Vec<String>) -> anyhow::Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        bail!("Usage: cargo xtask pedal-assets <export <dir>|prompt <model-id>|validate <model-id> <png>|preview <model-id> <png> <out.png>>");
    };

    match command {
        "export" => {
            let output = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("docs/generated/pedal-asset-contracts"));
            export_pedal_contracts(&output)
        }
        "prompt" => {
            let model_id = args
                .get(1)
                .context("Usage: cargo xtask pedal-assets prompt <model-id>")?;
            let (_, render) = pedal_render_spec(model_id)?;
            print!("{}", faceplate_prompt(render));
            Ok(())
        }
        "validate" => {
            let model_id = args
                .get(1)
                .context("Usage: cargo xtask pedal-assets validate <model-id> <png>")?;
            let png = args
                .get(2)
                .context("Usage: cargo xtask pedal-assets validate <model-id> <png>")?;
            let (_, render) = pedal_render_spec(model_id)?;
            validate_faceplate(render, Path::new(png))?;
            println!("PASS: {model_id} faceplate contract: {png}");
            Ok(())
        }
        "preview" => {
            let model_id = args
                .get(1)
                .context("Usage: cargo xtask pedal-assets preview <model-id> <png> <out.png>")?;
            let png = args
                .get(2)
                .context("Usage: cargo xtask pedal-assets preview <model-id> <png> <out.png>")?;
            let output = args
                .get(3)
                .context("Usage: cargo xtask pedal-assets preview <model-id> <png> <out.png>")?;
            let (_, render) = pedal_render_spec(model_id)?;
            write_preview(render, Path::new(png), Path::new(output))?;
            println!("Wrote deterministic layout overlay: {output}");
            Ok(())
        }
        _ => {
            bail!("Unknown pedal-assets command '{command}'. Expected export, prompt, validate, or preview.")
        }
    }
}

fn pedal_specs() -> Vec<(&'static str, &'static ModelRenderSpec)> {
    let mut specs = Vec::new();
    let mut ids = BTreeSet::new();
    for profile in [AppProfile::greybound_free(), AppProfile::greybound_glass()] {
        for device in profile.devices {
            if device.render.surface.kind == RenderSurfaceKind::Pedal
                && device.render.generation.is_some()
                && ids.insert(device.id)
            {
                specs.push((device.id, device.render));
            }
        }
    }
    specs
}

fn pedal_render_spec(model_id: &str) -> anyhow::Result<(&'static str, &'static ModelRenderSpec)> {
    pedal_specs()
        .into_iter()
        .find(|(id, _)| *id == model_id)
        .with_context(|| format!("Unknown regenerable pedal '{model_id}'. Run `cargo xtask pedal-assets export` to see the catalogue."))
}

fn export_pedal_contracts(output: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output).with_context(|| format!("Could not create {}", output.display()))?;

    for (model_id, render) in pedal_specs() {
        validate_control_layout(render)?;
        let contract_path = output.join(format!("{model_id}.json"));
        let guide_path = output.join(format!("{model_id}.construction.svg"));
        fs::write(
            &contract_path,
            serde_json::to_string_pretty(&pedal_contract(model_id, render))? + "\n",
        )
        .with_context(|| format!("Could not write {}", contract_path.display()))?;
        fs::write(&guide_path, construction_guide(model_id, render))
            .with_context(|| format!("Could not write {}", guide_path.display()))?;
    }

    println!(
        "Exported {} pedal contracts to {}",
        pedal_specs().len(),
        output.display()
    );
    Ok(())
}

fn pedal_contract(model_id: &str, render: &ModelRenderSpec) -> serde_json::Value {
    let generation = render.generation.expect("filtered to regenerable pedals");
    let controls = render.controls.iter().map(|control| {
        let x = control.anchor_x * PEDAL_SOURCE_WIDTH as f32;
        let y = control.anchor_y * PEDAL_SOURCE_HEIGHT as f32;
        json!({
            "role": control_role(control.role),
            "widget": control_widget(control.widget),
            "label": control.label,
            "anchor": { "normalized": { "x": control.anchor_x, "y": control.anchor_y }, "source_px": { "x": x, "y": y } },
            "radius": { "logical_px": control.radius, "source_px": control.radius * 4.0 },
            "hit_radius_logical_px": control.hit_radius,
            "dynamic_asset": control.asset.map(control_asset_contract),
            "label_baseline_source_y": if matches!(control.role, RenderControlRole::Parameter(_)) { Some(label_baseline_source_y(control)) } else { None },
        })
    }).collect::<Vec<_>>();

    json!({
        "schema": "greybound.pedal-asset-contract/v1",
        "model_id": model_id,
        "render_id": render.id,
        "recipe_version": generation.recipe_version,
        "output": {
            "faceplate_path": generation.faceplate_path,
            "format": "PNG RGBA",
            "width": PEDAL_SOURCE_WIDTH,
            "height": PEDAL_SOURCE_HEIGHT,
            "transparent_background": true,
        },
        "faceplate": {
            "model_name": generation.model_name,
            "art_direction": generation.art_direction,
            "view": "straight-on orthographic front elevation",
            "body_bounds_source_px": { "x": PEDAL_BODY_LEFT, "y": PEDAL_BODY_TOP, "width": PEDAL_BODY_RIGHT - PEDAL_BODY_LEFT, "height": PEDAL_BODY_BOTTOM - PEDAL_BODY_TOP, "outer_radius": 72, "bevel": 24 },
            "must_not_contain": ["knobs", "knob holes", "control labels", "control ticks", "footswitch", "footswitch hole", "LED jewel", "LED socket", "toggle", "slider", "front screws", "front jacks", "drop shadow", "floor", "background"],
        },
        "deterministic_runtime_layer": {
            "labels": "Drawn by UI from this contract; never generated as pixels.",
            "jacks": "Standard input/output jacks are rendered by the UI at the fixed mechanical anchors.",
            "controls": "All interactive hardware is rendered from the declared control assets below.",
        },
        "preflight_preview": "cargo xtask pedal-assets preview <model-id> <candidate.png> <inspection.png>",
        "controls": controls,
        "image_model_prompt": faceplate_prompt(render),
    })
}

fn control_asset_contract(asset: greybound_ui::RenderControlAssetSpec) -> serde_json::Value {
    json!({
        "image": asset.image.path,
        "active_image": asset.active_image.map(|image| image.path),
        "pressed_image": asset.pressed_image.map(|image| image.path),
        "rotation": asset.rotation.map(|rotation| json!({
            "min_degrees": rotation.min_degrees,
            "max_degrees": rotation.max_degrees,
            "pivot": { "x": rotation.pivot_x, "y": rotation.pivot_y },
        })),
    })
}

fn control_role(role: RenderControlRole) -> &'static str {
    match role {
        RenderControlRole::Parameter(_) => "parameter",
        RenderControlRole::Bypass => "bypass",
    }
}

fn control_widget(widget: RenderControlWidget) -> &'static str {
    match widget {
        RenderControlWidget::Pot => "pot",
        RenderControlWidget::Slider => "slider",
        RenderControlWidget::Toggle => "toggle",
        RenderControlWidget::Footswitch => "footswitch",
        RenderControlWidget::Led => "led",
    }
}

fn faceplate_prompt(render: &ModelRenderSpec) -> String {
    let generation = render.generation.expect("filtered to regenerable pedals");
    format!(
        "Use case: product-mockup\nAsset type: Greybound pedal faceplate source art\n\
Create exactly one EMPTY boutique guitar-pedal enclosure faceplate.\n\n\
Canvas and camera:\n- exact canvas: {PEDAL_SOURCE_WIDTH} x {PEDAL_SOURCE_HEIGHT} px\n- straight-on orthographic front elevation; no perspective, no rotation\n- transparent background; no floor, environment, reflection, or shadow outside the enclosure\n- enclosure outer bounds: x=72..1128, y=72..2100; outer corner radius 72 px; visible bevel 24 px\n\n\
Identity:\n- model wordmark (the only permitted text): \"{}\"\n- art direction: {}\n- premium photorealistic metal/enamel construction, crisp 4x detail\n\n\
The application will place every functional component deterministically. Leave the entire face clean and uninterrupted.\n\n\
Absolutely do not render: knobs, knob caps, knob holes, dial marks, labels, footswitch, footswitch hole, LED, LED socket, switches, sliders, jacks, screws, washers, nuts, bezels, UI, background, floor, or any cast/contact shadow.\n\n\
Output only this empty enclosure as a PNG. The supplied construction SVG is a guide only and must not be visible in the output.\n",
        generation.model_name, generation.art_direction
    )
}

fn construction_guide(model_id: &str, render: &ModelRenderSpec) -> String {
    let mut controls = String::new();
    for control in render.controls {
        let x = control.anchor_x * PEDAL_SOURCE_WIDTH as f32;
        let y = control.anchor_y * PEDAL_SOURCE_HEIGHT as f32;
        let radius = control.radius * 4.0;
        let label = xml_escape(control.label);
        controls.push_str(&format!(
            "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{radius:.1}\" fill=\"#ff4fd8\" fill-opacity=\".12\" stroke=\"#ff4fd8\" stroke-width=\"4\" stroke-dasharray=\"16 12\"/><text x=\"{x:.1}\" y=\"{label_y:.1}\" fill=\"#ff4fd8\" font-family=\"Arial\" font-size=\"28\" text-anchor=\"middle\">{label}</text>\n",
            label_y = label_baseline_source_y(control),
        ));
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{PEDAL_SOURCE_WIDTH}\" height=\"{PEDAL_SOURCE_HEIGHT}\" viewBox=\"0 0 {PEDAL_SOURCE_WIDTH} {PEDAL_SOURCE_HEIGHT}\"><title>{}</title><rect width=\"{PEDAL_SOURCE_WIDTH}\" height=\"{PEDAL_SOURCE_HEIGHT}\" fill=\"none\" stroke=\"#64748b\" stroke-width=\"4\"/><rect x=\"72\" y=\"72\" width=\"1056\" height=\"2028\" rx=\"72\" fill=\"none\" stroke=\"#00a7ff\" stroke-width=\"6\" stroke-dasharray=\"24 18\"/><rect x=\"0\" y=\"1018\" width=\"72\" height=\"136\" fill=\"#ffb347\" fill-opacity=\".16\"/><rect x=\"1128\" y=\"1018\" width=\"72\" height=\"136\" fill=\"#ffb347\" fill-opacity=\".16\"/><text x=\"120\" y=\"1096\" fill=\"#ffb347\" font-family=\"Arial\" font-size=\"44\">IN</text><text x=\"1080\" y=\"1096\" fill=\"#ffb347\" font-family=\"Arial\" font-size=\"44\" text-anchor=\"end\">OUT</text>{controls}</svg>\n",
        xml_escape(model_id)
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn validate_faceplate(render: &ModelRenderSpec, path: &Path) -> anyhow::Result<()> {
    let image =
        image::open(path).with_context(|| format!("Could not decode {}", path.display()))?;
    if image.dimensions() != (PEDAL_SOURCE_WIDTH, PEDAL_SOURCE_HEIGHT) {
        bail!(
            "{} must be {PEDAL_SOURCE_WIDTH}x{PEDAL_SOURCE_HEIGHT}, got {}x{}",
            path.display(),
            image.width(),
            image.height()
        );
    }
    let rgba = image.to_rgba8();
    for (x, y) in [
        (0, 0),
        (PEDAL_SOURCE_WIDTH - 1, 0),
        (0, PEDAL_SOURCE_HEIGHT - 1),
        (PEDAL_SOURCE_WIDTH - 1, PEDAL_SOURCE_HEIGHT - 1),
    ] {
        if rgba.get_pixel(x, y)[3] != 0 {
            bail!("{} has opaque canvas corner at ({x}, {y}); faceplates require a transparent background", path.display());
        }
    }
    let alpha_bounds = alpha_bounds(&rgba).context("Faceplate has no visible enclosure pixels")?;
    if alpha_bounds.0 < PEDAL_BODY_LEFT as u32 - 4
        || alpha_bounds.1 < PEDAL_BODY_TOP as u32 - 4
        || alpha_bounds.2 > PEDAL_BODY_RIGHT as u32 + 4
        || alpha_bounds.3 > PEDAL_BODY_BOTTOM as u32 + 4
    {
        bail!("{} visible pixels {:?} exceed the standard enclosure bounds; do not include a background, floor, or external shadow", path.display(), alpha_bounds);
    }
    validate_control_layout(render)?;
    Ok(())
}

fn alpha_bounds(image: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        bounds = Some(match bounds {
            Some((left, top, right, bottom)) => {
                (left.min(x), top.min(y), right.max(x), bottom.max(y))
            }
            None => (x, y, x, y),
        });
    }
    bounds
}

fn validate_control_layout(render: &ModelRenderSpec) -> anyhow::Result<()> {
    let controls = render.controls;
    for control in render.controls {
        let x = control.anchor_x * PEDAL_SOURCE_WIDTH as f32;
        let y = control.anchor_y * PEDAL_SOURCE_HEIGHT as f32;
        let radius = control.radius * 4.0;
        if x - radius < PEDAL_BODY_LEFT
            || x + radius > PEDAL_BODY_RIGHT
            || y - radius < PEDAL_BODY_TOP
            || y + radius > PEDAL_BODY_BOTTOM
        {
            bail!(
                "{} control '{}' exceeds the pedal body",
                render.id,
                control.label
            );
        }
        if matches!(control.role, RenderControlRole::Parameter(_))
            && label_baseline_source_y(control) > PEDAL_BODY_BOTTOM - 16.0
        {
            bail!(
                "{} control '{}' leaves no room for its deterministic label",
                render.id,
                control.label
            );
        }

        for other in controls {
            if std::ptr::eq(control, other) {
                continue;
            }
            let other_x = other.anchor_x * PEDAL_SOURCE_WIDTH as f32;
            let other_y = other.anchor_y * PEDAL_SOURCE_HEIGHT as f32;
            let other_radius = other.radius * 4.0;
            let minimum_distance = radius + other_radius + COMPONENT_CLEARANCE;
            if distance(x, y, other_x, other_y) < minimum_distance {
                bail!(
                    "{} controls '{}' and '{}' overlap (need {:.1}px centre clearance)",
                    render.id,
                    control.label,
                    other.label,
                    minimum_distance
                );
            }
        }

        if matches!(control.role, RenderControlRole::Parameter(_)) {
            let label_y = label_baseline_source_y(control);
            let label_width = label_width(control.label);
            let label_left = x - label_width * 0.5;
            let label_right = x + label_width * 0.5;
            let label_top = label_y - LABEL_HEIGHT;
            if label_left < PEDAL_BODY_LEFT + COMPONENT_CLEARANCE
                || label_right > PEDAL_BODY_RIGHT - COMPONENT_CLEARANCE
                || label_top < PEDAL_BODY_TOP + COMPONENT_CLEARANCE
                || label_y > PEDAL_BODY_BOTTOM - COMPONENT_CLEARANCE
            {
                bail!(
                    "{} label '{}' exceeds the printable faceplate area",
                    render.id,
                    control.label
                );
            }
            for other in controls {
                let other_x = other.anchor_x * PEDAL_SOURCE_WIDTH as f32;
                let other_y = other.anchor_y * PEDAL_SOURCE_HEIGHT as f32;
                let other_radius = other.radius * 4.0 + COMPONENT_CLEARANCE;
                if circle_intersects_rect(
                    other_x,
                    other_y,
                    other_radius,
                    label_left,
                    label_top,
                    label_right,
                    label_y,
                ) {
                    bail!(
                        "{} label '{}' overlaps the '{}' component",
                        render.id,
                        control.label,
                        other.label
                    );
                }
            }
        }
    }
    Ok(())
}

fn label_width(label: &str) -> f32 {
    // The UI uses compact all-caps engraving; this conservative fixed advance
    // rejects a layout before it can become legible only by luck at a
    // particular zoom level.
    label.chars().count() as f32 * 30.0 + LABEL_SIDE_PADDING * 2.0
}

fn label_baseline_source_y(control: &greybound_ui::RenderControlSpec) -> f32 {
    let y = control.anchor_y * PEDAL_SOURCE_HEIGHT as f32;
    let standard_below = LABEL_BASELINE_OFFSET.max(control.radius * 4.0 + 80.0);
    match control.widget {
        // The Muffin-style vertical selectors live above the footswitch. Their
        // labels are deliberately placed above the travel area so the physical
        // switch can never cover the control name.
        RenderControlWidget::Slider => y - LABEL_BASELINE_OFFSET,
        _ => y + standard_below,
    }
}

fn distance(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (ax - bx).hypot(ay - by)
}

fn circle_intersects_rect(
    circle_x: f32,
    circle_y: f32,
    radius: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
) -> bool {
    let nearest_x = circle_x.clamp(left, right);
    let nearest_y = circle_y.clamp(top, bottom);
    distance(circle_x, circle_y, nearest_x, nearest_y) < radius
}

fn write_preview(render: &ModelRenderSpec, source: &Path, output: &Path) -> anyhow::Result<()> {
    validate_faceplate(render, source)?;
    let mut image = image::open(source)
        .with_context(|| format!("Could not decode {}", source.display()))?
        .to_rgba8();

    stroke_rect(
        &mut image,
        PEDAL_BODY_LEFT as i32,
        PEDAL_BODY_TOP as i32,
        PEDAL_BODY_RIGHT as i32,
        PEDAL_BODY_BOTTOM as i32,
        [0, 167, 255, 230],
        4,
    );
    stroke_rect(&mut image, 0, 1018, 72, 1154, [255, 179, 71, 230], 4);
    stroke_rect(&mut image, 1128, 1018, 1200, 1154, [255, 179, 71, 230], 4);

    for control in render.controls {
        let x = (control.anchor_x * PEDAL_SOURCE_WIDTH as f32).round() as i32;
        let y = (control.anchor_y * PEDAL_SOURCE_HEIGHT as f32).round() as i32;
        let radius = (control.radius * 4.0).round() as i32;
        stroke_circle(&mut image, x, y, radius, [255, 79, 216, 230], 4);
        if matches!(control.role, RenderControlRole::Parameter(_)) {
            let width = label_width(control.label).round() as i32;
            let baseline = label_baseline_source_y(control).round() as i32;
            stroke_rect(
                &mut image,
                x - width / 2,
                baseline - LABEL_HEIGHT.round() as i32,
                x + width / 2,
                baseline,
                [167, 139, 250, 230],
                3,
            );
        }
    }
    image
        .save(output)
        .with_context(|| format!("Could not write {}", output.display()))
}

fn stroke_rect(
    image: &mut image::RgbaImage,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: [u8; 4],
    width: i32,
) {
    for offset in 0..width {
        for x in left + offset..=right - offset {
            blend_pixel(image, x, top + offset, color);
            blend_pixel(image, x, bottom - offset, color);
        }
        for y in top + offset..=bottom - offset {
            blend_pixel(image, left + offset, y, color);
            blend_pixel(image, right - offset, y, color);
        }
    }
}

fn stroke_circle(
    image: &mut image::RgbaImage,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: [u8; 4],
    width: i32,
) {
    let outer = radius * radius;
    let inner_radius = (radius - width).max(0);
    let inner = inner_radius * inner_radius;
    for y in center_y - radius..=center_y + radius {
        for x in center_x - radius..=center_x + radius {
            let dx = x - center_x;
            let dy = y - center_y;
            let distance_squared = dx * dx + dy * dy;
            if distance_squared <= outer && distance_squared >= inner {
                blend_pixel(image, x, y, color);
            }
        }
    }
}

fn blend_pixel(image: &mut image::RgbaImage, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 {
        return;
    }
    let destination = image.get_pixel_mut(x as u32, y as u32);
    let source_alpha = color[3] as f32 / 255.0;
    let destination_alpha = destination[3] as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    for channel in 0..3 {
        let source = color[channel] as f32 / 255.0;
        let destination_color = destination[channel] as f32 / 255.0;
        let output = if output_alpha == 0.0 {
            0.0
        } else {
            (source * source_alpha + destination_color * destination_alpha * (1.0 - source_alpha))
                / output_alpha
        };
        destination[channel] = (output * 255.0).round() as u8;
    }
    destination[3] = (output_alpha * 255.0).round() as u8;
}
