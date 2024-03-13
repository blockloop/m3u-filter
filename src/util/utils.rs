use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use log::{debug, error, Level};
use path_absolutize::*;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use crate::create_m3u_filter_error_result;
use crate::m3u_filter_error::{M3uFilterError, M3uFilterErrorKind};
use crate::model::config::{ConfigInput};

#[macro_export]
macro_rules! exit {
    ($($arg:tt)*) => {{
        error!($($arg)*);
        std::process::exit(1);
    }};
}


pub(crate) fn get_exe_path() -> PathBuf {
    let default_path = std::path::PathBuf::from("./");
    let current_exe = std::env::current_exe();
    match current_exe {
        Ok(exe) => {
            match fs::read_link(&exe) {
                Ok(f) => f.parent().map_or(default_path, |p| p.to_path_buf()),
                Err(_) => return exe.parent().map_or(default_path, |p| p.to_path_buf())
            }
        }
        Err(_) => default_path
    }
}

fn get_default_path(file: &str) -> String {
    let path: PathBuf = get_exe_path();
    let default_path = path.join(file);
    String::from(if default_path.exists() {
        default_path.to_str().unwrap_or(file)
    } else {
        file
    })
}

fn get_default_file_path(config_path: &str, file: &str) -> String {
    let path: PathBuf = PathBuf::from(config_path);
    let default_path = path.join(file);
    String::from(if default_path.exists() {
        default_path.to_str().unwrap_or(file)
    } else {
        file
    })
}

pub(crate) fn get_default_config_path() -> String {
    get_default_path("config")
}

pub(crate) fn get_default_config_file_path(config_path: &str) -> String {
    get_default_file_path(config_path, "config.yml")
}

pub(crate) fn get_default_sources_file_path(config_path: &str) -> String {
    get_default_file_path(config_path, "source.yml")
}

pub(crate) fn get_default_mappings_path(config_path: &str) -> String {
    get_default_file_path(config_path, "mapping.yml")
}

pub(crate) fn get_default_api_proxy_config_path(config_path: &str) -> String {
    get_default_file_path(config_path, "api-proxy.yml")
}

pub(crate) fn get_working_path(wd: &String) -> String {
    let current_dir = std::env::current_dir().unwrap();
    if wd.is_empty() {
        String::from(current_dir.to_str().unwrap_or("."))
    } else {
        let work_path = std::path::PathBuf::from(wd);
        let wdpath = match fs::metadata(&work_path) {
            Ok(md) => {
                if md.is_dir() && !md.permissions().readonly() {
                    match work_path.canonicalize() {
                        Ok(ap) => Some(ap),
                        Err(_) => None
                    }
                } else {
                    error!("Path not found {:?}", &work_path);
                    None
                }
            }
            Err(_) => None,
        };
        let rp: PathBuf = match wdpath {
            Some(d) => d,
            None => current_dir.join(wd)
        };
        match rp.canonicalize() {
            Ok(ap) => String::from(ap.to_str().unwrap_or("./")),
            Err(_) => {
                error!("Path not found {:?}", &rp);
                String::from("./")
            }
        }
    }
}

pub(crate) fn open_file(file_name: &Path) -> Result<fs::File, std::io::Error> {
    fs::File::open(file_name)
}

pub(crate) async fn get_input_text_content(input: &ConfigInput, working_dir: &String, url_str: &str, persist_filepath: Option<PathBuf>) -> Result<String, M3uFilterError> {
    debug!("getting input text content working_dir: {}, url: {}", working_dir, url_str);
    match url_str.parse::<url::Url>() {
        Ok(url) => match download_text_content(input, url, persist_filepath).await {
            Ok(content) => Ok(content),
            Err(e) => {
                error!("cant download input url: {}  => {}", url_str, e);
                create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "Failed to download")
            }
        }
        Err(_) => {
            let result = match get_file_path(working_dir, Some(PathBuf::from(url_str))) {
                Some(filepath) => {
                    if filepath.exists() {
                        if let Some(persist_file_value) = persist_filepath {
                            let to_file = &persist_file_value;
                            match fs::copy(&filepath, to_file) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("cant persist to: {}  => {}", to_file.to_str().unwrap_or("?"), e);
                                    return create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "Failed to persist: {}  => {}", to_file.to_str().unwrap_or("?"), e);
                                }
                            }
                        };
                        match open_file(&filepath) {
                            Ok(file) => {
                                let mut content = String::new();
                                match std::io::BufReader::new(file).read_to_string(&mut content) {
                                    Ok(_) => Some(content),
                                    Err(err) => {
                                        let file_str = &filepath.to_str().unwrap_or("?");
                                        error!("cant read file: {} {}", file_str,  err);
                                        return create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "Cant open file : {}  => {}", file_str,  err);
                                    }
                                }
                            }
                            Err(err) => {
                                let file_str = &filepath.to_str().unwrap_or("?");
                                error!("cant read file: {} {}", file_str,  err);
                                return create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "Cant open file : {}  => {}", file_str,  err);
                            }
                        }
                    } else {
                        None
                    }
                }
                None => None
            };
            match result {
                Some(content) => Ok(content),
                None => {
                    let msg = format!("cant read input url: {:?}", url_str);
                    error!("{}", msg);
                    create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "{}", msg)
                }
            }
        }
    }
}


