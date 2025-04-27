use super::processor::FftSource;

#[derive(Debug, Clone, Default)]
pub enum History<S: FftSource> {
    #[default]
    Inactive,
    ReadyToRecord,
    Recording(StoredHistory<S>),
}

impl<S: FftSource> History<S> {
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

impl<S: FftSource> History<S> {
    pub fn is_active(&self) -> bool {
        !matches!(self, History::Inactive)
    }

    pub fn active(&mut self) {
        if matches!(self, History::Inactive) {
            *self = History::ReadyToRecord;
        }
    }

    pub fn push(&mut self, data: &S) {
        match self {
            History::Inactive => (),
            History::ReadyToRecord => {
                *self = History::Recording(StoredHistory::new(vec![data.to_owned()]));
            }
            History::Recording(history) => {
                history.push(data);
            }
        }
    }

    pub fn get_data_size(&self) -> Option<(&[S], usize)> {
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

pub struct StoredHistory<S: FftSource> {
    pub(crate) data: Vec<S>,
    pub(crate) dim: usize,
}

impl<S: FftSource> Clone for StoredHistory<S> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            dim: self.dim,
        }
    }
}

impl<S: FftSource> std::fmt::Debug for StoredHistory<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("History")
            .field("dim", &self.dim)
            .field("data", &self.data.len())
            .finish()
    }
}

impl<S: FftSource> StoredHistory<S> {
    fn new(data: Vec<S>) -> Self {
        Self {
            dim: data[0].as_ref().len(),
            data,
        }
    }

    fn push(&mut self, data: &S) {
        self.data.push(data.to_owned());
    }
}
