
#[derive(
    Debug,
    Clone,
    Copy,
    enum_iterator::Sequence,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum PlotKind {
    Line,
    Points,
}

impl crate::util::DisplayStr for PlotKind {
    fn desc(&self) -> &str {
        match self {
            PlotKind::Line => "Line",
            PlotKind::Points => "Points",
        }
    }
}
