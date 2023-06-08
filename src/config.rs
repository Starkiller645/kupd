use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct KDConfig {
    pub api_key: Option<String>,
    pub server_url: Option<String>,
}

impl KDConfig {
    pub fn from(data: String) -> Self {
        match serde_json::from_str::<KDConfig>(data.as_str()) {
            Ok(mut config) => {
                if config.api_key == None {
                    config.api_key = Some(String::from(""));
                }
                if config.server_url == None {
                    config.server_url = Some(String::from("https://lamb.tallie.dev"));
                }
                config
            }
            Err(_) => {
                let config = KDConfig {
                    server_url: Some(String::from("https://lamb.tallie.dev")),
                    api_key: Some(String::from("")),
                };
                config
            }
        }
    }
}
