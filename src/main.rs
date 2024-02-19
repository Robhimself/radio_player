// #![windows_subsystem = "windows"]

use minimp3::{Frame};
use anyhow::{Context, Result};
use std::{
    error::Error,
    io::{Read},
    sync::{mpsc::{self, Sender}, Mutex},
    thread,
    time::Duration
};
use fltk::{
    button::*,
    prelude::*,
    app,
    window::*,
    enums,
    valuator,
    *,
    image::SvgImage
};
use radiobrowser::{blocking::RadioBrowserAPI,ApiStation};
use rodio::{OutputStream, Sink, Source};

static PLAYER: Mutex<Option<Player>> = Mutex::new(None);

pub struct Player {
    sender: Sender<PlayerMessage>,
    volume: u8, // Between 0 and 9
}

enum PlayerMessage {
    Play { listen_url: String, volume: u8 },
    Volume { volume: u8 },
}

impl Player {
    /// Creating a `Player` might be time-consuming. It might take several seconds on first run.
    pub fn try_new() -> Result<Self> {
        OutputStream::try_default().context("Audio device initialization failed")?;
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let (mut current_listen_url, mut current_volume) = loop {
                if let Ok(PlayerMessage::Play { listen_url, volume }) = receiver.recv() {
                    break (listen_url, volume);
                }
            };

            loop {
                let client = reqwest::blocking::Client::new();
                let respons = match client.get(&current_listen_url).send() {
                    Ok(r) => r,
                    _ => panic!()
                };
                let response = reqwest::blocking::get(&current_listen_url).unwrap();
                let source = Mp3StreamDecoder::new(respons).unwrap();
                let sink = Sink::try_new(&stream_handle).unwrap();
                sink.append(source);
                sink.set_volume(Self::map_volume_to_rodio_volume(current_volume));

                while let Ok(message) = receiver.recv() {
                    match message {
                        PlayerMessage::Play { listen_url, volume } => {
                            current_listen_url = listen_url;
                            current_volume = volume;
                            break;
                        }
                        PlayerMessage::Volume { volume } => {
                            current_volume = volume;
                            sink.set_volume(Self::map_volume_to_rodio_volume(current_volume));
                        }
                    }
                }
            }
        });

        Ok(Self { sender, volume: 9 })
    }

    pub fn play(&self, listen_url: &str) {
        self.sender
            .send(PlayerMessage::Play {
                listen_url: listen_url.to_owned(),
                volume: self.volume,
            })
            .unwrap();
    }

    pub const fn volume(&self) -> u8 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = Self::cap_volume(volume);

        self.sender
            .send(PlayerMessage::Volume {
                volume: self.volume,
            })
            .unwrap();
    }

    /// Cap volume to a value between 0 and 9
    fn cap_volume(volume: u8) -> u8 {
        volume.min(9)
    }

    /// Map a volume between 0 and 9 to between 0 and 1
    fn map_volume_to_rodio_volume(volume: u8) -> f32 {
        volume as f32 / 9_f32
    }
}


pub struct Mp3StreamDecoder<R>
    where
        R: Read,
{
    decoder: minimp3::Decoder<R>,
    current_frame: Frame,
    current_frame_offset: usize,
}

impl<R> Mp3StreamDecoder<R>
    where
        R: Read,
{
    pub fn new(mut data: R) -> Result<Self, R> {
        if !is_mp3(data.by_ref()) {
            return Err(data);
        }
        let mut decoder = minimp3::Decoder::new(data);
        let current_frame = decoder.next_frame().unwrap();

        Ok(Self {
            decoder,
            current_frame,
            current_frame_offset: 0,
        })
    }
    pub fn into_inner(self) -> R {
        self.decoder.into_inner()
    }
}

impl<R> Source for Mp3StreamDecoder<R>
    where
        R: Read,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.current_frame.data.len())
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.current_frame.channels as _
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.current_frame.sample_rate as _
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl<R> Iterator for Mp3StreamDecoder<R>
    where
        R: Read,
{
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset == self.current_frame.data.len() {
            match self.decoder.next_frame() {
                Ok(frame) => self.current_frame = frame,
                _ => return None,
            }
            self.current_frame_offset = 0;
        }

        let v = self.current_frame.data[self.current_frame_offset];
        self.current_frame_offset += 1;

        Some(v)
    }
}

