use std::ops::RangeInclusive;

use egui::{Align2, FontId, Pos2, Rect, Stroke, Ui};

use crate::drawer::colormap::Y_AXIS_MIN_WIDTH;

#[derive(Debug, Clone)]
pub struct AxisDrawer {
    pub x_axis_height: f32,
    pub y_axis_width: f32,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub label_font: Option<FontId>,
    pub tick_length: f32,
    pub tick_label_font: Option<FontId>,
    pub stroke: Option<Stroke>,
    pub x_range: RangeInclusive<f32>,
    pub y_range: RangeInclusive<f32>,
    pub y_tick_shift: i32,
    pub tick_interval_base: u8,
    pub align_x_axis: Option<(f32, f32)>,
}

impl Default for AxisDrawer {
    fn default() -> Self {
        Self {
            x_axis_height: 40.0,
            y_axis_width: Y_AXIS_MIN_WIDTH,
            x_label: None,
            y_label: None,
            label_font: None,
            tick_length: 5.0,
            tick_label_font: None,
            stroke: None,
            x_range: 0.0f32..=100.0,
            y_range: 0.0f32..=100.0,
            y_tick_shift: 0,
            tick_interval_base: 10,
            align_x_axis: None,
        }
    }
}

impl AxisDrawer {
    pub fn get_remained_rect(&self, rect: Rect) -> (Rect, Rect, Rect) {
        let Self {
            x_axis_height,
            y_axis_width,
            align_x_axis,
            ..
        } = self;
        const THRES: f32 = 80.;
        let align_x_axis = match align_x_axis {
            Some((x1, x2)) if x1 - rect.left() < THRES && rect.right() - x2 < THRES => {
                Some((x1, x2))
            }
            _ => None,
        };

        let x_axis_height = *x_axis_height;
        let y_axis_width = align_x_axis
            .map(|x| x.0 - rect.left())
            .unwrap_or(*y_axis_width);

        let min = rect.left_top();
        let mut max = rect.right_bottom();
        if let Some(x) = align_x_axis {
            max.x = *x.1;
        }

        let x_axis_rect = Rect::from_two_pos(
            Pos2::new(min.x + y_axis_width, max.y - x_axis_height),
            Pos2::new(max.x, max.y),
        );
        let y_axis_rect = Rect::from_two_pos(
            Pos2::new(min.x, min.y),
            Pos2::new(min.x + y_axis_width, max.y - x_axis_height),
        );

        // Compute the remaining rect
        let remaining_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + y_axis_width, rect.min.y),
            Pos2::new(rect.max.x, rect.max.y - x_axis_height),
        );

        // Return the remaining rect
        (remaining_rect, x_axis_rect, y_axis_rect)
    }

    pub fn draw_axes_with_labels_and_ticks(&self, ui: &mut Ui, rect: Rect) {
        let Self {
            x_label,
            y_label,
            label_font,
            tick_length,
            tick_label_font,
            stroke,
            x_range,
            y_range,
            y_tick_shift,
            tick_interval_base: base,
            ..
        } = self;

        let y_range = y_range.clone();
        let x_range = x_range.clone();
        let base = *base;

        let (_, x_axis_rect, y_axis_rect) = self.get_remained_rect(rect);

        let tick_length = *tick_length;

        let style = ui.style();
        let text_style = &style.text_styles;
        use egui::TextStyle;

        let label_font = label_font
            .clone()
            .unwrap_or_else(|| text_style[&TextStyle::Body].clone());

        let tick_label_font = tick_label_font
            .clone()
            .unwrap_or_else(|| text_style[&TextStyle::Body].clone());

        // Pick colors based on the UI style
        let visuals = ui.visuals();
        let axis_color = visuals.text_color();
        let axis_stroke = stroke.unwrap_or_else(|| Stroke::new(1.0, visuals.text_color()));

        ui.painter().line_segment(
            [x_axis_rect.left_top(), x_axis_rect.right_top()],
            axis_stroke,
        );

        // Draw Y axis

        ui.painter().line_segment(
            [y_axis_rect.right_bottom(), y_axis_rect.right_top()],
            axis_stroke,
        );

        // Draw X-axis ticks and labels
        const MIN_X_TICK_LABEL_HEIGHT: f32 = 5.0;

        for x in Self::calculate_tick_pos(x_range, base) {
            let x_pos = (x - self.x_range.start()) / (self.x_range.end() - self.x_range.start())
                * x_axis_rect.width()
                + x_axis_rect.left();
            let tick_start = Pos2::new(x_pos, x_axis_rect.top());
            let tick_end = Pos2::new(x_pos, x_axis_rect.top() + tick_length);
            ui.painter()
                .line_segment([tick_start, tick_end], axis_stroke);

            // Draw tick label
            let tick_label_pos = Pos2::new(
                x_pos,
                x_axis_rect.top() + (tick_length + 1.0).max(MIN_X_TICK_LABEL_HEIGHT),
            );
            ui.painter().text(
                tick_label_pos,
                Align2::CENTER_TOP,
                format!("{x}"),
                tick_label_font.clone(),
                axis_color,
            );
        }

        const MIN_Y_TICK_LABEL_WIDTH: f32 = 5.0;
        // Draw Y-axis ticks and labels in shifted-label space.
        for y in Self::calculate_tick_pos_shifted(y_range, base, *y_tick_shift as f32) {
            let y_pos = y_axis_rect.bottom()
                - (y - self.y_range.start()) / (self.y_range.end() - self.y_range.start())
                    * y_axis_rect.height();
            let tick_start = Pos2::new(y_axis_rect.right(), y_pos);
            let tick_end = Pos2::new(y_axis_rect.right() - tick_length, y_pos);
            ui.painter()
                .line_segment([tick_start, tick_end], axis_stroke);

            // Draw tick label
            let tick_label_pos = Pos2::new(
                y_axis_rect.right() - (tick_length + 1.0).max(MIN_Y_TICK_LABEL_WIDTH),
                y_pos,
            );
            ui.painter().text(
                tick_label_pos,
                Align2::RIGHT_CENTER,
                format!("{}", y + *y_tick_shift as f32),
                tick_label_font.clone(),
                axis_color,
            );
        }
        if let Some(x_label) = x_label {
            // Draw X-axis label
            let x_label_pos = x_axis_rect.center_bottom();
            ui.painter().text(
                x_label_pos,
                Align2::CENTER_BOTTOM,
                x_label,
                label_font.clone(),
                axis_color,
            );
        }

        if let Some(y_label) = y_label {
            // Draw Y-axis label
            let y_label_pos = y_axis_rect.left_center();
            ui.painter().text(
                y_label_pos,
                Align2::LEFT_CENTER,
                y_label,
                label_font.clone(),
                axis_color,
            );
        }
    }

    fn calculate_tick_pos(range: RangeInclusive<f32>, base: u8) -> impl Iterator<Item = f32> {
        let range_span = range.end() - range.start();
        let order = (range_span.log(base as f32) * 0.9).floor();
        let interval = (base as f32).powf(order);
        let start_ind = (range.start() / interval).ceil() as i32;
        std::iter::successors(Some(start_ind), |i| Some(i + 1))
            .map(move |i| i as f32 * interval)
            .take_while(move |&x| x <= *range.end())
    }

    fn calculate_tick_pos_shifted(
        range: RangeInclusive<f32>,
        base: u8,
        shift: f32,
    ) -> impl Iterator<Item = f32> {
        let shifted_start = *range.start() + shift;
        let shifted_end = *range.end() + shift;
        Self::calculate_tick_pos(shifted_start..=shifted_end, base).map(move |v| v - shift)
    }
}
