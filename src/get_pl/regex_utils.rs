use regex::Regex;
use std::sync::OnceLock;

pub fn get_regex_url() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"http://[0-9.]+(?::\d+)?/[^";\s]+"#).unwrap())
}

pub fn get_regex_temp() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"var temp_dir\s*=\s*["'](.*?)["'];"#).unwrap())
}

pub fn get_regex_user() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"var userName\s*=\s*["'](.*?)["'];"#).unwrap())
}

pub fn get_regex_percent() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"js_percent\s*=\s*([\d\.]+)").unwrap())
}

pub fn get_regex_error() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"error_json\s*=\s*["'](.*?)["']"#).unwrap())
}

pub fn get_regex_job_dir() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"var\s+current_jobDir\s*=\s*["'](.*?)["']"#).unwrap())
}
