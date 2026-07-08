use greybound_ui::{DESIGN_HEIGHT, DESIGN_WIDTH};

const ASPECT_RATIO: f32 = DESIGN_WIDTH / DESIGN_HEIGHT;
const RESIZE_TOLERANCE_PX: u32 = 2;

pub(crate) fn aspect_corrected_size(width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }

    let current_ratio = width as f32 / height as f32;
    if (current_ratio - ASPECT_RATIO).abs() < 0.003 {
        return None;
    }

    let width_from_height = (height as f32 * ASPECT_RATIO).round() as u32;
    let height_from_width = (width as f32 / ASPECT_RATIO).round() as u32;

    let width_delta = width.abs_diff(width_from_height);
    let height_delta = height.abs_diff(height_from_width);
    let (target_width, target_height) = if width_delta <= height_delta {
        (width_from_height, height)
    } else {
        (width, height_from_width)
    };

    if width.abs_diff(target_width) <= RESIZE_TOLERANCE_PX
        && height.abs_diff(target_height) <= RESIZE_TOLERANCE_PX
    {
        None
    } else {
        Some((target_width.max(1), target_height.max(1)))
    }
}
