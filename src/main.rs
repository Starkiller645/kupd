use dioxus::prelude::*;
use reqwest as req;
use fermi::*;
use riven;
use serde::{Deserialize, Serialize};
use shaco;
use std::env;
use std::default;
use std::sync::mpsc;
use std::thread;
use tokio;

pub mod backend;
pub mod gui;

use backend::*;

struct Champion {
    name: &'static str,
    id: i16,
}

struct Objective {
    name: &'static str,
    id: &'static str,
}

struct Summoner {
    kills: i8,
    deaths: i8,
    assists: i8,
    champion: Champion,
}

struct Team {
    isAlly: bool,
    champs: Vec<Champion>,
    objectives: Vec<Objective>,
}

struct Game {}

async fn launch_app() {
    dioxus::desktop::launch_cfg(gui::App, |c| {
        c.with_custom_head("<link href=\"assets/tailwind.css\" rel=\"stylesheet\" /><style>html {background: #e7e5e4; display: flex; flex-direction: column;}</style><script src=\"https://kit.fontawesome.com/a  0e919fade.js\" crossorigin=\"anonymous\"></script><link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/@fortawesome/fontawesome-free@6.1.1/css/fontawesome.min.css\" integrity=\"sha384-zIaWifL2YFF1qaDiAo0JFgsmasocJ/rqu7LKYH8CoB  EXqGbb9eO+Xi3s6fQhgFWM\" crossorigin=\"anonymous\">".to_string())
    });
}

#[tokio::main]
async fn main() {
    backend::log::startup_banner().unwrap();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-d" | "--daemon" => {
                let (_tx, rx) = futures::channel::mpsc::channel(4096);
                let green_thread = tokio::spawn(async move {
                    server::green_thread(rx).await
                });
                green_thread.await.unwrap();
            }
            "-m" | "--monitor" => {
                launch_app().await;
            }
            _ => {
                let (_tx, rx) = futures::channel::mpsc::channel(4096);
                let green_thread = tokio::spawn(async move {
                    server::green_thread(rx).await
                });
                green_thread.await.unwrap();
                launch_app().await;
            }
        }
    } else {
        let (_tx, rx) = futures::channel::mpsc::channel(4096);
        let green_thread = tokio::spawn(async move {
            server::green_thread(rx).await
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