fn persist_file(persist_file: Option<PathBuf>, text: &String) {
    if let Some(path_buf) = persist_file {
        let filename = &path_buf.to_str().unwrap_or("?");
        match fs::File::create(&path_buf) {
            Ok(mut file) => match file.write_all(text.as_bytes()) {
                Ok(_) => debug!("persisted: {}", filename),
                Err(e) => error!("failed to persist file {}, {}", filename, e)
            },
            Err(e) => error!("failed to persist file {}, {}", filename, e)
        }
    }
}

pub(crate) fn prepare_persist_path(file_name: &str, date_prefix: &str) -> Option<PathBuf> {
    let now = chrono::Local::now();
    let filename = file_name.replace("{}", format!("{}{}", date_prefix, now.format("%Y%m%d_%H%M%S").to_string().as_str()).as_str());
    Some(std::path::PathBuf::from(filename))
}

pub(crate) fn get_file_path(wd: &String, path: Option<PathBuf>) -> Option<PathBuf> {
    match path {
        Some(p) => {
            if p.is_relative() {
                let pb = PathBuf::from(wd);
                match pb.join(&p).absolutize() {
                    Ok(os) => Some(PathBuf::from(os)),
                    Err(e) => {
                        error!("path is not relative {:?}", e);
                        Some(p)
                    }
                }
            } else {
                Some(p)
            }
        }
        None => None
    }
}


pub(crate) fn get_client_request(input: &ConfigInput, url: url::Url, custom_headers: Option<&HashMap<&str, &[u8]>>) -> reqwest::RequestBuilder {
    let mut request = reqwest::Client::new().get(url);
    let headers = get_request_headers(&input.headers, custom_headers);
    request = request.headers(headers);
    request
}

pub(crate) fn get_request_headers(defined_headers: &HashMap<String, String>, custom_headers: Option<&HashMap<&str, &[u8]>>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (key, value) in defined_headers {
        headers.insert(
            HeaderName::from_bytes(key.as_bytes()).unwrap(),
            HeaderValue::from_bytes(value.as_bytes()).unwrap());
    }
    if let Some(custom) = custom_headers {
        let header_keys: HashSet<String> = headers.keys().map(|k| k.as_str().to_lowercase()).collect();
        for (key, value) in custom {
            let key_lc = key.to_lowercase();
            if !("host" == key_lc || header_keys.contains(key_lc.as_str())) {
                headers.insert(
                    HeaderName::from_bytes(key.as_bytes()).unwrap(),
                    HeaderValue::from_bytes(value).unwrap());
            } else {
                debug!("Ignoring request header {}={}", key_lc, String::from_utf8_lossy(value));
            }
        }
    }
    if log::max_level() == Level::Debug {
        let he: HashMap<String, String> = headers.iter().map(|(k, v)| (k.to_string(), String::from_utf8_lossy(v.as_bytes()).to_string())).collect();
        debug!("Request headers {:?}", he);
    }
    headers
}

async fn download_json_content(input: &ConfigInput, url: url::Url, persist_filepath: Option<PathBuf>) -> Result<serde_json::Value, String> {
    let request = get_client_request(input, url, None);
    match request.send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(content) => {
                        if persist_filepath.is_some() {
                            persist_file(persist_filepath, &serde_json::to_string(&content).unwrap());
                        }
                        Ok(content)
                    }
                    Err(e) => Err(e.to_string())
                }
            } else {
                Err(format!("Request failed: {}", response.status()))
            }
        }
        Err(e) => Err(e.to_string())
    }
}

pub(crate) async fn get_input_json_content(input: &ConfigInput, url_str: &str, persist_filepath: Option<PathBuf>) -> Result<serde_json::Value, M3uFilterError> {
    match url_str.parse::<url::Url>() {
        Ok(url) => match download_json_content(input, url, persist_filepath).await {
            Ok(content) => Ok(content),
            Err(e) => create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "cant download input url: {}  => {}", url_str, e)
        },
        Err(_) => create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "malformed input url: {}", url_str)
    }
}

async fn download_text_content(input: &ConfigInput, url: url::Url, persist_filepath: Option<PathBuf>) -> Result<String, String> {
    let request = get_client_request(input, url, None);
    match request.send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.text_with_charset("utf8").await {
                    Ok(content) => {
                        if persist_filepath.is_some() {
                            persist_file(persist_filepath, &content);
                        }
                        Ok(content)
                    }
                    Err(e) => Err(e.to_string())
                }
            } else {
                Err(format!("Request failed: {}", response.status()))
            }
        }
        Err(e) => Err(e.to_string())
    }
}

pub(crate) fn bytes_to_megabytes(bytes: u64) -> u64 {
    bytes / 1_048_576
}

pub(crate) fn add_prefix_to_filename(path: &Path, prefix: &str, ext: Option<&str>) -> PathBuf {
    let file_name = path.file_name().unwrap_or_default();
    let new_file_name = format!("{}{}", prefix, file_name.to_string_lossy());
    let result = path.with_file_name(new_file_name);
    match ext {
        None => result,
        Some(extension) => result.with_extension(extension)
    }
}

pub(crate) fn path_exists(file_path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(file_path) {
        return metadata.is_file();
    }
    false
}