/// Always returns true.
fn is_mp3<R>(mut data: R) -> bool
    where
        R: Read,
{
    true

    // Returns true if the stream contains mp3 data, then resets it to where it was.
    // let stream_pos = data.seek(SeekFrom::Current(0)).unwrap();
    // let mut decoder = Decoder::new(data.by_ref());
    // let ok = decoder.next_frame().is_ok();
    // data.seek(SeekFrom::Start(stream_pos)).unwrap();

    // ok
}


async fn get_stations() -> Result<Vec<ApiStation>, Box<dyn Error>> {
    let api = RadioBrowserAPI::new()?;
    let stations = api.get_stations()
        .country("Norway")
        .send()?;
    Ok(stations)
}

#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut wind = Window::new(600, 400, 420, 410, "Rob's Rusty Radio Player");
    wind.make_resizable(false);
    wind.set_icon(Some(SvgImage::load("assets/RustLogo.svg").unwrap()));

    let mut station_list = match get_stations().await {
        Ok(stations) => stations,
        Err(err) => {
            log::error!("Error! Could not get stations.. {}", err);
            println!("Error! Could not get stations.. {}", err);
            return;
        }};
    let mut cloned_station_list = station_list.clone();

    let mut get_btn = Button::new(10, 10, 60, 30, "Refresh");
    let mut play_btn = Button::new(10, 370, 50, 30, "Play");
    let mut stop_btn = Button::new(70, 370, 50, 30, "Stop");
    let mut slider = valuator::HorNiceSlider::new(310, 375, 100, 20, "");
    slider.set_minimum(0.);
    slider.set_maximum(9.);
    slider.set_step(1., 1);
    slider.set_value(5.);

    let mut cloned_slider = slider.clone();

    let mut tree = tree::Tree::default().with_size(400, 300);
    tree.set_pos(10, 60);
    tree.set_show_root(false);
    tree.set_connector_color(enums::Color::DarkRed);

    for station in &station_list {
        if station.codec.to_uppercase() == "MP3" {
            tree.add(&station.name.clone());
        }
    }

    tree.redraw();
    let mut tree_clone = tree.clone();

    get_btn.set_callback(move |_| {
        tree.clear();
        for station in &station_list {
            if station.codec.to_uppercase() == "MP3" {
                tree.add(&station.name.clone());
            }
        }
        tree.redraw();
    });

    play_btn.set_callback(move |_| {
        match tree_clone.get_item_focus() {
            Some(ti) => {
                let item = ti.label().unwrap();
                let selected = cloned_station_list
                    .iter()
                    .find(|s| s.name == item)
                    .map(|u| u.url_resolved.clone())
                    .unwrap();

                match Player::try_new() {
                    Ok(mut player) => {
                        player.set_volume(cloned_slider.value() as u8);
                        PLAYER.lock()
                            .unwrap()
                            .replace(player);
                    }
                    Err(e) => {
                        println!("Played: {}", e);
                    }
                }
                if let Some(player) = PLAYER.lock()
                    .unwrap()
                    .as_ref() {
                    player.play(&selected);
                    tree_clone.set_label("");
                    tree_clone.redraw_label();
                    tree_clone.set_label(&format!("Playing: {}", item));
                    tree_clone.redraw_label();
                }
            },
            _ => {}
        }
    });

    stop_btn.set_callback(move |_| {
        match Player::try_new() {
            Ok(mut player) => {
                PLAYER.lock()
                    .unwrap()
                    .replace(player);
            }
            Err(e) => {
                println!("Stopped: {}", e);
            }
        }
    });

    slider.set_callback(|s| {
        if let Some(player) = PLAYER.lock()
            .unwrap()
            .as_mut() {
            player.set_volume(s.value() as u8);
        }
    });

    wind.end();
    wind.show();
    app.run().unwrap();
}


