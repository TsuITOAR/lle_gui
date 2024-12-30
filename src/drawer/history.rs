use lle::num_complex::Complex64;

#[derive(Debug, Clone, Default)]
pub enum History {
    #[default]
    Inactive,
    ReadyToRecord,
    Recording(StoredHistory),
}

impl History {
    pub fn show_controller(&mut self, index: usize, ui: &mut egui::Ui) -> egui::Response {
        let mut active = self.is_active();
        let r = ui.toggle_value(&mut active, format!("Record history {index}"));
        if active {
            self.active();
        } else {
            self.reset();
        }
        r
    }
}

impl History {
    pub fn is_active(&self) -> bool {
        !matches!(self, History::Inactive)
    }

    pub fn active(&mut self) {
        if matches!(self, History::Inactive) {
            *self = History::ReadyToRecord;
        }
    }

    pub fn push(&mut self, data: &[Complex64]) {
        match self {
            History::Inactive => (),
            History::ReadyToRecord => {
                *self = History::Recording(StoredHistory::new(data.to_vec()));
            }
            History::Recording(history) => {
                history.push(data);
            }
        }
    }

    pub fn get_data_size(&self) -> Option<(&[Complex64], usize)> {
        match self {
            History::Inactive => None,
            History::ReadyToRecord => None,
            History::Recording(history) => Some((&history.data, history.dim)),
        }
    }

    pub fn reset(&mut self) {
        *self = History::Inactive;
    }
}

pub struct StoredHistory {
    pub(crate) data: Vec<Complex64>,
    pub(crate) dim: usize,
}

impl Clone for StoredHistory {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            dim: self.dim,
        }
    }
}

impl std::fmt::Debug for StoredHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("History")
            .field("dim", &self.dim)
            .field("data", &self.data.len())
            .finish()
    }
}

impl StoredHistory {
    fn new(data: Vec<Complex64>) -> Self {
        Self {
            dim: data.len(),
            data,
        }
    }

    fn push(&mut self, data: &[Complex64]) {
        self.data.extend_from_slice(data);
    }
}
