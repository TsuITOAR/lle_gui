use rand::Rng;

#[derive(Debug, Clone)]
pub struct TestApp {
    drawer: crate::drawer::gpu::Drawer,
}

impl TestApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let drawer = crate::drawer::gpu::Drawer::new(
            "test".to_string(),
            1024,
            128,
            cc.wgpu_render_state.as_ref().unwrap(),
        );
        Self { drawer }
    }
    pub fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let mut rand = rand::thread_rng();
        let mut data = self.drawer.data();
        let uniforms = self.drawer.uniforms();
        let mut max = 1.0;
        let mut min = 0.0;
        for x in 0..uniforms.height {
            let mut new_row = vec![0f32; uniforms.width as usize];
            rand.fill(new_row.as_mut_slice());
            max = new_row.iter().fold(1f32, |a, &b| a.max(b)).max(max);
            min = new_row.iter().fold(0f32, |a, &b| a.min(b)).min(min);

            data[(x * uniforms.width) as usize..((x + 1) * uniforms.width) as usize]
                .copy_from_slice(new_row.as_slice());
        }
        drop(data);
        self.drawer.uniforms_mut().z_range = [min, max];
        self.drawer.show(ui);
    }
}

impl eframe::App for TestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.custom_painting(ui);
                });
            });
        });
    }
}
