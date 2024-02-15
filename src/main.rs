#![windows_subsystem = "windows"]

use std::error::Error;
use std::io::Cursor;
use std::result::Result;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use fltk::{app, enums, tree};
use fltk::prelude::*;
use fltk::button::*;
use fltk::image::SvgImage;
use fltk::window::*;
use radiobrowser::blocking::RadioBrowserAPI;
use radiobrowser::{ApiStation};
use stream_download::http::reqwest::Client;
use rodio::{Decoder, Sink};


fn get_stations() -> Result<Vec<ApiStation>, Box<dyn Error>> {
    let api = RadioBrowserAPI::new()?;
    let stations = api.get_stations()
        .country("Norway")
        .send()?;
    Ok(stations)
}

fn get_station(stations: &MutexGuard<Vec<(String, String)>>, name: &str) -> Option<String> {
    stations.iter()
        .find(|station| station.0 == name)
        .map(|station| station.1.clone())
}

fn fetch_audio_data(url: String) -> Result<(), Box<dyn Error>> {
    // let client = Client::new();
    // println!("1");
    // let response = client.get(&url).send();
    // println!("2");
    // let bytes = response.bytes()?;
    // println!("3");
    //
    //
    Ok(())
}



fn play_stream(audio_data: Vec<u8>) {
// fn play_stream(url: String) {

    // let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    //
    // let cursor = Cursor::new(audio_data);
    // let decoder = Decoder::new(cursor).expect("Failed to decode audio data");
    //
    // // Create a sink for playback
    // let sink = Sink::try_new(&stream_handle).expect("Failed to create sink for playback");
    //
    // tokio::spawn(async move {
    //     // Play the audio stream
    //     sink.append(decoder);
    //     sink.sleep_until_end();
    // });

}

 fn main() -> Result<(), Box<dyn Error>> {

    let app = app::App::default();
    let mut wind = Window::new(100, 100, 600, 600, "WIP Radio Player");
    wind.make_resizable(true);
    let svg_image = SvgImage::load("./assets/RustLogo.svg").unwrap();
    wind.set_icon(Some(svg_image.clone()));

    let mut get_btn = Button::new(10, 20, 80, 40, "Fetch");
    let mut play_btn = Button::new(140, 530, 80, 40, "Open");
    let stop_btn = Button::new(380, 530, 80, 40, "Stop");

    let mut tree = tree::Tree::default().with_size(400, 400).center_of_parent();
    tree.set_label("Radio Stations");
    tree.set_root_label("Empty");
    tree.set_show_collapse(true);
    tree.set_connector_color(enums::Color::DarkRed);

    let tree_clone = tree.clone();
    let tracked: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let tracked_clone = tracked.clone();

    get_btn.set_callback(move |_| {
        match get_stations() {
            Ok(stations) => {
                tree.clear();
                let mut tracked = tracked.lock().unwrap();
                tracked.clear();
                for station in &stations {
                    tracked.push((station.name.clone(), station.url.clone()));
                    tree.add(&station.name);
                }
                tree.set_root_label("Norway");
                tree.redraw();

            }
            Err(err) => {
                tree.clear();
                tree.add(&*format!("Could not fetch, try again. {err}"));
                tree.set_root_label("ERROR");
                tree.redraw();
            }
        }
    });

    play_btn.set_callback(move |_| {
        let item = tree_clone.get_item_focus().and_then(|item| item.label());
        if let Some(tree_item) = item {
            let tracked = tracked_clone.lock().unwrap();
            if let Some(url) = get_station(&tracked, &tree_item) {
                open::that(url).expect("Failed to open url in default browser.");
                // play_stream(url.clone());
                //     if let Ok(audio_data) = fetch_audio_data(url.clone()) {
                //         println!("4");
                //         thread::spawn(move || {
                //         play_stream(audio_data);
                //         });
                //     } else {
                //         println!("Whops");
                //     }
            }
        }
    });

    wind.end();
    wind.show();
    app.run().unwrap();

    Ok(())
}