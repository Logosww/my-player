use url::Url;
use reqwest::Client;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use serde_json::{Map, Value};
use std::{
  env,
  fs::{self, File},
  io::{Read, Write},
};
use crate::{
  cache::CACHE_MAP,
  server::get_file_url,
};

lazy_static! {
  static ref VC_APP_ID: String = get_vc_app_var("VC_APP_ID");
  static ref VC_APP_ACCESS_TOKEN: String = get_vc_app_var("VC_APP_ACCESS_TOKEN");
  pub static ref HTTP_CLIENT: Client = Client::new();
}

#[derive(Deserialize)] 
struct UploadEndpointResult {  
  code: u64,
  message: String,
  id: String,
}

#[derive(Deserialize)]
struct ResultQueryEndpointResult {  
  code: u64,
  message: String,
  utterances: Vec<SubtitleEntry>,

  #[serde(flatten)]
  _unknown: Map<String, Value>,
}

#[derive(Deserialize)]
struct SubtitleEntry {
  text: String,
  start_time: u64,
  end_time: u64,

  #[serde(flatten)]
  _unknown: Map<String, Value>,
}

#[derive(Serialize)]
pub struct ApiResponse {
  success: bool,
  message: String,
  subtitle_url: String,
}

fn get_vc_app_var(key: &str) -> String {
  env::var(key).unwrap_or_else(|_| "".to_string())
}

pub async fn upload_audio(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut audio_file = File::open(file_path)?;
  let mut buffer: Vec<u8> = Vec::new();

  audio_file.read_to_end(&mut buffer)?;

  let mut full_url = String::new();
  {
    let mut url = Url::parse("https://openspeech.bytedance.com/api/v1/vc/submit")?;
    let mut builder = url.query_pairs_mut();
    builder.append_pair("appid", &VC_APP_ID).append_pair("max_lines", "2");
    full_url = builder.finish().to_string();
  }

  let response = HTTP_CLIENT
    .post(&full_url)
    .header("Content-Type", "audio/aac")
    .header("Authorization", "Bearer; ".to_string() + &VC_APP_ACCESS_TOKEN)
    .body(buffer)
    .send()
    .await?;

  match response.status().is_success() {
    true => {
      let result = response.json::<UploadEndpointResult>().await?;
      if result.code != 0 {
        return Err(result.message.into());
      } else {
        return Ok(result.id);
      }
    },
    false => Err("Failed to upload audio file".into()),
  }
}

fn convert_to_vtt(entries: &Vec<SubtitleEntry>, output_dir_name: &str) -> Result<(), Box<dyn std::error::Error>> {  
  let output_file_path = "hls/".to_string() + output_dir_name + "/subtitle.vtt";
  let mut output_file = File::create(&output_file_path)?; 

  writeln!(output_file, "WEBVTT\n")?; 

  for entry in entries {  
    let start_time = format_time(entry.start_time);
    let end_time = format_time(entry.end_time);

    writeln!(output_file, "{} --> {}\n{}\n", start_time, end_time, entry.text)?;
  } 

  Ok(())
}

fn format_time(millis: u64) -> String {  
  let hours = millis / (1000 * 60 * 60);
  let minutes = (millis / (1000 * 60)) % 60;
  let seconds = (millis / 1000) % 60;
  let millis_part = millis % 1000;

  format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis_part)
}

async fn get_order_result(order_id: &str) -> Result<Vec<SubtitleEntry>, Box<dyn std::error::Error>> {
  let mut full_url = String::new();
  {
    let mut url = Url::parse("https://openspeech.bytedance.com/api/v1/vc/query")?;
    let mut builder = url.query_pairs_mut();
    builder.append_pair("appid", &VC_APP_ID).append_pair("id", order_id);
    full_url = builder.finish().to_string();
  }

  let response = HTTP_CLIENT
    .get(&full_url)
    .header("Authorization", "Bearer; ".to_string() + &VC_APP_ACCESS_TOKEN)
    .send()
    .await?;

  match response.status().is_success() {
    true => {
      let result = response.json::<ResultQueryEndpointResult>().await?;
      if result.code != 0 {
        return Err(result.message.into());
      } else {
        return Ok(result.utterances);
      }
    }
    false => Err("Failed to get order result".into()),
  }
}

#[tauri::command]
pub async fn generate_subtitle(input_path: String) -> Result<ApiResponse, String> {
  let mut output_dir_name = String::new();
  {
    let cache_map = CACHE_MAP.lock().unwrap();
    let cache = cache_map.get(&input_path).unwrap();
    output_dir_name = cache.output_dir_name.clone();
    drop(cache_map);
  }
  let audio_path = "hls/".to_string() + &output_dir_name + "/audio.aac";
  let subtitle_path = "hls/".to_string() + &output_dir_name + "/subtitle.vtt";
  if fs::metadata(&subtitle_path).is_ok() {
    return Ok(ApiResponse {
      success: true,
      message: "Subtitle generated successfully.".to_string(),
      subtitle_url: get_file_url(&subtitle_path),
    });
  }

  let order_id = upload_audio(&audio_path).await.unwrap();
  let subtitle = get_order_result(&order_id).await.unwrap();
  convert_to_vtt(&subtitle, &output_dir_name).unwrap();

  Ok(ApiResponse {
    success: true,
    message: "Subtitle generated successfully.".to_string(),
    subtitle_url: get_file_url(&subtitle_path),
  })
}