use super::datatypes::*;
use super::log::*;
use super::data::*;
use std::collections::HashMap;
use futures::{channel, StreamExt, FutureExt};
use regex::*;
use reqwest as rs;
use std::convert::Infallible;
use std::default::Default;
use std::io::{Read, Write};
use std::sync::{mpsc, Arc, Mutex};
use crate::config::KDConfig;
use std::thread;
use std::time::Duration;
use serde::de::Error;
use serde::Deserialize;
use tokio_core::reactor::Core;
use warp::Filter;
use lazy_static::*;

struct Client {
    user_id: usize,
    topics: Vec<String>,
    sender: Option<channel::mpsc::UnboundedSender<std::result::Result<warp::ws::Message, warp::Error>>>,
}

lazy_static! {
    static ref QUEUE_MAP: HashMap<i16, &'static str> = HashMap::from([
        (0, "None"),
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

impl Client {
	fn send(&self, data: String) -> Result<String, String> {
		match self.sender.clone() {
			Some(sender) => {
				match sender.unbounded_send(Ok(warp::ws::Message::text(data.clone()))) {
					Ok(_) => Ok(data),
					_ => Err("Failed sending message".to_string())
				}
			},
			None => {
				Err("Couldn't send message!".to_string())
			}
		}
	}
}

fn load_champions() -> Result<HashMap<i16, KDChampion>, String> {
	let champ_data_vec: Vec<&str> = CHAMP_DATA.split(";").collect();
	let mut final_hashmap: HashMap<i16, KDChampion> = HashMap::new();

	for champion in champ_data_vec {
		let vec_1: Vec<&str> = champion.split(":").collect();
		let name = String::from(vec_1[0]);
		let vec_2: Vec<&str> = vec_1[1].split("/").collect();
		let title = String::from(vec_2[0]);
		let vec_3: Vec<&str> = vec_2[1].split("-").collect();
		let class = match vec_3[0] {
			"mg" => KDChampionClass::Mage,
			"fi" => KDChampionClass::Fighter,
			"sp" => KDChampionClass::Specialist,
			"mk" => KDChampionClass::Marksman,
			"as" => KDChampionClass::Assassin,
			"tk" => KDChampionClass::Tank,
			"ct" => KDChampionClass::Controller,
			_ => KDChampionClass::Unknown
		};
		let id: i16 = match vec_3[1].parse() {
			Ok(id) => id,
			Err(_) => return Err(String::from("Error parsing champions file!"))
		};
		let champion_data = KDChampion {
			name,
			title,
			id,
			class
		};
		final_hashmap.insert(id, champion_data);
	}

	Ok(final_hashmap)
}

async fn get_game_metadata(client: reqwest::Client, queue_id: i16) -> Result<KDGameMetadata, ()> {
    let res = match client.get("https://127.0.0.1:2999/liveclientdata/gamestats")
        .send()
        .await {
            Ok(res) => res,
            Err(_) => return Err(())
        };
    match res.status().is_success() {
        true => {},
        false => return Err(())
    };
    let res_json: serde_json::Value = serde_json::from_str(res.text().await.unwrap().as_str()).unwrap();
    let time_now = chrono::Local::now();
    let game_seconds: i64 = res_json["gameTime"].as_f64().unwrap() as i64;
    let start_time = time_now - chrono::Duration::seconds(game_seconds);
    let start_time_seconds = start_time.timestamp() as i64;

    Ok(KDGameMetadata {
        start_time: start_time_seconds,
        game_type: String::from(QUEUE_MAP[&queue_id])
    })
}

fn map_champ_select(data: String) -> Result<KDChampSelect, serde_json::Error> {
	
	let champs = match load_champions() {
		Ok(champs) => champs,
		Err(_) => return Err(serde_json::Error::custom("Error in loading champions"))
	};

    let json: serde_json::Value = serde_json::from_str(data.as_str())?;
	let mut actions: Vec<riot::CSAction> = vec![];

	for action_list in json["actions"].as_array() {
		for action_list_2 in action_list {
            for action_list_3 in action_list_2.as_array() {
                for action in action_list_3 {
                    actions.push(Deserialize::deserialize(action).unwrap());
                }
            }
		}
	};

	let mut ally_team: HashMap<i16, KDChampSelectPick> = HashMap::new();
	let mut enemy_team: HashMap<i16, KDChampSelectPick> = HashMap::new();

	let mut ally_bans: HashMap<i16, KDChampSelectPick> = HashMap::new();
	let mut enemy_bans: HashMap<i16, KDChampSelectPick> = HashMap::new();

	for action in actions {
        if action.cell_id < 0 {
            continue;
        };
		match action.action_type {
			riot::CSActionType::Pick => {
                if action.champion_id < 0 {continue;};
				let champion = champs[&action.champion_id].clone();
				let pick = KDChampSelectPick {
					cell_id: action.cell_id as i8,
					position: KDPosition::Bottom,
					champion,
					r#type: KDPickType::Pick,
					hover: action.in_progress,
					locked: action.completed
				};
				if action.is_ally {
					ally_team.insert(action.cell_id, pick)
				} else {
					enemy_team.insert(action.cell_id, pick)
				};
			},
			riot::CSActionType::Ban => {
                if action.champion_id < 0 {continue;};
				let champion = champs[&action.champion_id].clone();
				let pick = KDChampSelectPick {
					cell_id: action.cell_id as i8,
					position: KDPosition::Bottom,
					champion,
					r#type: KDPickType::Ban,
					hover: action.in_progress,
					locked: action.completed
				};
				if action.is_ally {
					ally_bans.insert(action.cell_id, pick)
				} else {
					enemy_bans.insert(action.cell_id, pick)
				};
			},
            _ => {}
		}
	}

	let timer: riot::CSTimer = Deserialize::deserialize(json["timer"].clone()).unwrap();
	let phase: KDChampSelectPhase = match timer.phase {
		_ => KDChampSelectPhase::Pick,
	};
	let total_time: i16 = (timer.total_time_ms / 1000) as i16;
	let time_left: i16 = (timer.time_left_ms / 1000) as i16;

	let champ_select = KDChampSelect {
		ally: KDChampSelectTeam {
			picks: ally_team,
			bans: ally_bans
		},
		enemy: KDChampSelectTeam {
			picks: enemy_team,
			bans: enemy_bans
		},
		metadata: KDChampSelectMetadata {
			phase,
			time: time_left,
			phase_duration: total_time
		}
	};

    Ok(champ_select)
}

type Clients = Arc<Mutex<Vec<Client>>>;

async fn yellow_thread(mut rx: channel::mpsc::UnboundedReceiver<LBMessage>, config: KDConfig, state: Arc<Mutex<KDData>>) {
    let y = Some(LogFrom::Yellow);
    log("Initialising...", y).unwrap();
    let clients: Clients = Arc::new(Mutex::new(vec![]));
    let mut queue_id: i16 = 0;
    log("Initialising webserver...", y).unwrap();
    let path = warp::path!("ws")
        .and(warp::ws())
        .and(with_state(state.clone()))
        .and(with_clients(clients.clone()))
        .and_then(handle_connection);

	tokio::spawn(async move {
        log("Webserver initialised!", y).unwrap();
		warp::serve(path).run(([127, 0, 0, 1], 8000)).await;
	});
    //let ws_handle = launch_webserver(state.clone(), clients.clone());
    log("Done!", y).unwrap();
    loop {
		match rx.try_next() {
			Ok(message) => {
				match message {
					Some(message) => {
						match message.ident.as_str() {
                            "live/champ-select" | "live/champ-select/update" => {
                                state.lock().unwrap().client.status = KDStatus::ChampSelect;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
                            "live/end" => {
                                state.lock().unwrap().client.status = KDStatus::Connected;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;                       
                            }
                            "live/disconnect" => {
                                state.lock().unwrap().client.status = KDStatus::Disconnected;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
                            "live/connect" => {
                                state.lock().unwrap().client.status = KDStatus::Connected;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
                            "live/queue" => {
                                state.lock().unwrap().client.status = KDStatus::InQueue;
                                state.lock().unwrap().client.queue = serde_json::from_str(&message.data).unwrap();
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
                            "live/await" => {
                                state.lock().unwrap().client.status = KDStatus::Waiting;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
                            "live/metadata" => {
                                state.lock().unwrap().client.status = KDStatus::InGame;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
							"meta/shutdown" => {
                                log("Shutting down thread...", y).unwrap();
								break;
							}
							_ => log(format!("Got an unknown message :O ({})", message.ident.as_str()).as_str(), y).unwrap(),
						}
					},
					None => {}
				}
			},
			Err(_) => {
			}
		}
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

/*async fn launch_webserver(state: Arc<Mutex<KDData>>, clients: Clients) -> tokio::task::JoinHandle<()> {
}*/

async fn broadcast_on(clients: Clients, data: String) {
	clients.lock()
		.unwrap()
		.iter_mut()
		.for_each(|client| {
			let _ = client.send(data.clone());
		})
}

fn with_state(
    state: Arc<Mutex<KDData>>,
) -> impl Filter<Extract = (Arc<Mutex<KDData>>,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}
async fn handle_connection(ws: warp::ws::Ws, state: Arc<Mutex<KDData>>, clients: Clients) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |socket| handle_request(socket, state, clients)))
}

async fn handle_request(ws: warp::ws::WebSocket, state: Arc<Mutex<KDData>>, clients: Clients) {
    let (client_ws_tx, mut client_ws_rx) = ws.split();
    let (client_tx, client_rx) = futures::channel::mpsc::unbounded();

    let mut message: LBMessage = Default::default();
    message.r#type = LBMessageType::Update;
    message.data = String::from("{}");

    log("Sending 'welcome' message to new client with basic state detail", Some(LogFrom::Yellow)).unwrap();
    let state_lock = state.lock();
    let client_status = state_lock.as_ref().unwrap().client.status;
    drop(state_lock);

    match client_status {
        KDStatus::Disconnected => message.ident = String::from("live/disconnect"),
        KDStatus::Connected => message.ident = String::from("live/connect"),
        KDStatus::ChampSelect => message.ident = String::from("live/champ-select"),
        KDStatus::InQueue => {
            message.ident = String::from("live/queue");
            message.data = serde_json::to_string(&state.clone().lock().unwrap().client.queue).unwrap();
        }
        _ => {}
    }

    tokio::task::spawn(client_rx.forward(client_ws_tx).map(|res| {
        if let Err(_) = res {
            log("Client disconnected, dropping...", Some(LogFrom::Yellow)).unwrap();
        }
    }));

    client_tx.unbounded_send(Ok(warp::ws::Message::text(serde_json::to_string(&message).unwrap().as_str()))).unwrap();
    drop(message);
    clients.lock().unwrap().push(Client {
        sender: Some(client_tx),
        topics: vec!["".to_string()],
        user_id: 0
    });
}

async fn blue_thread(mut rx: channel::mpsc::UnboundedReceiver<LBMessage>, config: KDConfig, state: Arc<Mutex<KDData>>) {
    let b = Some(LogFrom::Blue);
    let base_url: String = config.server_url.unwrap();
    log("Initialising...", b).unwrap();
    log("Checking pingback response from LAMB server...", b).unwrap();
    log_additional(format!("{}/pingback", base_url).as_str()).unwrap();
    let clients: Clients = Arc::new(Mutex::new(vec![]));
    let http_client = rs::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    log("Ready!", b).unwrap();
    match http_client.get(format!("{}/pingback", base_url))
        .send()
        .await {
            Ok(res) => match res.status().is_success() {
                true => log("LAMB server is up, connection established", b).unwrap(),
                false => {
                    log_warning("Got an error response from LAMB server", b, LogSeverity::Warning).unwrap();
                    log_additional("running in offline mode, shutting down blue thread...").unwrap();
                }
            },
            Err(_) => {
                log_warning("Couldn't connect to LAMB server", b, LogSeverity::Warning).unwrap();
                log_additional("running in offline mode, shutting down blue thread...").unwrap();
            }
        }
    loop {
        match rx.try_next() {
            Ok(message) => {
                let message = message.unwrap();
                match message.ident.as_str() {
                    "meta/shutdown" => {
                        log("Shutting down thread...", b).unwrap()
                    }
                    "live/metadata" => {
                        state.lock().unwrap().client.status = KDStatus::InGame;
                        http_client.post(base_url.clone() + "/live")
                            .body("{\"event\": \"start\"}")
                            .send()
                            .await;
                    },
                    _ => log("Message not handled :O", b).unwrap()
                }
            }
            Err(_) => {}
        }
    }
}

pub async fn green_thread(mut rx: channel::mpsc::Receiver<LBMessage>, mut config: KDConfig) {
    let g = Some(LogFrom::Green);
    log("Initialising...", g).unwrap();
    let mut state: KDData = Default::default();
    let (yellow_tx, y_rx) = channel::mpsc::unbounded();
    let mut shared_state = Arc::new(Mutex::new(Default::default()));
    let mut queue_id: i16 = 0;
    let conf = config.clone();
    let y_state = shared_state.clone();
    let y_thread = tokio::spawn(async move {
        log("Starting up yellow thread...", g).unwrap();
        yellow_thread(y_rx, conf, y_state).await;
    });

    let (blue_tx, b_rx) = channel::mpsc::unbounded();
    let conf = config.clone();
    let b_state = shared_state.clone();
    let b_thread = tokio::spawn(async move {
        log("Starting up blue thread...", g).unwrap();
        blue_thread(b_rx, conf, b_state).await;
    });
let mut lcu_base_url = String::new();
    let mut lcu_key = String::new();

    let http_client = rs::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    log("Ready!", g).unwrap();

    let mut result_cache = String::new();
    let mut i = 0;

    loop {
        match rx.try_next() {
            Ok(message) => {
				let message = message.unwrap();
				match message.ident.as_str() {
					"meta/shutdown" => {
						let return_message = LBMessage {
							ident: String::from("meta/shutdown"),
							r#type: LBMessageType::Update,
							data: String::from("{}"),
						};
						let return_message_clone = return_message.clone();
						yellow_tx.unbounded_send(return_message).unwrap();
						blue_tx.unbounded_send(return_message_clone).unwrap();
						y_thread.await.unwrap();
						b_thread.await.unwrap();
						break;
					}
					_ => {
						yellow_tx.unbounded_send(message.clone()).unwrap();
						blue_tx.unbounded_send(message.clone()).unwrap();
					}
				}
			},
			Err(_) => {}
        }

        match state.client.status {
            KDStatus::Disconnected => {
                #[cfg(target_os = "windows")]
                {
                    panic!("ERROR: windows support not yet implemented!");
                }

                {
                    let pscmd = std::process::Command::new("ps")
                        .args(["-ef", "ww"])
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                        .unwrap();
                    let grepcmd = std::process::Command::new("grep")
                        .stdin(std::process::Stdio::from(pscmd.stdout.unwrap()))
                        .arg("LeagueClientUx")
                        .output()
                        .expect("Couldn't spawn grep process");

                    let grep_output = String::from_utf8(grepcmd.stdout).unwrap();
                    let port_re = Regex::new(r"--app-port=([0-9]+)").unwrap();
                    let port_match = match port_re.find(grep_output.as_str()) {
                        Some(m) => m.as_str(),
                        None => {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                    };
                    let port: u16 =
                        match port_match.split("=").collect::<Vec<&str>>()[1].parse::<u16>() {
                            Ok(p) => p,
                            Err(_) => {
                                log("Error decoding LC cmdline args", g).unwrap();
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                continue;
                            }
                        };
                    let app_key_re =
                        Regex::new(r"--remoting-auth-token=([a-zA-Z0-9'-_]+)").unwrap();
                    let app_key_match = match app_key_re.find(grep_output.as_str()) {
                        Some(m) => m.as_str(),
                        None => {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                    };
                    let app_key_split: Vec<&str> = app_key_match.split("=").collect();
                    let app_key: String;
                    if app_key_split.len() > 1 {
                        app_key = String::from(app_key_split[1]);
                    } else {
                        app_key = String::new();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    lcu_base_url = String::from(format!("https://127.0.0.1:{}", port));
                    lcu_key = String::from(app_key);

                    let secret = base64::encode(format!("riot:{}", lcu_key));
                    let auth_header = format!("Basic {}", secret);
                    let res = match http_client
                        .get(format!("{}/lol-gameflow/v1/gameflow-phase", lcu_base_url))
                        .header("Authorization", auth_header.as_str())
                        .send()
                        .await {
                            Ok(res) => {
                                res
                            },
                            Err(_) => {
                                state.client.status = KDStatus::Disconnected;
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                continue;
                            }
                        };
                    match res.status().is_success() {
                        true => {
                            log(format!("Found the League Client on {}", lcu_base_url.clone()).as_str(), g).unwrap();
                            log_additional(format!("with authentication riot:{}", lcu_key.clone()).as_str()).unwrap();
                            let res_text = res.text().await.unwrap();
                            match res_text.as_str() {
                                "\"InProgress\"" => {
                                    log("We're already in a game!", g).unwrap();
                                    let game_metadata = get_game_metadata(http_client.clone(), queue_id).await.unwrap();
                                    let y_message = LBMessage {
                                        ident: String::from("live/metadata"),
                                        r#type: LBMessageType::Broadcast,
                                        data: String::from(serde_json::to_string(&game_metadata).unwrap())
                                    };
                                    yellow_tx.unbounded_send(y_message).unwrap();
                                    state.client.status = KDStatus::InGame;
                                },
                                "\"ChampSelect\"" => {
                                    log("We're already in Champ Select!", g).unwrap();
                                    state.client.status = KDStatus::ChampSelect;
                                    let y_message = LBMessage {
                                        ident: String::from("live/champ-select"),
                                        r#type: LBMessageType::Broadcast,
                                        data: String::from("{}")
                                    };
                                    yellow_tx.unbounded_send(y_message).unwrap();
                                }
                                "\"Matchmaking\"" => {
                                    log("We're already in queue", g).unwrap();
                                    state.client.status = KDStatus::InQueue;
                                }
                                _ => {
                                    state.client.status = KDStatus::Connected;
                                }
                            }
                            if state.client.status == KDStatus::Connected {
                                let y_message = LBMessage {
                                    ident: String::from("live/connect"),
                                    r#type: LBMessageType::Broadcast,
                                    data: String::from("{}")
                                };
                                yellow_tx.unbounded_send(y_message).unwrap();
                            }
                        }
                        false => {
                        }
                    }
                }
            }
            KDStatus::Connected  => {
                //println!("[GREEN] Attempting connection to champ select...");
                //println!("        -> At url: {}/lol-champ-select/v1/session", lcu_base_url);

                let secret = base64::encode(format!("riot:{}", lcu_key.clone()));
                let auth_header = format!("Basic {}", secret);
                
                let queue_res = match http_client
                    .get(format!("{}/lol-champ-select/v1/session", lcu_base_url))
                    .header("Authorization", auth_header.as_str())
                    .send()
                    .await {
                        Ok(response) => response,
                        Err(_) => {
                                    log("Uh oh, lost connection to League. Setting state as disconnected!", g).unwrap();
                                    state.client.status = KDStatus::Disconnected;
                                    let y_message = LBMessage {
                                        ident: String::from("live/disconnect"),
                                        r#type: LBMessageType::Broadcast,
                                        data: String::from("{}")
                                    };
                                    yellow_tx.unbounded_send(y_message).unwrap();
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                    continue;
                                }
                        };
                        match queue_res.status().is_success() {
                            false => {
                                let res = match http_client
                                    .get(format!("{}/lol-matchmaking/v1/search", lcu_base_url))
                                    .header("Authorization", auth_header.as_str())
                                    .send()
                                    .await {
                                    Ok(response) => response,
                                    Err(_err) => {
                                        log("Uh oh, lost connection to League. Setting state as disconnected!", g).unwrap();
                                        state.client.status = KDStatus::Disconnected;
                                        let y_message = LBMessage {
                                            ident: String::from("live/disconnect"),
                                            r#type: LBMessageType::Broadcast,
                                            data: String::from("{}")
                                        };
                                        yellow_tx.unbounded_send(y_message).unwrap();
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        continue;
                                    }
                                };
                                match res.status().is_success() {
                                    true => {
                                        state.client.status = KDStatus::InQueue;
                                        let res_json: serde_json::Value = serde_json::from_str(res.text().await.unwrap().as_str()).unwrap();

                                        let start_time: i64 = chrono::Local::now().timestamp() - res_json["timeInQueue"].as_f64().unwrap() as i64;

                                        queue_id = res_json["queueId"].as_i64().unwrap() as i16;

                                        let data = KDQueue {
                                            time: res_json["timeInQueue"].as_f64().unwrap() as i16,
                                            expected_time: res_json["estimatedQueueTime"].as_f64().unwrap() as i16,
                                            id: res_json["queueId"].as_i64().unwrap() as i16,
                                            start_time
                                        };
                                        log(format!("Now in queue {}!", data.id).as_str(), Some(LogFrom::Green)).unwrap();
                                        log_additional(format!("from timestamp {}", data.start_time).as_str()).unwrap();

                                        let y_message = LBMessage {
                                            ident: String::from("live/queue"),
                                            r#type: LBMessageType::Broadcast,
                                            data: serde_json::to_string(&data).unwrap()
                                        };
                                        yellow_tx.unbounded_send(y_message).unwrap();
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                    }
                                    false => {
                                        //println!("[GREEN] Request complete!");
                                        //println!("        -> Got error response: {:?}", res);
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        continue;
                                    }
                                }
                            },
                            true => {
                                state.client.status = KDStatus::ChampSelect;
                                let y_message = LBMessage {
                                    ident: String::from("live/champ-select"),
                                    r#type: LBMessageType::Broadcast,
                                    data: String::from("{}")
                                };
                                yellow_tx.unbounded_send(y_message).unwrap();
                                continue;
                            }
                    };
                
            },
            KDStatus::InQueue => {
                let secret = base64::encode(format!("riot:{}", lcu_key.clone()));
                let auth_header = format!("Basic {}", secret);
                let queue_res = match http_client
                    .get(format!("{}/lol-matchmaking/v1/search", lcu_base_url))
                    .header("Authorization", auth_header.as_str())
                    .send()
                    .await {
                        Ok(response) => response,
                        Err(_) => {
                                    log("Uh oh, lost connection to League. Setting state as disconnected!", g).unwrap();
                                    state.client.status = KDStatus::Disconnected;
                                    let y_message = LBMessage {
                                        ident: String::from("live/disconnect"),
                                        r#type: LBMessageType::Broadcast,
                                        data: String::from("{}")
                                    };
                                    yellow_tx.unbounded_send(y_message).unwrap();
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                    continue;
                                }
                        };
                        match queue_res.status().is_success() {
                            true => {
                            },
                            false => {
                                log("Dropped out of queue", g).unwrap();
                                state.client.status = KDStatus::Connected;
                                let y_message = LBMessage {
                                    ident: String::from("live/connect"),
                                    r#type: LBMessageType::Broadcast,
                                    data: String::from("{}")
                                };
                                yellow_tx.unbounded_send(y_message).unwrap();
                                continue;
                            }
                        };
                        let res = match http_client
                            .get(format!("{}/lol-champ-select/v1/session", lcu_base_url))
                            .header("Authorization", auth_header.as_str())
                            .send()
                            .await {
                            Ok(response) => response,
                            Err(_err) => {
                                log("Uh oh, lost connection to League. Setting state as disconnected!", g).unwrap();
                                state.client.status = KDStatus::Disconnected;
                                let y_message = LBMessage {
                                    ident: String::from("live/disconnect"),
                                    r#type: LBMessageType::Broadcast,
                                    data: String::from("{}")
                                };
                                yellow_tx.unbounded_send(y_message).unwrap();
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                continue;
                            }
                        };
                        match res.status().is_success() {
                            true => {
                                log("Switching over to Champ Select...", g).unwrap();
                                state.client.status = KDStatus::ChampSelect;
                                let y_message = LBMessage {
                                    ident: String::from("live/champ-select"),
                                    r#type: LBMessageType::Broadcast,
                                    data: String::from("{}")
                                };
                                yellow_tx.unbounded_send(y_message).unwrap();
                                continue;
                            }
                            false => {
                                //println!("[GREEN] Request complete!");
                                //println!("        -> Got error response: {:?}", res);
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                continue;
                            }
                        }
            }
            KDStatus::ChampSelect => {
                let secret = base64::encode(format!("riot:{}", lcu_key.clone()));
                let auth_header = format!("Basic {}", secret);
                let res = match http_client
                    .get(format!("{}/lol-champ-select/v1/session", lcu_base_url))
                    .header("Authorization", auth_header.as_str())
                    .send()
                    .await {
                    Ok(response) => response,
                    Err(err) => {
                        state.client.status = KDStatus::Connected;
                        let y_message = LBMessage {
                            ident: String::from("live/end"),
                            r#type: LBMessageType::Broadcast,
                            data: String::from("{}")
                        };
                        yellow_tx.unbounded_send(y_message).unwrap();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                };
                let mut result = String::new();
                match res.status().is_success() {
                    true => {
                        result = res.text().await.unwrap();
                    }
                    false => {
                        //println!("[GREEN] Request complete!");
                        //println!("        -> Got error response: {:?}", res);
                        // Check if we're still in queue
                        let res = match http_client
                            .get(format!("{}/lol-matchmaking/v1/search", lcu_base_url))
                            .header("Authorization", auth_header.as_str())
                            .send()
                            .await {
                            Ok(response) => response,
                            Err(err) => {
                                state.client.status = KDStatus::Connected;
                                let y_message = LBMessage {
                                    ident: String::from("live/end"),
                                    r#type: LBMessageType::Broadcast,
                                    data: String::from("{}")
                                };
                                yellow_tx.unbounded_send(y_message).unwrap();
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                continue;
                            }
                        };
                        match res.status().is_success() {
                           true => {
                               state.client.status = KDStatus::InQueue;
                               continue;
                           },
                           false => {
                               log("Champ select ended, waiting to see if we're in a game...", g).unwrap();
                               state.client.status = KDStatus::Waiting;
                               continue;
                           }
                        }
                    }
                }

				let data = map_champ_select(result.clone()).unwrap();

                if result_cache != result {
                    let y_message = LBMessage {
                        ident: String::from("live/champ-select/update"),
                        r#type: LBMessageType::Broadcast,
                        data: serde_json::to_string(&data).unwrap()
                    };
                    yellow_tx.unbounded_send(y_message).unwrap();
                }

				tokio::time::sleep(Duration::from_millis(500)).await;

            }
            KDStatus::Waiting => {
                let y_message = LBMessage {
                    ident: String::from("live/await"),
                    r#type: LBMessageType::Broadcast,
                    data: String::from("{}")
                };
                let secret = base64::encode(format!("riot:{}", lcu_key));
                let auth_header = format!("Basic {}", secret);
                yellow_tx.unbounded_send(y_message).unwrap();
                let mut skip = false;
                loop {
                    let res = match http_client
                        .get(format!("{}/lol-gameflow/v1/gameflow-phase", lcu_base_url))
                        .header("Authorization", auth_header.as_str())
                        .send()
                        .await {
                            Ok(res) => res,
                            Err(_) => {
                                tokio::time::sleep(Duration::from_millis(2000)).await;
                                log("Error connecting...", g).unwrap();
                                break;
                            }
                        };
                    let res_text = res.text().await.unwrap();
                    log(res_text.as_str(), g).unwrap();
                    match res_text.as_str() {
                        "\"InProgress\"" => {
                            state.client.status = KDStatus::InGame;
                            log("Loading into a game now...", g).unwrap();
                            log("Connected to game client, yay!", g).unwrap();
                            log_additional("on https://127.0.0.1:2999").unwrap();
                            let game_metadata = match get_game_metadata(http_client.clone(), queue_id).await {
                                Ok(metadata) => metadata,
                                Err(_) => {
                                    log("Couldn't connect to the client atm, trying again...", g).unwrap();
                                    tokio::time::sleep(Duration::from_millis(2000)).await;
                                    continue;
                                }
                            };
                            let y_message = LBMessage {
                                ident: String::from("live/metadata"),
                                r#type: LBMessageType::Broadcast,
                                data: String::from(serde_json::to_string(&game_metadata).unwrap())
                            };
                            yellow_tx.unbounded_send(y_message).unwrap();
                            break;
                        },
                        "\"ChampSelect\"" => {
                            tokio::time::sleep(Duration::from_millis(2000)).await;
                            continue;
                        }
                        _ => {
                            state.client.status = KDStatus::Connected;
                            break;
                        }
                    }
                }
            }
            KDStatus::InGame => {
                let res = match http_client
                    .get("https://127.0.0.1:2999/liveclientdata/playerlist")
                    .send()
                    .await {
                        Ok(res) => res,
                        Err(_) => {
                            tokio::time::sleep(Duration::from_millis(2000)).await;
                            log("Game over, disconnecting...", g).unwrap();
                            state.client.status = KDStatus::Connected;
                            let y_message = LBMessage {
                                ident: String::from("live/connect"),
                                r#type: LBMessageType::Broadcast,
                                data: String::from("{}")
                            };
                            yellow_tx.unbounded_send(y_message).unwrap();
                            state.client.status = KDStatus::Connected;
                            continue;
                        }
                    };
            }
            _ => {}
        }
    }
}
