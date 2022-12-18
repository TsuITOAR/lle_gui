#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
pub(crate) struct Property<T> {
    pub(crate) value: T,
    pub(crate) range: (T, T),
    pub(crate) label: String,
}

impl Property<f64> {
    pub fn new(v: f64, label: impl ToString) -> Self {
        Self {
            value: v,
            range: (v - 10., v + 20.),
            label: label.to_string(),
        }
    }
}
impl<T> Property<T> {
    pub fn set(&mut self, v: T) {
        self.value = v;
    }
    pub fn l_range_update(&mut self, l: T) {
        self.range.0 = l;
    }
    pub fn h_range_update(&mut self, h: T) {
        self.range.1 = h;
    }
    pub fn label(&self) -> &str {
        &self.label
    }
}
