// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
mod hls_command;
mod server;
mod cache;
mod subtitle;

use crate::{
  utils::set_window_shadow,
  // hls::generate_hls
  hls_command::generate_hls,
  server::{serve_hls, SERVER_ADDRESS},
  cache::init_hashmap,
  subtitle::generate_subtitle,
};
use std::fs;
use actix_web::{web, App, HttpServer};
use actix_cors::Cors;

// mod hls;

fn dir_exists(dir_name: &str) -> bool {
  fs::metadata(dir_name).is_ok()
}

fn main() {
  tauri::Builder::default()
  .setup(|app| {
    set_window_shadow(app);
    if !dir_exists("hls") {
      fs::create_dir("hls").unwrap();
    }
    init_hashmap("hls")?;
    
    tauri::async_runtime::spawn(
      HttpServer::new(move || {
        let cors = Cors::default()
          .allow_any_origin()
          .allow_any_header()
          .allow_any_method();
        App::new()
          .wrap(cors)
          .route("/{filename:.+}", web::get().to(serve_hls))
      })
      .bind(SERVER_ADDRESS)?
      .run(),
    );
    Ok(())
  })
  .invoke_handler(tauri::generate_handler![generate_hls, generate_subtitle])
  .run(tauri::generate_context!())
  .expect("error while running tauri application");
}
