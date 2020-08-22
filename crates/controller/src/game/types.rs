use bs_diesel_utils::BSDieselEnum;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use s2_grpc_utils::{S2ProtoEnum, S2ProtoPack, S2ProtoUnpack};
use serde::{Deserialize, Serialize};

use flo_net::proto::flo_connect as packet;

use crate::map::Map;
use crate::node::{NodeRef, NodeRefColumns};
use crate::player::{PlayerRef, PlayerRefColumns};
use crate::schema::{game, game_used_slot};

#[derive(Debug, Serialize, Deserialize, S2ProtoPack, S2ProtoUnpack, Clone)]
#[s2_grpc(message_type(flo_grpc::game::Game))]
pub struct Game {
  pub id: i32,
  pub name: String,
  #[s2_grpc(proto_enum)]
  pub status: GameStatus,
  pub map: Map,
  pub slots: Vec<Slot>,
  pub node: Option<NodeRef>,
  pub is_private: bool,
  pub secret: Option<i32>,
  pub is_live: bool,
  pub num_players: i32,
  pub max_players: i32,
  pub created_by: Option<PlayerRef>,
  pub started_at: Option<DateTime<Utc>>,
  pub ended_at: Option<DateTime<Utc>>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl S2ProtoPack<flo_net::proto::flo_connect::GameInfo> for Game {
  fn pack(self) -> Result<flo_net::proto::flo_connect::GameInfo, s2_grpc_utils::result::Error> {
    use flo_net::proto::flo_connect::*;
    let status: flo_net::proto::flo_connect::GameStatus = self.status.into_proto_enum();
    Ok(GameInfo {
      id: self.id,
      name: self.name,
      status: status.into(),
      map: Some(flo_net::proto::flo_connect::Map {
        sha1: self.map.sha1.to_vec(),
        checksum: self.map.checksum,
        path: self.map.path,
      }),
      slots: self.slots.pack()?,
      node: self.node.pack()?,
      is_private: self.is_private,
      is_live: self.is_live,
      created_by: self.created_by.pack()?,
    })
  }
}

impl Game {
  pub fn get_player_ids(&self) -> Vec<i32> {
    self
      .slots
      .iter()
      .filter_map(|slot| slot.player.as_ref().map(|p| p.id))
      .collect()
  }

  pub fn find_player_slot_index(&self, player_id: i32) -> Option<usize> {
    self.slots.iter().position(|slot| {
      slot
        .player
        .as_ref()
        .map(|p| p.id == player_id)
        .unwrap_or_default()
    })
  }

  pub fn get_player_slot_info(&self, player_id: i32) -> Option<PlayerSlotInfo> {
    let slot_index = self.find_player_slot_index(player_id)?;

    let slot = &self.slots[slot_index];

    Some(PlayerSlotInfo {
      slot_index,
      slot,
      player: slot.player.as_ref().expect("player slot at index"),
    })
  }
}

#[derive(Debug)]
pub struct PlayerSlotInfo<'a> {
  pub slot_index: usize,
  pub slot: &'a Slot,
  pub player: &'a PlayerRef,
}

#[derive(Debug, Serialize, Deserialize, S2ProtoPack, S2ProtoUnpack, Queryable)]
#[s2_grpc(message_type(flo_grpc::game::GameEntry))]
pub struct GameEntry {
  pub id: i32,
  pub name: String,
  pub map_name: String,
  #[s2_grpc(proto_enum)]
  pub status: GameStatus,
  pub is_private: bool,
  pub is_live: bool,
  pub num_players: i32,
  pub max_players: i32,
  pub started_at: Option<DateTime<Utc>>,
  pub ended_at: Option<DateTime<Utc>>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub node: Option<NodeRef>,
  pub created_by: Option<PlayerRef>,
}

pub(crate) type GameEntryColumns = (
  game::dsl::id,
  game::dsl::name,
  game::dsl::map_name,
  game::dsl::status,
  game::dsl::is_private,
  game::dsl::is_live,
  diesel::expression::SqlLiteral<diesel::sql_types::Integer>,
  game::dsl::max_players,
  game::dsl::started_at,
  game::dsl::ended_at,
  game::dsl::created_at,
  game::dsl::updated_at,
  diesel::helper_types::Nullable<NodeRefColumns>,
  diesel::helper_types::Nullable<PlayerRefColumns>,
);

impl GameEntry {
  pub(crate) fn columns() -> GameEntryColumns {
    (
      game::dsl::id,
      game::dsl::name,
      game::dsl::map_name,
      game::dsl::status,
      game::dsl::is_private,
      game::dsl::is_live,
      diesel::dsl::sql("0"),
      game::dsl::max_players,
      game::dsl::started_at,
      game::dsl::ended_at,
      game::dsl::created_at,
      game::dsl::updated_at,
      NodeRef::COLUMNS.nullable(),
      PlayerRef::COLUMNS.nullable(),
    )
  }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, BSDieselEnum, S2ProtoEnum)]
