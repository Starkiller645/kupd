use super::datatypes::*;
use super::log::*;
use futures::{channel, StreamExt, FutureExt};
use regex::*;
use reqwest as rs;
use std::convert::Infallible;
use std::default::Default;
use std::io::{Read, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio_core::reactor::Core;
use warp::Filter;

struct Client {
    user_id: usize,
    topics: Vec<String>,
    sender: Option<channel::mpsc::UnboundedSender<std::result::Result<warp::ws::Message, warp::Error>>>,
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

type Clients = Arc<Mutex<Vec<Client>>>;

async fn yellow_thread(mut rx: channel::mpsc::UnboundedReceiver<LBMessage>) {
    let y = Some(LogFrom::Yellow);
    log("Initialising...", y).unwrap();
    let state: Arc<Mutex<KDData>> = Arc::new(Mutex::new(Default::default()));
    let clients: Clients = Arc::new(Mutex::new(vec![]));
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
                            "live/champ-select" => {
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
                                log("Sending queue message to clients...", y).unwrap();
                                state.lock().unwrap().client.status = KDStatus::InQueue;
                                broadcast_on(clients.clone(), serde_json::to_string(&message).unwrap()).await;
                            }
							"meta/shutdown" => {
                                log("Shutting down thread...", y).unwrap();
								break;
							}
							_ => log("Message not handled :O", y).unwrap(),
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

    match state.lock().unwrap().client.status {
        KDStatus::Disconnected => message.ident = String::from("live/disconnect"),
        KDStatus::Connected => message.ident = String::from("live/connect"),
        KDStatus::ChampSelect => message.ident = String::from("live/champ-select"),
        _ => {}
    }

    tokio::task::spawn(client_rx.forward(client_ws_tx).map(|res| {
        if let Err(_) = res {
            log("Client disconnected, dropping...", Some(LogFrom::Yellow)).unwrap();
        }
    }));

    log("Sending 'welcome' message to new client with basic state detail", Some(LogFrom::Yellow)).unwrap();
    client_tx.unbounded_send(Ok(warp::ws::Message::text(serde_json::to_string(&message).unwrap().as_str()))).unwrap();

    clients.lock().unwrap().push(Client {
        sender: Some(client_tx),
        topics: vec!["".to_string()],
        user_id: 0
    });
}

async fn blue_thread(rx: mpsc::Receiver<LBMessage>) {
    let b = Some(LogFrom::Blue);
    log("Initialising...", b).unwrap();
    let state: Arc<Mutex<KDData>> = Arc::new(Mutex::new(Default::default()));
    let clients: Clients = Arc::new(Mutex::new(vec![]));
    log("Ready!", b).unwrap();
    for message in rx {
        match message.ident.as_str() {
            "live/champ-select" => {},
            "meta/shutdown" => {
                log("Shutting down thread...", b).unwrap();
                break;
            }
            _ => log("Message not handled :O", b).unwrap(),
        }
    }
}

pub async fn green_thread(mut rx: channel::mpsc::Receiver<LBMessage>) {
    let g = Some(LogFrom::Green);
    log("Initialising...", g).unwrap();
    let mut state: KDData = Default::default();
    let (yellow_tx, y_rx) = channel::mpsc::unbounded();
    let y_thread = tokio::spawn(async move {
        log("Starting up yellow thread...", g).unwrap();
        yellow_thread(y_rx).await;
    });

    let (blue_tx, b_rx) = mpsc::channel();
    let b_thread = tokio::spawn(async move {
        log("Starting up blue thread...", g).unwrap();
        blue_thread(b_rx).await;
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
						blue_tx.send(return_message_clone).unwrap();
						y_thread.await.unwrap();
						b_thread.await.unwrap();
						break;
					}
					_ => {
						yellow_tx.unbounded_send(message.clone()).unwrap();
						blue_tx.send(message.clone()).unwrap();
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
                        .get(format!("{}/lol-login/v1/session", lcu_base_url))
                        .header("Authorization", auth_header.as_str())
                        .send()
						.await
                    {
                        Ok(response) => response,
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
                            state.client.status = KDStatus::Connected;
                            let y_message = LBMessage {
                                ident: String::from("live/connect"),
                                r#type: LBMessageType::Broadcast,
                                data: String::from("{}")
                            };
                            yellow_tx.unbounded_send(y_message).unwrap();
                        }
                        false => {
                            state.client.status = KDStatus::Connected;
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
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
                            false => {
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
                                        state.client.status = KDStatus::ChampSelect;
                                    }
                                    false => {
                                        //println!("[GREEN] Request complete!");
                                        //println!("        -> Got error response: {:?}", res);
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        continue;
                                    }
                                }
                                continue;
                            },
                            true => {
                                state.client.status = KDStatus::InQueue;
                                
                                let res_json: serde_json::Value = serde_json::from_str(queue_res.text().await.unwrap().as_str()).unwrap();

                                let start_time: i64 = chrono::Local::now().timestamp() - res_json["timeInQueue"].as_f64().unwrap() as i64;

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
                                        state.client.status = KDStatus::ChampSelect;
                                    }
                                    false => {
                                        //println!("[GREEN] Request complete!");
                                        //println!("        -> Got error response: {:?}", res);
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        continue;
                                    }
                                }
                                continue;
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
                }
                let result_json: serde_json::Value = serde_json::from_str(result.as_str()).unwrap();

                if result_cache != result {
                    let y_message = LBMessage {
                        ident: String::from("live/champ-select"),
                        r#type: LBMessageType::Broadcast,
                        data: String::from(result)
                    };
                    yellow_tx.unbounded_send(y_message).unwrap();
                }

				tokio::time::sleep(Duration::from_millis(500)).await;

            }
            _ => {}
        }
    }
}
