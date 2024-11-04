use std::ops::RangeInclusive;

use egui::{Align2, FontId, Pos2, Rect, Stroke, Ui};

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
    pub tick_interval_base: u8,
}

impl Default for AxisDrawer {
    fn default() -> Self {
        Self {
            x_axis_height: 40.0,
            y_axis_width: 40.0,
            x_label: None,
            y_label: None,
            label_font: None,
            tick_length: 5.0,
            tick_label_font: None,
            stroke: None,
            x_range: 0.0f32..=100.0,
            y_range: 0.0f32..=100.0,
            tick_interval_base: 10,
        }
    }
}

impl AxisDrawer {
    pub fn get_remained_rect(&self, rect: Rect) -> (Rect, Rect, Rect) {
        let Self {
            x_axis_height,
            y_axis_width,
            ..
        } = self;

        let x_axis_height = *x_axis_height;
        let y_axis_width = *y_axis_width;

        let min = rect.left_top();
        let max = rect.right_bottom();
        let x_axis_rect = Rect::from_two_pos(
            Pos2::new(min.x + y_axis_width, max.y - x_axis_height),
            Pos2::new(max.x, max.y),
        );
        let y_axis_rect = Rect::from_two_pos(
            Pos2::new(min.x, min.y),
            Pos2::new(min.x + y_axis_width, max.y - x_axis_height),
        );

        // 计算剩余的 Rect
        let remaining_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + y_axis_width, rect.min.y),
            Pos2::new(rect.max.x, rect.max.y - x_axis_height),
        );

        // 返回剩余的 Rect
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

        // 根据 ui 的样式设置颜色
        let visuals = ui.visuals();
        let axis_color = visuals.text_color();
        let axis_stroke = stroke.unwrap_or_else(|| Stroke::new(1.0, visuals.text_color()));

        ui.painter().line_segment(
            [x_axis_rect.left_top(), x_axis_rect.right_top()],
            axis_stroke,
        );

        // 绘制 Y 轴

        ui.painter().line_segment(
            [y_axis_rect.right_bottom(), y_axis_rect.right_top()],
            axis_stroke,
        );

        // 绘制 X 轴刻度和刻度标签
        const TICK_LABEL_HEIGHT: f32 = 5.0;

        for x in Self::calculate_tick_pos(x_range, base) {
            let x_pos = (x - self.x_range.start()) / (self.x_range.end() - self.x_range.start())
                * x_axis_rect.width()
                + x_axis_rect.left();
            let tick_start = Pos2::new(x_pos, x_axis_rect.top());
            let tick_end = Pos2::new(x_pos, x_axis_rect.top() + tick_length);
            ui.painter()
                .line_segment([tick_start, tick_end], axis_stroke);

            // 绘制刻度标签
            let tick_label_pos =
                Pos2::new(x_pos, x_axis_rect.top() + tick_length + TICK_LABEL_HEIGHT);
            ui.painter().text(
                tick_label_pos,
                Align2::CENTER_TOP,
                format!("{}", x),
                tick_label_font.clone(),
                axis_color,
            );
        }

        const TICK_LABEL_WIDTH: f32 = 5.0;
        // 绘制 Y 轴刻度和刻度标签

        for y in Self::calculate_tick_pos(y_range, base) {
            let y_pos = y_axis_rect.bottom()
                - (y - self.y_range.start()) / (self.y_range.end() - self.y_range.start())
                    * y_axis_rect.height();
            let tick_start = Pos2::new(y_axis_rect.right(), y_pos);
            let tick_end = Pos2::new(y_axis_rect.right() - tick_length, y_pos);
            ui.painter()
                .line_segment([tick_start, tick_end], axis_stroke);

            // 绘制刻度标签
            let tick_label_pos = Pos2::new(y_axis_rect.right() - TICK_LABEL_WIDTH, y_pos);
            ui.painter().text(
                tick_label_pos,
                Align2::RIGHT_CENTER,
                format!("{}", y),
                tick_label_font.clone(),
                axis_color,
            );
        }
        if let Some(x_label) = x_label {
            // 绘制 X 轴标签
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
            // 绘制 Y 轴标签
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
        let start_ind = (range.start() / interval).ceil() as u32;
        (start_ind..)
            .map(move |i| i as f32 * interval)
            .take_while(move |&x| x <= *range.end())
    }
}
