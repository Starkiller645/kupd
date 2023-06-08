#![allow(non_snake_case)]
use crate::backend::datatypes::*;
use crate::backend::*;
use crate::backend::log::*;
use std::collections::HashMap;
use dioxus::prelude::*;
use fermi::*;
use futures::{FutureExt, StreamExt};
use chrono::{DateTime, TimeZone, Utc, Local, Timelike};
use tokio_tungstenite as ts;
use tungstenite::Message::*;
use lazy_static::*;

static GLOBAL_STATE: AtomRef<datatypes::KDData> = |_| Default::default();

#[derive(Copy, Clone, PartialEq)]
pub enum LoadingState {
    Loading,
    NoAction,
    Finished,
}

#[derive(Clone, PartialEq)]
pub struct LoadingData {
    state: LoadingState,
    message: String,
}

lazy_static! {
    static ref QUEUE_MAP: HashMap<i16, &'static str> = HashMap::from([
        (325, "All Random"),
        (400, "5v5 Draft Pick"),
        (420, "5v5 Ranked Solo/Duo"),
        (430, "5v5 Blind Pick"),
        (440, "5v5 Ranked Flex"),
        (450, "5v5 ARAM"),
        (700, "5v5 Clash"),
        (830, "Co-op vs. AI Beginner"),
        (840, "Co-op vs. AI Intro"),
        (850, "Co-op vs. AI Intermediate"),
        (900, "5v5 All Random URF"),
        (1020, "5v5 One for All"),
        (1090, "FFA Teamfight Tactics"),
        (1100, "FFA Teamfight Tactics Ranked"),
        (1300, "5v5 Nexus Blitz"),
        (1400, "5v5 Ultimate Spellbook"),
        (1900, "5v5 Ultra Rapid Rife")
    ]);
}

fn secs_to_string(secs: i16) -> String {
    let time = Local.timestamp_opt(secs as i64, 0).unwrap();
    let mut return_string = time.format("%M:%S").to_string();
    if time.minute() < 10 {
        return_string = String::from(&return_string[1..]);
    }
    return_string
}

pub fn App(cx: Scope) -> Element {
    let state = use_atom_ref(&cx, GLOBAL_STATE);
    let new_state = state.clone();
    use_coroutine(&cx, |_rx: UnboundedReceiver<String>| {
        async move {
            let m = Some(LogFrom::Main);
            let (mut client, _response) =
                ts::connect_async("ws://127.0.0.1:8000/ws").await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            log("Connected to websocket bus!", m).unwrap();
            log_additional("on ws://127.0.0.1:8000/ws").unwrap();

            while let Some(message) = client.next().await {
                match message {
                    Ok(message) => match message {
                        Text(message) => {
                            let message: LBMessage =
                                serde_json::from_str(message.as_str()).unwrap();
                            match message.ident.as_str() {
                                "live/champ-select" => {
                                    if new_state.read().client.status != KDStatus::ChampSelect {
                                        new_state.write().client.status = KDStatus::ChampSelect;
                                        log("Entering champ select phase...", m).unwrap();
                                        new_state.write().client.champ_select.metadata.phase = KDChampSelectPhase::Waiting;
                                    };
                                }
                                "live/champ-select/update" => {
                                    let data: KDChampSelect = serde_json::from_str(message.data.as_str()).unwrap();
                                    new_state.write().client.champ_select = data.clone();
                                }
                                "live/end" => {
                                    match new_state.read().client.status {
                                        KDStatus::ChampSelect => log("Champ Select cancelled, returning to main screen...", m,).unwrap(),
                                        _ => log("Game ended, returning to main screen...", m).unwrap()
                                    }
                                    new_state.write().client.status = KDStatus::Connected;
                                }
                                "live/disconnect" => {
                                    new_state.write().client.status = KDStatus::Disconnected;
                                    log("Lost our connection to the client, bailing...", m).unwrap();
                                }
                                "live/queue" => {
                                    new_state.write().client.status = KDStatus::InQueue;
                                    let data: KDQueue = serde_json::from_str(message.data.as_str()).unwrap();
                                    new_state.write().client.queue = data.clone();
                                    log(format!("Joined queue {}", data.id).as_str(), m).unwrap();
                                    log_additional(format!("Current time {} seconds, ETA {} seconds", data.time, data.expected_time).as_str()).unwrap();
                                }
                                "live/connect" => {
                                    new_state.write().client.status = KDStatus::Connected;
                                    log("Connected to the League Client!", m).unwrap();
                                    new_state.write().metadata.has_connected = true;
                                }
                                "live/await" => {
                                    new_state.write().client.status = KDStatus::Waiting;
                                    log("Waiting for game to start...", m).unwrap();
                                }
                                "live/metadata" => {
                                    new_state.write().client.status = KDStatus::InGame;
                                    new_state.write().client.game.metadata = serde_json::from_str(message.data.as_str()).unwrap();
                                    log("We're now in a game, yay!", m).unwrap()
                                }
                                _ => {
                                    log(format!("Uh oh, got a message we don't understand: {}", message.ident.clone()).as_str(), m).unwrap();
                                    log_additional("Assuming we're still connected, WARNING FIXME!").unwrap();
                                    new_state.write().client.status = KDStatus::Connected;
                                }
                            };
                        }
                        _ => {
                            log("Websocket error: only text is supported!", m).unwrap();
                        }
                    },
                    Err(err) => {
                        println!("[MAIN] Error receiving websocket message!");
                    }
                }
            }
        }
    });
    
    cx.render(rsx! {
        div {
            class: "absolute inset-5 flex flex-col p-6 bg-stone-200",
            match state.read().client.status {
                KDStatus::ChampSelect => rsx! {
                    ChampSelect {}
                },
                KDStatus::Disconnected | KDStatus::Connected | KDStatus::InQueue => rsx! {
                    MainScreen {}
                },
                KDStatus::InGame | KDStatus::Waiting => rsx! {
                    InGame {}
                },
                _ => rsx! {
                    h1 {
                        class: "font-league text-6xl animate-in-50 text-red-500",
                        "Uh oh! We're not quite sure what happened here..."
                    }
                }
            }
        }
    })
}

