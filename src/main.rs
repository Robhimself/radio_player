#![windows_subsystem = "windows"]

mod player;
mod mp3_stream_decoder;

use anyhow::{Result};
use std::{error::Error, sync::{Mutex}, thread, time::Duration};
use fltk::{button::*, prelude::*, app, window::*, enums, valuator, *};
use fltk::enums::Event;
use radiobrowser::{blocking::RadioBrowserAPI,ApiStation};
use rodio::{Source};

use crate::player::Player;

static PLAYER: Mutex<Option<Player>> = Mutex::new(None);

#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut wind = Window::new(600, 400, 420, 400, "Rob's Rusty Radio Player");
    wind.make_resizable(false);

    let station_list = match get_stations().await {
        Ok(stations) => stations,
        Err(err) => {
            println!("Error! Could not get stations.. {}", err);
            return;
        }};
    let cloned_station_list = station_list.clone();

    let mut get_btn = Button::new(10, 10, 60, 30, "Refresh");
    let mut play_btn = Button::new(10, 360, 50, 30, "Play");
    let mut stop_btn = Button::new(70, 360, 50, 30, "Stop");
    let mut slider = valuator::HorNiceSlider::new(310, 365, 100, 20, "");
    slider.set_minimum(0.);
    slider.set_maximum(9.);
    slider.set_step(1., 1);
    slider.set_value(5.);
    let cloned_slider = slider.clone();

    let mut tree = tree::Tree::default().with_size(400, 300);
    tree.set_pos(10, 50);
    tree.set_show_root(false);
    tree.set_connector_color(enums::Color::DarkRed);

    for station in &station_list {
        if station.codec.to_uppercase() == "MP3" {
            tree.add(&station.name.clone());
        }
    }
    tree.redraw();
    let tree_clone = tree.clone();

    get_btn.set_callback(move |_| {
        tree.clear();
        for station in &station_list {
            if station.codec.to_uppercase() == "MP3" {
                tree.add(&station.name.clone());
            }
        }
        tree.redraw();
    });

    match Player::try_new() {
        Ok(mut player) => {
            player.set_volume(cloned_slider.value() as u8);
            PLAYER.lock()
                .unwrap()
                .replace(player);
        }
        Err(e) => {
            println!("match Player.try_new() failed: {}", e);
        }
    };

    play_btn.set_callback(move |_| {
        match tree_clone.get_item_focus() {
            Some(ti) => {
                let item = ti.label().unwrap();
                let selected = cloned_station_list
                    .iter()
                    .find(|s| s.name == item)
                    .map(|u| u.url_resolved.clone())
                    .unwrap();

                if let Some(player) = PLAYER.lock()
                    .unwrap()
                    .as_ref() {
                    player.play(&selected);
                }
            },
            _ => {}
        }
    });

    stop_btn.set_callback(move |_| {
        if let Some(player) = PLAYER.lock()
            .unwrap()
            .as_mut() {
            player.stop();
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

    // Callback for the window close event
    wind.handle(move |_, ev| {
        match ev {
            Event::Close => {
                // Perform any necessary cleanup here
                if let Some(player) = PLAYER.lock()
                    .unwrap()
                    .as_mut() {
                    player.stop();
                }
                thread::sleep(Duration::new(2,0));
                std::process::abort();
            }
            _ => false,
        }
    });
    app.run().unwrap();
}

async fn get_stations() -> Result<Vec<ApiStation>, Box<dyn Error>> {
    let api = RadioBrowserAPI::new()?;
    let stations = api.get_stations()
        .country("Norway")
        .send()?;
    Ok(stations)
}
