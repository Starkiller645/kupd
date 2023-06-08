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
    pub lamb_url: String,
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
    pub game: KDGame,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDChampSelect {
    pub metadata: KDChampSelectMetadata,
    pub ally: KDChampSelectTeam,
    pub enemy: KDChampSelectTeam,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDGame {
    pub ally: KDTeam,
    pub enemy: KDTeam,
    pub metadata: KDGameMetadata,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KDGameMetadata {
    pub game_type: String,
    pub start_time: i64,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDChampSelectMetadata {
    pub phase: KDChampSelectPhase,
    pub time: i16,
    pub phase_duration: i16,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum KDChampSelectPhase {
    #[default]
    Hover,
    Ban,
    Pick,
    Loadout,
    Waiting,
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum KDPickType {
    #[serde(alias = "pick")]
    Pick,
    #[default]
    #[serde(alias = "ban")]
    Ban,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDChampSelectPick {
    #[serde(alias = "cellID")]
    pub cell_id: i8,
    pub position: KDPosition,
    pub champion: KDChampion,
    pub r#type: KDPickType,
    pub hover: bool,
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDChampion {
    pub name: String,
    pub title: String,
    pub id: i16,
    pub class: KDChampionClass,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDSummoner {
    pub kills: i16,
    pub deaths: i16,
    pub assists: i16,
    pub champion: KDChampion,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDTeam {
    pub team: KDGameTeam,
    pub kills: i16,
    pub objectives: Vec<KDObjective>,
    pub buffs: Vec<KDObjectiveBuff>,
    pub summoners: Vec<KDSummoner>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDObjective {}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDObjectiveBuff {}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum KDGameTeam {
    #[serde(alias = "ORDER")]
    #[default]
    Order,
    #[serde(alias = "CHAOS")]
    Chaos,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum KDChampionClass {
    #[default]
    Unknown,
    Fighter,
    Controller,
    Mage,
    Marksman,
    Assassin,
    Tank,
    Specialist,
}

// KDChampSelect
use std::collections::HashMap;
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KDChampSelectTeam {
    pub picks: HashMap<i16, KDChampSelectPick>,
    pub bans: HashMap<i16, KDChampSelectPick>,
}

// All struct definitions below this are Riot DTO objects
pub mod riot {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize, Debug, Default)]
    pub struct CSAction {
        #[serde(alias = "actorCellId")]
        pub cell_id: i16,
        #[serde(alias = "championId")]
        pub champion_id: i16,
        pub completed: bool,
        #[serde(alias = "id")]
        pub action_id: i16,
        #[serde(alias = "isAllyAction")]
        pub is_ally: bool,
        #[serde(alias = "isInProgress")]
        pub in_progress: bool,
        #[serde(alias = "type")]
        pub action_type: CSActionType,
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    pub enum CSActionType {
        #[serde(alias = "pick")]
        #[default]
        Pick,
        #[serde(alias = "ban")]
        Ban,
        #[serde(alias = "ten_bans_reveal")]
        BanReveal,
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    pub struct CSTimer {
        #[serde(alias = "adjustedTimeLeftInPhase")]
        pub time_left_ms: i64,
        #[serde(alias = "totalTimeInPhase")]
        pub total_time_ms: i64,
        pub phase: CSPhase,
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    pub enum CSPhase {
        #[serde(alias = "BAN_PICK")]
        #[default]
        PickBan,
        #[serde(alias = "FINALIZATION")]
        Final,
        #[serde(alias = "PLANNING")]
        Planning,
        #[serde(alias = "GAME_STARTING")]
        GameStarting,
    }
}
