use serde::{Deserialize, Serialize};

// Lamb Bus types
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LBMessage {
    pub ident: String,
    pub r#type: LBMessageType,
    pub data: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum LBMessageType {
    #[default]
    #[serde(alias = "request")]
    Request,
    #[serde(alias = "update")]
    Update,
    #[serde(alias = "broadcast")]
    Broadcast,
}

// KD base types
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDData {
    pub metadata: KDClientMetadata,
    debug: Option<String>,
    pub client: KDClient,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDClientMetadata {
    exit: bool,
    pub has_connected: bool,
    messages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDQueue {
    pub time: i16,
    pub expected_time: i16,
    pub id: i16,
    pub start_time: i64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDClient {
    pub status: KDStatus,
    pub queue: KDQueue,
    pub champ_select: KDChampSelect,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDChampSelect {
    metadata: KDChampSelectMetadata,
    pub ally: KDChampSelectTeam,
    pub enemy: KDChampSelectTeam,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDGame {}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDChampSelectMetadata {
    phase: String,
    time: u32,
    queue: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum KDLivePhase {
    #[default]
    #[serde(alias = "waiting")]
    Waiting,
    #[serde(alias = "champ-select")]
    ChampSelect,
    #[serde(alias = "loading")]
    Loading,
    #[serde(alias = "ingame")]
    Ingame,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Copy, Clone)]
pub enum KDStatus {
    #[serde(alias = "con")]
    Connected,
    #[default]
    #[serde(alias = "dsc")]
    Disconnected,
    #[serde(alias = "csl")]
    ChampSelect,
    #[serde(alias = "gme")]
    InGame,
    #[serde(alias = "end")]
    PostGame,
    #[serde(alias = "wtn")]
    Waiting,
    #[serde(alias = "que")]
    InQueue,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum KDPosition {
    #[default]
    #[serde(alias = "top")]
    Top,
    #[serde(alias = "jgl")]
    Jungle,
    #[serde(alias = "mid")]
    Middle,
    #[serde(alias = "bot")]
    Bottom,
    #[serde(alias = "sup")]
    Support,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum KDPickType {
    #[serde(alias = "pick")]
    Pick,
    #[default]
    #[serde(alias = "ban")]
    Ban,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDChampSelectPick {
    #[serde(alias = "cellID")]
    cell_id: i8,
    position: KDPosition,
    #[serde(alias = "championName")]
    pub champion_name: String,
    r#type: KDPickType,
    hover: bool,
    locked: bool,
}

// KDChampSelect
pub type KDChampSelectTeam = [KDChampSelectPick; 5];

// All struct definitions below this are Riot DTO objects
pub mod riot {}