fn MainScreen(cx: Scope) -> Element {
    let state = use_atom_ref(&cx, GLOBAL_STATE);

    cx.render(rsx! {
        div {
            class: "m-auto rounded-xl bg-stone-100 text-stone-800 text-center p-6 w-[500px]",
            h1 {
                class: "text-9xl font-league text-yellow-700 animate-in-50",
                "K/DA"
            },
            match state.read().client.status {
                KDStatus::InQueue => rsx! {
                    h2 {
                        class: "text-4xl font-league text-stone-600 animate-in-50",
                        "In Queue"
                    },
                },
                _ => rsx! {
                    h2 {
                        class: "text-4xl font-league text-stone-600 animate-in-50",
                        "Kindred Daemon"
                    }
                }
            }
            p {
                class: "font-lato animate-in-100",
                match state.read().client.status {
                    KDStatus::Disconnected => {
                        rsx! {
                            p {
                                class: "pt-2 text-xl text-stone-500 font-lato animate-in-100 animate-pulse",
                                "Waiting for connection to League"
                            },
                            match state.read().metadata.has_connected {
                                false => rsx! {
                                    p {
                                        class: "text-stone-100 text-xl",
                                        "..."
                                    }
                                },
                                true => rsx! {
                                    p {
                                        class: "text-red-700 text-xl font-bold font-lato animate-in-out-after-2",
                                        "Disconnected"
                                    }
                                }
                            },
                            p {
                            class: "opacity-0 text-lg font-lato",
                            "..."
                        }
                        }
                    }
                    KDStatus::InQueue => {
                        let timer = use_state(&cx, || 0 as i64);
                        let timer_clone = timer.clone();
                        let state_clone = state.clone();
                        use_coroutine(&cx, |_: UnboundedReceiver<String>| async move {
                            loop {
                                let queue = state_clone.read().client.queue.clone();
                                let start_time = queue.start_time;
                                let current_time: i64 = chrono::Local::now().timestamp();
                                timer_clone.set(current_time - start_time);
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                        });

                        let queue_data: KDQueue = state.read().client.queue.clone();
                        let queue_name = QUEUE_MAP[&queue_data.id];
                        let queue_time = secs_to_string(*timer.current() as i16);
                        let queue_expected = secs_to_string(queue_data.expected_time);
                        rsx! {
                            div {
                                class: "font-lato text-xl animate-in-100",
                                p {
                                    class: "font-league text-3xl text-yellow-700",
                                    "{queue_name}"
                                },
                                p {
                                    class: "font-stone-500 font-bold",
                                    "Queue time: ",
                                    span {
                                        class: "text-yellow-700",
                                        "{queue_time}"
                                    }
                                }
                                p {
                                    class: "font-stone-500 text-lg",
                                    "Estimated: {queue_expected}"
                                }
                            }
                        }
                    }
                    _ => rsx! {
                        p {
                                class: "pt-2 text-xl font-lato mx-auto animate-pulse",
                            "Waiting for Champ Select"
                        }
                        p {
                            class: "animate-in-100 relative text-xl text-green-600 font-lato font-bold animate-in-out-after-2 top-0 mx-auto",
                            "Connected!"
                        }
                        p {
                            class: "opacity-0 text-lg font-lato",
                            "..."
                        }
                    }
                }
            }
        }
    })
}

fn InGame(cx: Scope) -> Element {
    let state = use_atom_ref(&cx, GLOBAL_STATE);
    cx.render(rsx! {
        match state.read().client.status {
            KDStatus::Waiting => rsx! {
                h1 {
                    class: "text-6xl font-league text-yellow-700",
                    "Waiting for game to load..."
                }
            },
            KDStatus::InGame => {
                let queue_type = state.read().client.game.metadata.game_type.clone();
                let time_state = use_state(&cx, || String::from("--:--"));
                let time_clone = time_state.clone();
                let state_clone = state.clone();
                use_coroutine(&cx, |_: UnboundedReceiver<String>| async move {
                    loop {
                        let time_now = chrono::Local::now().timestamp();
                        let elapsed_seconds = time_now - state_clone.read().client.game.metadata.start_time;
                        let time = secs_to_string(elapsed_seconds as i16);
                        time_clone.set(time);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                });
                rsx! {
                    div {
                        class: "mx-auto text-center animate-in-50",
                        h1 {
                            class: "text-6xl font-league text-yellow-700",
                            "In Game"
                        },
                        h3 {
                            class: "text-2xl font-league text-stone-600",
                            "{queue_type}"
                        },
                        p {
                            class: "font-lato font-bold text-yellow-700",
                            "{time_state}"
                        }
                    }
                }
            },
            _ => rsx! {
                "Uh oh D:"
            }
        }
    })
}

fn ChampSelect(cx: Scope) -> Element {
    let state = use_atom_ref(&cx, GLOBAL_STATE);
    cx.render(rsx! {
        div {
            class: "w-full h-full rounded-xl bg-stone-100 p-6 pt-0",
            h1 {
                class: "pt-0 rounded-xl-bottom font-league text-yellow-700 text-9xl bg-stone-200 pb-4 mx-auto w-fit px-4 mb-6",
                "Champ Select"
            },

            match state.read().client.champ_select.metadata.phase {
                KDChampSelectPhase::Waiting => rsx! {
                    p {
                        class: "font-league text-stone-600 text-4xl",
                        "Waiting..."
                    }
                },
                KDChampSelectPhase::Pick | KDChampSelectPhase::Ban => {
                    let mut ally_picks_vec: Vec<(i16, KDChampSelectPick)> = state.read().client.champ_select.ally.picks.clone().into_iter().collect();
                    ally_picks_vec.sort_by_key(|a| a.0);
                    let mut enemy_picks_vec: Vec<(i16, KDChampSelectPick)> = state.read().client.champ_select.enemy.picks.clone().into_iter().collect();
                    enemy_picks_vec.sort_by_key(|a| a.0);
                    rsx! {
                        div {
                            class: "flex",
                            div {
                                class: "text-left font-lato text-stone-600 flex-col flex w-auto grow",
                                    ally_picks_vec.iter().map(|pick| {
                                        let mut champ_name_lowercase = pick.1.champion.name.to_lowercase();
                                        champ_name_lowercase = champ_name_lowercase.replace(['-', '\'', ' ', '.'], "");
                                        rsx!{
                                            li {
                                                class: "flex",
                                                key: "{pick.1.cell_id}",
                                                    img {
                                                        class: "rounded-full h-16 object-center object-none p-2",
                                                        src: "https://tallie.dev/kupd/assets/champions/{champ_name_lowercase}.png"
                                                    }
                                                    div {
                                                        class: "flex-col",
                                                        h3 {
                                                            class: "font-league text-6xl text-yellow-700",
                                                            "{pick.1.champion.name} {pick.1.cell_id}"
                                                        },
                                                        p {
                                                            "{pick.1.champion.title}"
                                                        }
                                                    }
                                                }
                                            }
                                        })
                            },
                            div {
                                class: "text-right font-lato text-stone-600 flex-col flex w-auto grow ml-auto",
                                    enemy_picks_vec.iter().map(|pick| {
                                        let mut champ_name_lowercase = pick.1.champion.name.to_lowercase();
                                        champ_name_lowercase = champ_name_lowercase.replace(['-', '\'', ' ', '.'], "");
                                        rsx!{
                                            li {
                                                class: "flex flex-row justify-end",
                                                key: "{pick.1.cell_id}",
                                                    div {
                                                        class: "flex-col",
                                                        h3 {
                                                            class: "font-league text-6xl text-yellow-700",
                                                            "{pick.1.champion.name} {pick.1.cell_id}"
                                                        },
                                                        p {
                                                            "{pick.1.champion.title}"
                                                        }
                                                    }
                                                img {
                                                        class: "rounded-full h-16 object-center object-none p-2",
                                                        src: "https://tallie.dev/kupd/assets/champions/{champ_name_lowercase}.png"
                                                    }
                                                }
                                            }
                                        })
                            }
                        }
                    } //END OF RSX
                },
                _ => rsx! { p {
                    "Uh oh, not sure what happened here!"
                } }
            }

            p {
                class: "font-league text-stone-600 text-4xl",
                "Not Implemented Yet!"
            }
        }
    })
}