#[repr(i32)]
#[s2_grpc(proto_enum_type(flo_grpc::game::GameStatus, flo_net::proto::flo_connect::GameStatus))]
pub enum GameStatus {
  Preparing = 0,
  Created = 1,
  Running = 2,
  Ended = 3,
  Paused = 4,
  Terminated = 5,
}

#[derive(Debug, Serialize, Deserialize, S2ProtoPack, S2ProtoUnpack, Clone)]
#[s2_grpc(message_type(flo_grpc::game::Slot, flo_net::proto::flo_connect::Slot))]
pub struct Slot {
  pub player: Option<PlayerRef>,
  pub settings: SlotSettings,
  pub client_status: SlotClientStatus,
}

impl Slot {
  pub fn is_used(&self) -> bool {
    self.settings.status != SlotStatus::Open
  }
}

impl Default for Slot {
  fn default() -> Self {
    Slot {
      player: None,
      settings: Default::default(),
      client_status: SlotClientStatus::Pending,
    }
  }
}

#[derive(Debug, Serialize, Deserialize, S2ProtoPack, S2ProtoUnpack, Clone, Queryable)]
#[s2_grpc(message_type(
  flo_grpc::game::SlotSettings,
  flo_net::proto::flo_connect::SlotSettings
))]
pub struct SlotSettings {
  pub team: i32,
  pub color: i32,
  #[s2_grpc(proto_enum)]
  pub computer: Computer,
  pub handicap: i32,
  #[s2_grpc(proto_enum)]
  pub status: SlotStatus,
  #[s2_grpc(proto_enum)]
  pub race: Race,
}

pub(crate) type SlotSettingsColumns = (
  game_used_slot::dsl::team,
  game_used_slot::dsl::color,
  game_used_slot::dsl::computer,
  game_used_slot::dsl::handicap,
  game_used_slot::dsl::status,
  game_used_slot::dsl::race,
);

impl SlotSettings {
  pub(crate) const COLUMNS: SlotSettingsColumns = (
    game_used_slot::dsl::team,
    game_used_slot::dsl::color,
    game_used_slot::dsl::computer,
    game_used_slot::dsl::handicap,
    game_used_slot::dsl::status,
    game_used_slot::dsl::race,
  );
}

impl Default for SlotSettings {
  fn default() -> Self {
    SlotSettings {
      team: 0,
      color: 0,
      computer: Computer::Easy,
      handicap: 100,
      status: SlotStatus::Open,
      race: Race::Human,
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, S2ProtoEnum, BSDieselEnum)]
#[repr(i32)]
#[s2_grpc(proto_enum_type(flo_grpc::game::SlotStatus, flo_net::proto::flo_connect::SlotStatus))]
pub enum SlotStatus {
  Open = 0,
  Closed = 1,
  Occupied = 2,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, S2ProtoEnum, BSDieselEnum)]
#[repr(i32)]
#[s2_grpc(proto_enum_type(flo_grpc::game::Race, flo_net::proto::flo_connect::Race))]
pub enum Race {
  Human = 0,
  Orc = 1,
  NightElf = 2,
  Undead = 3,
  Random = 4,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, S2ProtoEnum, BSDieselEnum)]
#[repr(i32)]
#[s2_grpc(proto_enum_type(flo_grpc::game::Computer, flo_net::proto::flo_connect::Computer))]
pub enum Computer {
  Easy = 0,
  Normal = 1,
  Insane = 2,
}

impl SlotSettings {
  pub fn into_packet(self) -> packet::SlotSettings {
    packet::SlotSettings {
      team: self.team,
      color: self.color,
      computer: match self.computer {
        Computer::Easy => packet::Computer::Easy,
        Computer::Normal => packet::Computer::Normal,
        Computer::Insane => packet::Computer::Insane,
      }
      .into(),
      handicap: self.handicap,
      status: match self.status {
        SlotStatus::Open => packet::SlotStatus::Open,
        SlotStatus::Closed => packet::SlotStatus::Closed,
        SlotStatus::Occupied => packet::SlotStatus::Occupied,
      }
      .into(),
      race: match self.race {
        Race::Human => packet::Race::Human,
        Race::Orc => packet::Race::Orc,
        Race::NightElf => packet::Race::NightElf,
        Race::Undead => packet::Race::Undead,
        Race::Random => packet::Race::Random,
      }
      .into(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, S2ProtoEnum, BSDieselEnum)]
#[repr(i32)]
#[s2_grpc(proto_enum_type(flo_net::proto::flo_node::SlotClientStatus))]
pub enum SlotClientStatus {
  Pending = 0,
  Connected = 1,
  Loading = 2,
  Loaded = 3,
  Left = 4,
  Disconnected = 5,
}
