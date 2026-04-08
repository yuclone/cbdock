#[derive(Debug, Clone)]
pub struct DockingScore {
    pub protein: String,
    pub ligand: String,
    pub score: f64,
    pub download_link: String,
}
