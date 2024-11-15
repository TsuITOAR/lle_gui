use lle::num_complex::Complex64;

pub struct History {
    pub(crate) data: Vec<Complex64>,
    pub(crate) dim: usize,
}

impl Clone for History {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            dim: self.dim,
        }
    }
}

impl std::fmt::Debug for History {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("History")
            .field("dim", &self.dim)
            .field("data", &self.data.len())
            .finish()
    }
}

impl History {
    pub fn new(data: Vec<Complex64>) -> Self {
        Self {
            dim: data.len(),
            data,
        }
    }

    pub fn push(&mut self, data: &[Complex64]) {
        self.data.extend_from_slice(data);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    //todo: implement this

    /* pub fn show(&mut self, ui: &mut egui::Ui) {
        // a button for clear
        if ui.button("Clear").clicked() {
            self.data.clear();
        }
        if ui.button("Save Data").clicked() {
            // save the data
            if let Some(file) = rfd::FileDialog::new().save_file() {
                let serialized_data = bincode::serialize(&self.data).unwrap();
                std::fs::write(file, serialized_data).unwrap();
            }
        }
        // show current data size and memory usage
        // show a slider for selecting the range
        // show a button for saving the data
        // show a button for loading the data
    } */
}
