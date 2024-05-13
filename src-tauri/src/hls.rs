extern crate ffmpeg_next as ffmpeg;

use std::collections::HashMap;
use serde::Serialize;
use std::time::Instant;

use ffmpeg::{
  codec::{self, traits::Encoder}, decoder, encoder, format, frame, log, media, picture, Dictionary, Packet, Rational,
};

#[derive(Serialize)]
pub struct ApiResponse {
  success: bool,
  message: String,
  playlist_url: Option<String>,
}

const DEFAULT_X264_OPTS: &str = "preset=medium";

struct Transcoder {
  ost_index: usize,
  decoder: decoder::Video,
  encoder: encoder::video::Encoder,
  logging_enabled: bool,
  frame_count: usize,
  last_log_frame_count: usize,
  starting_time: Instant,
  last_log_time: Instant,
}

impl Transcoder {
  fn new(
    ist: &format::stream::Stream,
    octx: &mut format::context::Output,
    ost_index: usize,
    x264_opts: Dictionary,
    enable_logging: bool,
  ) -> Result<Self, ffmpeg::Error> {
    let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
    let decoder = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?
      .decoder()
      .video()?;
    let codec = ffmpeg_next::encoder::find(ffmpeg::codec::Id::H264).unwrap();
    let mut ost = octx.add_stream(codec)?;
    let mut encoder = codec::context::Context::new_with_codec(codec).encoder().video()?;
    encoder.set_height(decoder.height());
    encoder.set_width(decoder.width());
    encoder.set_aspect_ratio(decoder.aspect_ratio());
    encoder.set_format(decoder.format());
    encoder.set_frame_rate(decoder.frame_rate());
    encoder.set_time_base(decoder.frame_rate().unwrap().invert());
    if global_header {
      encoder.set_flags(codec::Flags::GLOBAL_HEADER);
    }

    let encoder = encoder.open_with(x264_opts).expect("Failed to open encoder");
    ost.set_parameters(&encoder);
    Ok(Self {
      ost_index,
      decoder,
      encoder,
      logging_enabled: enable_logging,
      frame_count: 0,
      last_log_frame_count: 0,
      starting_time: Instant::now(),
      last_log_time: Instant::now(),
    })
  }

  fn send_packet_to_decoder(&mut self, packet: &Packet) {
    self.decoder.send_packet(packet).unwrap();
  }

  fn send_eof_to_decoder(&mut self) {
    self.decoder.send_eof().unwrap();
  }

  fn receive_and_process_decoded_frames(
    &mut self,
    octx: &mut format::context::Output,
    ost_time_base: Rational,
  ) {
    let mut frame = frame::Video::empty();
    while self.decoder.receive_frame(&mut frame).is_ok() {
      self.frame_count += 1;
      let timestamp = frame.timestamp();
      self.log_progress(f64::from(
        Rational(timestamp.unwrap_or(0) as i32, 1) * self.decoder.time_base(),
      ));
      frame.set_pts(timestamp);
      frame.set_kind(picture::Type::None);
      self.send_frame_to_encoder(&frame);
      self.receive_and_process_encoded_packets(octx, ost_time_base);
    }
  }

  fn send_frame_to_encoder(&mut self, frame: &frame::Video) {
    self.encoder.send_frame(frame).expect("Failed to send frame to encoder");
  }

  fn send_eof_to_encoder(&mut self) {
    self.encoder.send_eof().expect("Failed to send eof to encoder");
  }

  fn receive_and_process_encoded_packets(
    &mut self,
    octx: &mut format::context::Output,
    ost_time_base: Rational,
  ) {
      let mut encoded = Packet::empty();
      let mut pts: i64 = 0;
      // let mut dts: i64 = 0;
      while self.encoder.receive_packet(&mut encoded).is_ok() {
        encoded.set_stream(self.ost_index);
        let cur_pts = encoded.pts();
        let cur_dts = encoded.dts();
        if cur_pts.is_none() {
          encoded.set_pts(Some(pts));
        }
        encoded.rescale_ts(self.decoder.time_base(), ost_time_base);
        encoded.write_interleaved(octx).unwrap();
      }
  }

  fn log_progress(&mut self, timestamp: f64) {
    if !self.logging_enabled
      || (self.frame_count - self.last_log_frame_count < 100
        && self.last_log_time.elapsed().as_secs_f64() < 1.0)
    {
      return;
    }
    eprintln!(
      "time elpased: \t{:8.2}\tframe count: {:8}\ttimestamp: {:8.2}",
      self.starting_time.elapsed().as_secs_f64(),
      self.frame_count,
      timestamp
    );
    self.last_log_frame_count = self.frame_count;
    self.last_log_time = Instant::now();
  }
}

