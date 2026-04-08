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