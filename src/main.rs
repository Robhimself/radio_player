use std::error::Error;
use std::io::Read;
use std::result::Result;
use std::sync::{Arc, Mutex, MutexGuard};
use fltk::{app, enums, tree};
use fltk::prelude::*;
use fltk::button::*;
use fltk::window::*;
use radiobrowser::blocking::RadioBrowserAPI;
use radiobrowser::{ApiStation};
use stream_download::http::HttpStream;
use stream_download::http::reqwest::Client;
use stream_download::source::SourceStream;
use stream_download::storage::temp::TempStorageProvider;
use stream_download::{Settings, StreamDownload};
use log::info;


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


async fn play_stream(url: String) -> Result<(), Box<dyn Error>> {

    let (_stream, handle) = rodio::OutputStream::try_default()?;
    let sink = rodio::Sink::try_new(&handle)?;
    let stream = HttpStream::<Client>::create(url.parse()?,).await?;

    info!("content length={:?}", stream.content_length());
    info!("content type={:?}", stream.content_type());

    let reader = StreamDownload::from_stream(stream, TempStorageProvider::default(), Settings::default())
        .await?;

    sink.append(rodio::Decoder::new(reader)?);

    let handle = tokio::task::spawn_blocking(move || {
        sink.sleep_until_end();
    });
    handle.await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut wind = Window::new(100, 100, 600, 600, "Radio Player");
    let mut get_btn = Button::new(10, 20, 100, 40, "Get Stations");
    let mut play_btn = Button::new(200, 530, 80, 40, "Play");
    // let stop_btn = Button::new(300, 530, 80, 40, "Stop");
    wind.make_resizable(false);

    let mut tree = tree::Tree::default().with_size(400, 400).center_of_parent();
    tree.set_label("Radio Stations: ");
    tree.set_show_root(false);
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
                tree.redraw();
            }
            Err(err) => {
                println!("Error: could not fetch stations: {}", err);
                tree.clear();
                tree.add("Could not fetch, try again.");
                tree.redraw();
            }
        }
    });

    play_btn.set_callback(move |_| {
        let item = tree_clone.get_item_focus().and_then(|item| item.label());
        if let Some(tree_item) = item {
            let tracked = tracked_clone.lock().unwrap();
            if let Some(url) = get_station(&tracked, &tree_item) {
                println!("Name: {}, URL: {}", tree_item, url);
                open::that(url);
                // play_stream(url.clone());

            }
        }
    });

    wind.end();
    wind.show();
    app.run().unwrap();
}