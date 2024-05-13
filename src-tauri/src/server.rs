use std::fs;
use actix_web::{HttpResponse, HttpRequest};

pub const SERVER_ADDRESS: &str = "localhost:3117";

pub async fn serve_hls(req: HttpRequest) -> HttpResponse {
  let path = req.path().trim_start_matches('/');
  
  if let Ok(content) = fs::read_to_string(path) {
    HttpResponse::Ok()
      .content_type("application/vnd.apple.mpegurl")
      .body(content)
  } else if let Ok(content) = fs::read(path) {
    HttpResponse::Ok()
      .content_type("video/MP2T")
      .body(content)
  } else {
    HttpResponse::NotFound().body("File not found")
  }
}

pub fn get_file_url(path: &str) -> String {
  "http://".to_string() + SERVER_ADDRESS + "/" + &path
}