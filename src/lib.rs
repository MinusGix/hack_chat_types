use std::{collections::HashMap, fmt::Display, num::ParseIntError};

#[cfg(feature = "json_parsing")]
use crate::util::FromJsonError;

use util::MaybeExist;

pub mod client;
pub mod id;
pub mod server;
pub mod util;

// TODO: provide a feature that disables the json

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ServerApi {
    /// Only partially supported as of this moment as the server is only partially implemented!
    HackChatV2,
    /// Latest 'legacy'
    HackChatPreV2,
    /// Legacy hc. More variable in what it is missing and supports.
    HackChatLegacy,
}

/// PreV2/V2 hash of ip address
pub type Hash = String;
// TODO: make this zeroable?
pub type Password = String;
/// Note: this is not assured to be <= 24 characters.
pub type Nickname = String;
/// This channel should not have any question mark prefix from the way the website is accessed.
pub type Channel = String;
/// Note: This is not assured to be exactly 6 characters, because exotic hc instances may exist.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Trip(pub String);
impl Display for Trip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl Trip {
    #[cfg(feature = "json_parsing")]
    pub fn from_json(json: &mut json::JsonValue) -> MaybeExist<Trip> {
        MaybeExist::from_option_unknown(json.take_string()).and_then(|x| {
            if x.is_empty() {
                MaybeExist::Not
            } else {
                MaybeExist::Has(Trip(x))
            }
        })
    }
}
pub type Text = String;
/// Unix timestamp.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Timestamp(pub u64);
impl Timestamp {
    pub fn parse(text: &str) -> Result<Timestamp, ParseIntError> {
        text.parse().map(Timestamp)
    }

    #[cfg(feature = "json_parsing")]
    pub fn from_json(value: &json::JsonValue) -> Result<Timestamp, FromJsonError> {
        value
            .as_u64()
            .map(Timestamp)
            .ok_or(FromJsonError::InvalidField(id::TIME))
    }
}
/// An identifier sent by the server that identifies the user.
pub type UserId = u64;
/// The level of the user, decides certain permissions.
/// This is technically an f64 since the server is written in javascript, but none of
/// the current values are actually floats.
pub type UserLevel = u64;
/// Session id in v2 version, which is received from the session cmd
pub type SessionId = String;

#[derive(Debug, Clone)]
pub enum ServerIdentifierTag {
    UserId,
    Nickname,
    Trip,
}
#[derive(Debug, Clone)]
pub enum ServerIdentifier<'a> {
    UserId(UserId),
    Nickname(&'a str),
    Trip(&'a str),
}

pub struct Users {
    /// An id that is used to generat new AccessUserId::Generated instances.
    id: UserId,
    /// The id of our own connection.
    pub ourself: Option<AccessUserId>,
    /// Mapping of ids (from server or generated) to info about the user.
    pub users: HashMap<AccessUserId, UserInfo>,
}
impl Users {
    pub fn generate_id(&mut self) -> AccessUserId {
        let id = self.id;
        self.id += 1;
        AccessUserId::Generated(id)
    }

    // TODO: this could be moved out of this and into the Connection
    /// Acquire the id of our own connection.
    pub fn ourself(&self) -> Option<AccessUserId> {
        self.ourself
    }

    /// Clear the list of users
    pub fn clear(&mut self) {
        self.users.clear();
    }

    /// Acquite a reference to some UserInfo
    pub fn get(&self, id: AccessUserId) -> Option<&UserInfo> {
        self.users.get(&id)
    }

    /// Acquire a mutable reference to some UserInfo
    pub fn get_mut(&mut self, id: AccessUserId) -> Option<&mut UserInfo> {
        self.users.get_mut(&id)
    }

    /// Insert a new user with id and user info
    pub fn insert(&mut self, id: AccessUserId, user_info: UserInfo) {
        self.users.insert(id, user_info);
    }

    /// Check if the list of users contains the given id.
    pub fn contains_key(&self, id: AccessUserId) -> bool {
        self.users.contains_key(&id)
    }

    /// Find the given nickname within, returning only the first found instance that is online.
    pub fn find_online_nick(&self, nick: &str) -> Option<(AccessUserId, &UserInfo)> {
        self.users
            .iter()
            .find(|(_, info)| info.online && info.nick == nick)
            .map(|(id, info)| (*id, info))
    }

    pub fn acquire_server_identifier(
        &self,
        id: AccessUserId,
        tag: ServerIdentifierTag,
    ) -> Option<ServerIdentifier<'_>> {
        match tag {
            ServerIdentifierTag::UserId => Some(ServerIdentifier::UserId(id.into_server_id()?)),
            ServerIdentifierTag::Nickname => {
                let info = self.get(id)?;
                Some(ServerIdentifier::Nickname(info.nick.as_ref()))
            }
            ServerIdentifierTag::Trip => {
                let info = self.get(id)?;
                info.trip
                    .as_ref()
                    .map(|x| x.0.as_ref())
                    .map(ServerIdentifier::Trip)
                    .into()
            }
        }
    }
}
impl Default for Users {
    fn default() -> Self {
        Self {
            id: 0,
            ourself: None,
            users: HashMap::with_capacity(64),
        }
    }
}
/// This exists because not everything might have an id, or we might be connecting to a legacy
/// HC instance that does not have user ids.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AccessUserId {
    /// Sent from the server
    Server(UserId),
    /// Generated by this client.
    Generated(UserId),
}
impl AccessUserId {
    pub fn into_server_id(self) -> Option<UserId> {
        match self {
            AccessUserId::Server(id) => Some(id),
            AccessUserId::Generated(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserInfo {
    pub nick: Nickname,
    pub trip: MaybeExist<Trip>,
    pub online: bool,
}
