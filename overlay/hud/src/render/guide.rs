use super::*;

/// Filled chevron. Angle 0 points up (bike-forward); positive points bike-right.
pub(crate) fn paint_guide_chevron(
    px: &mut Pixmap,
    center_x: f32,
    center_y: f32,
    angle: f32,
    height: f32,
    fill: Color,
) {
    let Some(path) = guide_chevron_path(center_x, center_y, angle, height) else {
        return;
    };
    fill_path(px, &path, fill);
}

fn guide_chevron_path(center_x: f32, center_y: f32, angle: f32, height: f32) -> Option<Path> {
    let forward_x = angle.sin();
    let forward_y = -angle.cos();
    let side_x = -forward_y;
    let side_y = forward_x;
    let width = height * 0.62;
    let tip_x = center_x + forward_x * height * 0.48;
    let tip_y = center_y + forward_y * height * 0.48;
    let base_x = center_x - forward_x * height * 0.42;
    let base_y = center_y - forward_y * height * 0.42;
    let notch_x = base_x + forward_x * height * 0.16;
    let notch_y = base_y + forward_y * height * 0.16;
    let mut path = PathBuilder::new();
    path.move_to(tip_x, tip_y);
    path.line_to(base_x + side_x * width, base_y + side_y * width);
    path.line_to(notch_x, notch_y);
    path.line_to(base_x - side_x * width, base_y - side_y * width);
    path.close();
    path.finish()
}

/// The Guide widget: the same chevron, filling its rect. Hidden unless the latch is up.
pub(crate) fn draw_guide(px: &mut Pixmap, s: &Snapshot, cfg: &HudConfig, sw: f32, sh: f32) {
    if !cfg.guide_widget_visible() {
        return;
    }
    let Some(bearing) = crate::track_guide::visible(s) else {
        return;
    };
    let slot = cfg[WidgetId::Guide].rect;
    let x = slot.x * sw;
    let y = slot.y * sh;
    let w = (slot.w * sw).max(1.0);
    let h = (slot.h * sh).max(1.0);
    let height = w.min(h) * 0.72;
    paint_guide_chevron(
        px,
        x + w * 0.5,
        y + h * 0.5,
        bearing.angle,
        height,
        accent(),
    );
}