fn parse_opts<'a>(s: String) -> Option<Dictionary<'a>> {
  let mut dict = Dictionary::new();
  for keyval in s.split_terminator(',') {
    let tokens: Vec<&str> = keyval.split('=').collect();
    match tokens[..] {
      [key, val] => dict.set(key, val),
      _ => return None,
    }
  }
  Some(dict)
}

#[tauri::command]
pub async fn generate_hls(input_path: String) -> Result<ApiResponse, String> {
  let output_path = r"C:\Users\Logosw\Desktop\test\output.mp4";
  let x264_opts = parse_opts(DEFAULT_X264_OPTS.to_string()).unwrap();

  eprintln!("x264 options: {:?}", x264_opts);
  
  log::set_level(log::Level::Info);

  let mut ictx = format::input(&input_path).unwrap();
  let mut octx = format::output(&output_path).unwrap();

  format::context::input::dump(&ictx, 0, Some(&input_path));

  let best_video_stream_index = ictx
    .streams()
    .best(media::Type::Video)
    .map(|stream| stream.index());
  let mut stream_mapping: Vec<isize> = vec![0; ictx.nb_streams() as _];
  let mut ist_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];
  let mut ost_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];
  let mut transcoders = HashMap::new();
  let mut ost_index = 0;
  for (ist_index, ist) in ictx.streams().enumerate() {
    let ist_medium = ist.parameters().medium();
    if ist_medium != media::Type::Audio
      && ist_medium != media::Type::Video
      && ist_medium != media::Type::Subtitle
    {
      stream_mapping[ist_index] = -1;
      continue;
    }
    stream_mapping[ist_index] = ost_index;
    ist_time_bases[ist_index] = ist.time_base();
    if ist_medium == media::Type::Video {
      // Initialize transcoder for video stream.
      transcoders.insert(
        ist_index,
        Transcoder::new(
          &ist,
          &mut octx,
          ost_index as _,
          x264_opts.to_owned(),
          Some(ist_index) == best_video_stream_index,
        )
        .unwrap(),
      );
    } else {
      // Set up for stream copy for non-video stream.
      let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
      ost.set_parameters(ist.parameters());
      // We need to set codec_tag to 0 lest we run into incompatible codec tag
      // issues when muxing into a different container format. Unfortunately
      // there's no high level API to do this (yet).
      unsafe {
        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
      }
    }
    ost_index += 1;
  }

  octx.set_metadata(ictx.metadata().to_owned());
  format::context::output::dump(&octx, 0, Some(&output_path));
  octx.write_header().unwrap();

  for (ost_index, _) in octx.streams().enumerate() {
    ost_time_bases[ost_index] = octx.stream(ost_index as _).unwrap().time_base();
  }

  for (stream, mut packet) in ictx.packets() {
    let ist_index = stream.index();
    let ost_index = stream_mapping[ist_index];
    if ost_index < 0 {
        continue;
    }
    let ost_time_base = ost_time_bases[ost_index as usize];
    match transcoders.get_mut(&ist_index) {
      Some(transcoder) => {
        packet.rescale_ts(stream.time_base(), transcoder.decoder.time_base());
        transcoder.send_packet_to_decoder(&packet);
        transcoder.receive_and_process_decoded_frames(&mut octx, ost_time_base);
      }
      None => {
        // Do stream copy on non-video streams.
        packet.rescale_ts(ist_time_bases[ist_index], ost_time_base);
        packet.set_position(-1);
        packet.set_stream(ost_index as _);
        packet.write_interleaved(&mut octx).unwrap();
      }
    }
  }

  // Flush encoders and decoders.
  for (ost_index, transcoder) in transcoders.iter_mut() {
    let ost_time_base = ost_time_bases[*ost_index];
    transcoder.send_eof_to_decoder();
    transcoder.receive_and_process_decoded_frames(&mut octx, ost_time_base);
    transcoder.send_eof_to_encoder();
    transcoder.receive_and_process_encoded_packets(&mut octx, ost_time_base);
  }

  octx.write_trailer().unwrap();

  Ok(ApiResponse {
    success: true,
    message: "HLS stream generated successfully".to_string(),
    playlist_url: Some(r"C:\Users\Logosw\Desktop\test\output.mp4".to_string()),
  })
}