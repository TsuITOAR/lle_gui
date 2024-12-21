pub(crate) fn show_dim(ui: &mut egui::Ui, dim: &mut usize) {
    ui.label("Dimension");
    let mut d_log = (*dim as f64).log(2.) as u32;
    ui.add(
        egui::DragValue::new(&mut d_log)
            .speed(0.1)
            .range(7..=15)
            .custom_parser(|s| {
                Some(
                    (s.parse::<u32>()
                        .map(|x| (x as f64).log(2.) as u32)
                        .unwrap_or(7)) as _,
                )
            })
            .custom_formatter(|v, _| format!("{}", 2u32.pow(v as u32)))
            .clamp_existing_to_range(true), //.suffix(format!("(2^{})", (*dim as f64).log(2.) as u32)),
    );
    *dim = 2u32.pow(d_log) as usize;
}

pub(crate) fn show_vector<V: ui_traits::ControllerUI + Default>(ui: &mut egui::Ui, v: &mut Vec<V>) {
    ui.vertical(|ui| {
        let mut to_remove = None;
        for (i, value) in v.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                value.show_controller(ui);
                ui.add_space(4.0);
                if ui.button("🗑").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        // 删除选中的元素
        if let Some(index) = to_remove {
            v.remove(index);
        }

        ui.add_space(8.0);

        // 添加新元素按钮
        if ui.button("➕").clicked() {
            v.push(V::default());
        }
    });
}

pub(crate) fn show_option<T: Default>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let mut ch = v.is_some();
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = T::default().into();
    } else if !ch {
        *v = None;
    }

    r
}

pub(crate) fn show_option_with<T, F>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
    f: F,
) -> egui::Response
where
    F: FnOnce() -> Option<T>,
{
    let mut ch = v.is_some();
    //let r = ui.checkbox(&mut ch, text);
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = f();
    } else if !ch {
        *v = None;
    }

    r
}

pub fn show_profiler(show: &mut bool, ui: &mut egui::Ui) {
    if ui.toggle_value(show, "profile performance").clicked() {
        puffin::set_scopes_on(*show); // Remember to call this, or puffin will be disabled!
    }
    if *show {
        puffin_egui::profiler_ui(ui)
    }
}
