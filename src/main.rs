use argparse::{ArgumentParser, StoreTrue, Store};
//use riven;
//use shaco;
use std::env;
use tokio;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

pub mod backend;
pub mod gui;
pub mod config;

use backend::*;
use backend::log::*;
use config::*;

async fn launch_app() {
    dioxus::desktop::launch_cfg(gui::App, |c| {
        c.with_custom_head("<link href=\"assets/tailwind.css\" rel=\"stylesheet\" /><style>html {background: #e7e5e4; display: flex; flex-direction: column;}</style><script src=\"https://kit.fontawesome.com/a  0e919fade.js\" crossorigin=\"anonymous\"></script><link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/@fortawesome/fontawesome-free@6.1.1/css/fontawesome.min.css\" integrity=\"sha384-zIaWifL2YFF1qaDiAo0JFgsmasocJ/rqu7LKYH8CoB  EXqGbb9eO+Xi3s6fQhgFWM\" crossorigin=\"anonymous\">".to_string())
    });
}

#[tokio::main]
async fn main() {
    startup_banner().unwrap();

    let config_dir;
    let config_file;
    #[cfg(target_os = "windows")]
    {
       let appdata_dir = env::var("APPDATA").unwrap();
       config_dir = appdata_dir + "\\Kindred Daemon";
       config_file = config_dir + "\\config.json";
    }

    #[cfg(target_os = "linux")]
    {
        config_dir = match env::var("XDG_CONFIG_HOME").is_ok() {
            true => env::var("XDG_CONFIG_HOME").unwrap() + "/kindred-daemon",
            false => env::var("HOME").unwrap() + "/.config/kindred-daemon"
        };
        tokio::fs::create_dir_all(config_dir.clone()).await.unwrap();
        config_file = config_dir.clone() + "/config.json";
    }

    if !std::path::Path::new(&config_dir).exists() {
       log_warning("Creating config directory", Some(LogFrom::Setup), LogSeverity::Warning).unwrap();
       log_additional(format!("in {}", config_dir).as_str()).unwrap();
       tokio::fs::create_dir_all(config_dir).await.unwrap();
    }

    if !std::path::Path::new(&config_file).exists() {
       log_warning("Creating initial config file", Some(LogFrom::Setup), LogSeverity::Warning).unwrap();
       log_additional(format!("at {}", config_file).as_str()).unwrap();
       let mut file = tokio::fs::File::create(config_file.clone()).await.unwrap();
       file.write_all(b"{}").await.unwrap();
   }

    let mut verbose = false;
    let mut daemon = false;
    let mut monitor = false;
    let mut target_url = String::from("NONE");
    let mut api_key = String::from("NONE");

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("K/DA, the Kindred Daemon - A program to collect data from a running League of Legends client and feed it to the LAMB server.");
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Be verbose in logging (NYI).");
        ap.refer(&mut daemon)
            .add_option(&["-d", "--daemon"], StoreTrue, "Run in daemon/headless mode. Cannot be used with (-m|--monitor), will take precedence.");
        ap.refer(&mut monitor)
            .add_option(&["-m", "--monitor"], StoreTrue, "Run in monitor mode, attaching a GUI to and already-running daemon. Cannot be used with (-d|--daemon), which will take precedence.");
        ap.refer(&mut target_url)
            .add_option(&["-s", "--server"], Store, "The base url of the LAMB server to contact.");
        ap.refer(&mut api_key)
            .add_option(&["-k", "--api-key"], Store, "The API key to use for the LoL official API (NYI).");
        ap.parse_args_or_exit();
    }

    let mut config_file = tokio::fs::File::open(config_file).await.unwrap();
    let mut config_bytes: Vec<u8> = vec![];
    config_file.read_to_end(&mut config_bytes).await.unwrap();

    if daemon && monitor {
        log_warning("Both monitor and daemon mode specified!", Some(LogFrom::Main), LogSeverity::Warning).unwrap();
        log_additional("falling back to running in daemon mode (-d|--daemon)").unwrap();
        monitor = false;
    }

    let config_json: String = String::from_utf8(config_bytes).unwrap();
    let mut conf = KDConfig::from(config_json);

    if target_url != conf.server_url.clone().unwrap() && &target_url != "NONE" {
        conf.server_url = Some(target_url);
    };

    if api_key != conf.api_key.clone().unwrap() && &api_key != "NONE" {
        conf.api_key = Some(api_key);
    }

    if daemon {
        let (_tx, rx) = futures::channel::mpsc::channel(4096);
        let conf = conf.clone();
        let green_thread = tokio::spawn(async move {
            server::green_thread(rx, conf).await
        });
        green_thread.await.unwrap();
    }
    if monitor {
        launch_app().await;
    }
    if !daemon && !monitor {
        let (_tx, rx) = futures::channel::mpsc::channel(4096);
        let green_thread = tokio::spawn(async move {
            server::green_thread(rx, conf).await
        });
        launch_app().await;
        green_thread.await.unwrap();
    }
    
    /*thread::sleep(std::time::Duration::from_millis(1000));
    tx.send(datatypes::LBMessage {
        ident: String::from("meta/shutdown"),
        r#type: datatypes::LBMessageType::Update,
        data: String::from("{}")
    }).unwrap();*/
    println!("All threads joined, safe to exit!");
}
