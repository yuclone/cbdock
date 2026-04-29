use std::path::PathBuf;
use walkdir::WalkDir;

/// 获取指定目录下匹配后缀名的所有文件路径
pub fn get_files_with_extension(dir: &str, ext: &str) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new(ext)))
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// 获取指定目录下匹配多个后缀名的所有文件路径
pub fn get_files_with_extensions(dir: &str, exts: &[&str]) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| exts.iter().any(|candidate| candidate == &ext))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// 去掉对接文件名中的常见后缀，保留基础名称
pub fn strip_docking_extension(file_name: &str) -> String {
    for ext in [".pdb", ".sdf", ".mol2", ".mol"] {
        if let Some(stripped) = file_name.strip_suffix(ext) {
            return stripped.to_string();
        }
    }
    file_name.to_string()
}
