#[derive(Debug, Clone)]
pub struct DockingScore {
    pub protein: String,
    pub ligand: String,
    pub score: f64,
    pub download_link: String,
}

#[derive(Debug)]
pub struct Config {
    pub protein_dir: String,
    pub ligand_dir: String,
    pub concurrency: usize,
    pub top_size: usize,
    pub root_url: Option<String>,
}
