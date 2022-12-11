use serde::{Serialize, Deserialize}

struct KDConfig {
    api_key: String,
    server_url: String,
}

impl KDConfig {
    fn from(data: String) -> Option<Self> {
        match serde_json::from_str::<KDConfig>(data.as_str()) {
            Some(val) => val,
            Err() => Err("Failed!")
        }
    }
}
