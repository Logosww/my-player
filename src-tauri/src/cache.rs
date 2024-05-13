use std::{
  fs,
  path::Path,
  sync::Mutex,
  collections::HashMap,
};
use regex::Regex;
use lazy_static::lazy_static;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};

pub struct Cache {
  pub duration: f64,
  pub output_dir_name: String,
  pub original_file_path: String,
}

lazy_static! {
  pub static ref CACHE_MAP: Mutex<HashMap<String, Cache>> = Mutex::new(HashMap::new());
}

pub fn init_hashmap(dir_path: &str) -> Result<(), std::io::Error> {
  let dir_path = Path::new(dir_path);
  for entry in fs::read_dir(dir_path)? {
    let entry = entry?;
    let path = entry.path();

    if path.is_dir() {
      if let Some(dir_name) = path.file_name() {
        let cache = decode_dir_name_to_path(dir_name.to_string_lossy().into_owned()).unwrap();
        let mut cache_map = CACHE_MAP.lock().unwrap();
        let original_file_path = cache.original_file_path.clone();
        cache_map.insert(original_file_path, cache);
        drop(cache_map);
      }
    }
  }

  Ok(())
}

pub fn generate_dir_name(input_path: &str) -> String {
  URL_SAFE.encode(input_path)
}

pub fn cache_map_insert(input_path: String, cache: Cache) -> Result<(), std::io::Error> {
  let mut cache_map = CACHE_MAP.lock().unwrap();
  let duration_dir_name = URL_SAFE.encode("[duration=".to_string() + &cache.duration.clone().to_string() + "]");
  let duration_dir_path = "hls/".to_string() + &cache.output_dir_name + "/" + &duration_dir_name;
  fs::create_dir(&duration_dir_path).unwrap();
  cache_map.insert(input_path, cache);
  drop(cache_map);

  Ok(())
}

fn parse_cache(dir_name: &str, encoded_dir_name: &str) -> Result<Cache, std::io::Error> {
  let original_file_path = encoded_dir_name.to_string();
  let mut duration: f64 = 0.0;
  let dir_path = "hls/".to_string() + dir_name;
  let dir_path = Path::new(&dir_path);
  let duration_re = Regex::new(r"\[duration=(\d+.\d+)\]").unwrap();

  for entry in fs::read_dir(dir_path)? {
    let entry = entry?;
    let path = entry.path();

    if path.is_dir() {
      if let Some(dir_name) = path.file_name() {
        let dir_name = URL_SAFE.decode(dir_name.to_string_lossy().into_owned())
          .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
          .unwrap();
        if let Some(captures) = duration_re.captures(&dir_name) {
          duration = captures[1].parse::<f64>().unwrap();
        }
      }
    }
  }

  Ok(Cache {
    duration,
    original_file_path,
    output_dir_name: dir_name.to_owned(),
  })
}

fn decode_dir_name_to_path(dir_name: String) -> Result<Cache, base64::DecodeError> {
  URL_SAFE.decode(dir_name.clone())
    .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    .map(|encoded_dir_name| parse_cache(&dir_name, &encoded_dir_name).unwrap())
}