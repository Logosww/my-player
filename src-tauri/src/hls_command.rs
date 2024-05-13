use serde::Serialize;
use std::{
  fs,
  time::Duration,
  process::Stdio,
};
use regex::Regex;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use crate::{
  server::get_file_url,
  cache::{
    Cache,
    CACHE_MAP,
    cache_map_insert,
    generate_dir_name,
  }
};

struct FFmpegCommand {
  input_path: String,
  output_dir_name: String,
}

impl FFmpegCommand {
  fn new(
    input_path: String,
    output_dir_name: String,
  ) -> Result<Self, ()> {
    Ok(Self {
      input_path,
      output_dir_name
    })
  }

  pub async fn execute(&mut self) -> Result<f64, Box<dyn std::error::Error>> {
    let output_segement_path = "hls/".to_string() + &self.output_dir_name + "/%03d.ts";
    let output_playlist_path = "hls/".to_string() + &self.output_dir_name + "/playlist.m3u8";
    let output_audio_path = "hls/".to_string() + &self.output_dir_name + "/audio.aac";

    let mut transcode_cmd = Command::new("ffmpeg");
    transcode_cmd
      .args([
        "-hwaccel", "cuda",
        "-hwaccel_output_format", "cuda",
        "-i", &self.input_path,
        "-c:v", "h264_nvenc",
        "-c:a", "aac",
        "-hls_time", "10",
        "-hls_list_size", "0",
        "-hls_segment_filename", &output_segement_path,
        &output_playlist_path,
      ])
      .stderr(Stdio::piped());

    let mut audio_extrac_cmd = Command::new("ffmpeg")
      .args([
        "-i", &self.input_path,
        "-map", "0:a",
        "-c:a", "aac",
        "-ar", "16000",
        "-ac", "2",
        &output_audio_path,
      ])
      .spawn()
      .expect("Failed to spawn audio extract command");

    let mut child = transcode_cmd.spawn().expect("Failed to spawn command");
    let stdout = child.stderr.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout).lines();

    tokio::spawn(async move {
      let status = child.wait().await
        .expect("child process encountered an error");

      println!("child status was: {}", status);
    });
    tokio::spawn(async move {
      audio_extrac_cmd.wait().await.expect("Failed to extract audio");
    });

    let playlist_regex = Regex::new(r"Opening '.+?m3u8.tmp' for writing").unwrap();
    let duration_regex = Regex::new(r"Duration: (\d{2}):(\d{2}):(\d{2}\.\d{2})").unwrap();
    let mut duration: f64 = 0.0;
    while let Some(line) = reader.next_line().await? {
      if let Some(captures) = duration_regex.captures(&line) {
        let hours = captures[1].parse::<u64>().unwrap();
        let minutes = captures[2].parse::<u64>().unwrap();
        let seconds = captures[3].parse::<f64>().unwrap();

        duration = Duration::new(hours * 3600 + minutes * 60 + seconds.trunc() as u64, (seconds.fract() * 1e9) as u32).as_secs_f64();
      }
      if playlist_regex.is_match(&line) {
        return Ok(duration)
      }
    }

    Ok(duration)
  }
}

#[derive(Serialize)]
pub struct ApiResponse {
  success: bool,
  message: String,
  playlist_url: String,
  duration: f64,
}

#[tauri::command]
pub async fn generate_hls(input_path: String) -> Result<ApiResponse, String> {
  let mut duration: f64 = 0.0;
  {
    let cache_map = CACHE_MAP.lock().unwrap();
    let cache = cache_map.get(&input_path);
    if cache.is_some() {
      let cache = cache.unwrap();
      let output_dir_name = cache.output_dir_name.clone();
      duration = cache.duration.clone();
      let output_playlist_path = "hls/".to_string() + &output_dir_name + "/playlist.m3u8";
      drop(cache_map);

      return Ok(ApiResponse {
        duration,
        success: true,
        message: "HLS stream generated successfully".to_string(),
        playlist_url: get_file_url(&output_playlist_path),
      })
    }
    drop(cache_map);
  }
  let output_dir_name = generate_dir_name(&input_path);
  let output_playlist_path = "hls/".to_string() + &output_dir_name + "/playlist.m3u8";
  let mut cmd = FFmpegCommand::new(
    input_path.clone(),
    output_dir_name.clone(),
  ).unwrap();
  
  fs::create_dir("hls/".to_owned() + &output_dir_name).unwrap();

  if let Ok(duration) = cmd.execute().await {
    cache_map_insert(input_path.clone(), Cache {
      duration,
      output_dir_name,
      original_file_path: input_path,
    }).unwrap();
    
    return Ok(ApiResponse {
      duration,
      success: true,
      message: "HLS stream generated successfully".to_string(),
      playlist_url: get_file_url(&output_playlist_path),
    })
  }

  Ok(ApiResponse {
    duration,
    success: true,
    message: "HLS stream generated successfully".to_string(),
    playlist_url: get_file_url(&output_playlist_path),
  })
}
