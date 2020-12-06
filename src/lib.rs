use std::{collections::HashMap, fmt::Display, num::ParseIntError};

use json::JsonValue;
use util::{FromJsonError, MaybeExist};

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
    pub fn from_json(json: &mut JsonValue) -> MaybeExist<Trip> {
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

    pub fn from_json(value: &JsonValue) -> Result<Timestamp, FromJsonError> {
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

/// Common identifiers
pub mod id {
    pub const CMD: &str = "cmd";
    pub const CHANNEL: &str = "channel";
    pub const TEXT: &str = "text";
    pub const NICK: &str = "nick";
    pub const TIME: &str = "time";
    pub const TRIP: &str = "trip";
    pub const USER_TYPE: &str = "uType";
    pub const HASH: &str = "hash";
    pub const LEVEL: &str = "level";
    pub const USER_ID: &str = "userid";
    pub const COLOR: &str = "color";
    pub const IS_BOT: &str = "isBot";
}

/// Messages sent from the client (us)
pub mod client {
    use json::{object, JsonValue};

    use crate::{
        id,
        util::{ClientCommand, Command, IntoJson},
    };

    use super::{Channel, Nickname, Password, ServerApi, SessionId, Text};

    /// V2 Specific
    /// Sent to the server before even joining the channel.
    /// Server replies back with `server::Session`.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Session {
        /// Whether or not this client is a bot.
        pub is_bot: bool,
        /// Optional session id to resume as.
        /// Currently unsupported on the server, but it exists.
        pub id: Option<SessionId>,
    }
    impl Command for Session {
        const CMD: &'static str = "session";
    }
    impl ClientCommand for Session {}
    impl IntoJson for Session {
        fn into_json(self, _server_api: ServerApi) -> JsonValue {
            let mut value = object! {};
            value[id::CMD] = Self::CMD.into();
            value["isBot"] = self.is_bot.into();
            if let Some(id) = self.id {
                value["id"] = id.into();
            }
            value
        }
    }

    /// Command for joining a channel.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Join {
        /// The nickname that you wish to join as. Usually it is what you get.
        pub nick: Nickname,
        /// The channel you wish to join. Often-times is what you join.
        pub channel: Channel,
        /// The password, which is optional, and generates your trip.
        /// On some server-apis this is appended to the nick, on others it is in a separate field.
        /// That is handled by the `IntoJson` method.
        pub password: Option<Password>,
    }
    impl Command for Join {
        const CMD: &'static str = "join";
    }
    impl ClientCommand for Join {}
    impl IntoJson for Join {
        fn into_json(mut self, server_api: ServerApi) -> JsonValue {
            const PASS: &str = "pass";

            let mut value = object! {};
            value[id::CMD] = Self::CMD.into();
            // We don't set nick early on as password can modify it
            value[id::CHANNEL] = self.channel.into();
            if let Some(password) = self.password {
                match server_api {
                    // TODO: should this be hackchatprev2? its relatively recent...
                    ServerApi::HackChatV2 | ServerApi::HackChatPreV2 => {
                        value[PASS] = password.into()
                    }
                    ServerApi::HackChatLegacy => {
                        // Format is 'nick#password' for legacy servers
                        self.nick.push('#');
                        self.nick.push_str(&password);
                    }
                }
            }
            value[id::NICK] = self.nick.into();
            value
        }
    }

    /// Tells the server that you wish to send a message.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Chat {
        /// Only needed on V2 as it desires to have multi-channel support.
        pub channel: Option<Channel>,
        /// The text that is sent.
        pub text: Text,
    }
    impl Command for Chat {
        const CMD: &'static str = "chat";
    }
    impl ClientCommand for Chat {}
    impl IntoJson for Chat {
        fn into_json(self, server_api: ServerApi) -> JsonValue {
            let mut value = object! {};
            value[id::CMD] = Self::CMD.into();
            value[id::TEXT] = self.text.into();
            if let ServerApi::HackChatV2 = server_api {
                value[id::CHANNEL] = self.channel.into();
            }

            value
        }
    }
}

pub mod server {
    use std::{collections::HashMap, convert::TryFrom};

    use json::JsonValue;

    use crate::{
        id,
        util::Color,
        util::Command,
        util::FromJson,
        util::FromJsonError,
        util::MaybeExist,
        util::ServerCommand,
        util::{as_array, as_object},
        Channel, Hash, Nickname, ServerApi, SessionId, Text, Timestamp, Trip, UserId, UserLevel,
    };

    /// The type of the user. Deprecated in v2 and replaced with levels.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum UserType {
        // "userType": "user"
        User,
        // "userType": "mod",
        Mod,
        // "userType": "admin", (I think)
        Admin,
    }
    impl UserType {
        // TODO: should this use a MaybeExist?
        pub fn from_json(value: &JsonValue) -> Option<UserType> {
            value
                .as_str()
                .map(UserType::try_from)
                .and_then(|x| x.map(Some).unwrap_or(None))
        }
    }
    impl TryFrom<&str> for UserType {
        type Error = ();
        fn try_from(user_type: &str) -> Result<UserType, ()> {
            Ok(match user_type {
                "user" => UserType::User,
                "mod" => UserType::Mod,
                "admin" => UserType::Admin,
                _ => return Err(()),
            })
        }
    }

    /// Informs client about the users within a channel.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct OnlineSet {
        /// The nicknames of all the users in the channel. Legacy/PreV2/V2
        pub nicks: Option<Vec<Nickname>>,
        /// Provides more information about the users. Prev2/V2
        pub users: Option<Vec<OnlineSetUser>>,
        /// The channel that we've joined. V2/(PreV2?)
        pub channel: Option<Channel>,
        /// The time that we joined.
        pub time: Timestamp,
    }
    impl Command for OnlineSet {
        const CMD: &'static str = "onlineSet";
    }
    impl ServerCommand for OnlineSet {}
    impl FromJson for OnlineSet {
        fn from_json(mut json: JsonValue, server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            const NICKS: &str = "nicks";
            const USERS: &str = "users";

            let nicks = as_array(json[NICKS].take())
                .map(|x| {
                    x.into_iter()
                        .map(|mut x| x.take_string().map(Nickname::from))
                        .collect::<Option<Vec<Nickname>>>()
                })
                .flatten();
            let users = as_array(json[USERS].take())
                .map(|users| {
                    users
                        .into_iter()
                        .map(|x| OnlineSetUser::from_json(x, server_api))
                        .collect::<Result<Vec<OnlineSetUser>, FromJsonError>>()
                })
                .transpose()?;
            let channel = json[id::TEXT].take_string();
            let time = Timestamp::from_json(&json[id::TIME])?;
            Ok(Self {
                nicks,
                users,
                channel: channel.map(Channel::from),
                time,
            })
        }
    }
    /// Detailed information about a specific user from OnlineSet
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct OnlineSetUser {
        /// The channel they are in. Unsure as to why this is bothered to be included.
        pub channel: Channel,
        /// If it is the user who joined the channel and is getting sent this message.
        pub is_me: Option<bool>,
        /// If the user i sa bot.
        pub is_bot: Option<bool>,
        /// The user's name.
        pub nick: Nickname,
        /// The user's trip.
        pub trip: MaybeExist<Trip>,
        /// The usertype which specifies their permissions.
        pub user_type: Option<UserType>,
        /// An id that identifies them
        pub user_id: Option<UserId>,
        /// Their ip hash.
        pub hash: Hash,
        /// The color that they have selected within the chat.
        pub color: Option<Color>,
        /// The user's permission level.
        pub level: Option<UserLevel>,
    }
    impl FromJson for OnlineSetUser {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            const IS_ME: &str = "isme";

            let channel = json[id::CHANNEL]
                .take_string()
                .ok_or(FromJsonError::InvalidCommandField(id::CHANNEL))?;
            let is_me = json[IS_ME].as_bool();
            let is_bot = json[id::IS_BOT].as_bool();
            let nick = json[id::NICK]
                .take_string()
                .ok_or(FromJsonError::InvalidCommandField(id::NICK))?;
            let trip = Trip::from_json(&mut json[id::TRIP]);
            let user_type = UserType::from_json(&json[id::USER_TYPE]);
            let user_id = json[id::USER_ID].as_u64();
            let hash = json[id::HASH]
                .take_string()
                .ok_or(FromJsonError::InvalidCommandField(id::CHANNEL))?;
            // We ignore color if it is malformed.
            // TODO: log that it was malformed
            let color = json[id::COLOR]
                .as_str()
                .and_then(|x| Color::try_from(x).ok());
            let level = json[id::LEVEL].as_u64();
            Ok(Self {
                channel,
                is_me,
                is_bot,
                nick,
                trip,
                user_type,
                user_id,
                hash,
                color,
                level,
            })
        }
    }

    /// Information about the user's session and the server.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Session {
        /// Number of users server wide
        pub users: u32,
        /// Number of channels with at least a single user server-wide.
        pub channels: u32,
        /// A list of certain 'public' (frontpaged) channels with user count.
        pub public: HashMap<Channel, u32>,
        /// The user's session id.
        pub session_id: SessionId,
        /// Whether or not their session was restored.
        pub restored: Option<bool>,
        /// The time that this was sent at.
        pub time: Timestamp,
    }
    impl Command for Session {
        const CMD: &'static str = "session";
    }
    impl ServerCommand for Session {}
    impl FromJson for Session {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            const USERS: &str = "users";
            const CHANNELS: &str = "chans";
            const PUBLIC: &str = "public";
            const SESSION_ID: &str = "sessionID";
            const RESTORED: &str = "restored";

            let users = json[USERS]
                .as_u32()
                .ok_or(FromJsonError::InvalidField(USERS))?;
            let channels = json[CHANNELS]
                .as_u32()
                .ok_or(FromJsonError::InvalidField(CHANNELS))?;
            let public = as_object(json[PUBLIC].take())
                .map(|mut object| {
                    // TODO: it would be nice to take ownership of key if possible.
                    let mut public = HashMap::with_capacity(object.len());
                    for (channel, user_count) in object.iter_mut() {
                        let channel = channel.to_owned();
                        let user_count = user_count
                            .as_u32()
                            .ok_or(FromJsonError::InvalidField(PUBLIC))?;
                        public.insert(channel, user_count);
                    }
                    Ok(public)
                })
                .transpose()?
                .unwrap_or_else(HashMap::new);
            let session_id = json[SESSION_ID]
                .take_string()
                .ok_or(FromJsonError::InvalidField(SESSION_ID))?;
            let restored = json[RESTORED].as_bool();
            let time = Timestamp::from_json(&json[id::TIME])?;
            Ok(Self {
                users,
                channels,
                public,
                session_id,
                restored,
                time,
            })
        }
    }

    /// General info text.
    /// In the legacy server this often has to be synthesized (see the synthetic module)
    /// into types which let you deal with them.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Info {
        pub text: Text,
        pub channel: Option<Channel>,
        pub time: Timestamp,
    }
    impl Command for Info {
        const CMD: &'static str = "info";
    }
    impl ServerCommand for Info {}
    impl FromJson for Info {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            let text = json[id::TEXT]
                .take_string()
                .ok_or(FromJsonError::InvalidField(id::TEXT))?;
            let channel = json[id::CHANNEL].take_string();
            let time = Timestamp::from_json(&json[id::TIME])?;
            Ok(Info {
                text,
                channel,
                time,
            })
        }
    }

    // TODO: provide a more limited synthetic version that lets you just get the user id and access
    // latest permissions state?
    /// A chat message from a user on the server
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Chat {
        /// Nickname of the user
        pub nick: Nickname,
        /// That user's user type.
        pub user_type: Option<UserType>,
        /// That user's identifier.
        pub user_id: Option<UserId>,
        /// The channel it was sent in. PreV2(?)/V2
        pub channel: Option<Channel>,
        /// The content of the message
        pub text: Text,
        /// The user's permission level.
        pub level: Option<UserLevel>,
        /// Default: false
        /// Whether they are a mod. Superseded by usertpye and then user levels.
        pub is_mod: bool,
        /// Default: false
        /// Whether they are an admin. Same status as is_mod.
        pub is_admin: bool,
        // TODO: can we consider not having a trip field to mean that user does not have a trip?
        // The identifying trip code of the user.
        pub trip: MaybeExist<Trip>,
        /// The time the message was sent.
        pub time: Timestamp,
    }
    impl Command for Chat {
        const CMD: &'static str = "chat";
    }
    impl ServerCommand for Chat {}
    impl FromJson for Chat {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            const MOD: &str = "mod";
            const ADMIN: &str = "admin";

            let nick = json[id::NICK]
                .take_string()
                .ok_or(FromJsonError::InvalidField(id::NICK))?;
            // This defaults to None if it was not parsed correctly.
            // TODO: log somehow that we failed to parse it?
            let user_type = json[id::USER_TYPE]
                .as_str()
                .map(UserType::try_from)
                .and_then(|x| x.map(Some).unwrap_or(None));
            let user_id = json[id::USER_ID].as_u64();
            let channel = json[id::CHANNEL].take_string();
            let text = json[id::TEXT]
                .take_string()
                .ok_or(FromJsonError::InvalidField(id::TEXT))?;
            let level = json[id::LEVEL].as_u64();
            let is_mod = json[MOD].as_bool().unwrap_or(false);
            let is_admin = json[ADMIN].as_bool().unwrap_or(false);
            let trip = Trip::from_json(&mut json[id::TRIP]);
            let time = Timestamp::from_json(&json[id::TIME])?;

            Ok(Self {
                nick,
                user_type,
                user_id,
                channel: channel.map(Channel::from),
                text,
                level,
                is_mod,
                is_admin,
                trip: trip.map(Trip::from),
                time,
            })
        }
    }

    /// Captcha message to stop spamming bots from joining.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Captcha {
        pub text: Text,
        pub channel: Option<Channel>,
    }
    impl Command for Captcha {
        const CMD: &'static str = "captcha";
    }
    impl ServerCommand for Captcha {}
    impl FromJson for Captcha {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            let text = json[id::TEXT]
                .take_string()
                .ok_or(FromJsonError::InvalidField(id::TEXT))?;
            let channel = json[id::CHANNEL].take_string();

            Ok(Self {
                text,
                channel: channel.map(Channel::from),
            })
        }
    }

    /// A /me message
    /// Ex: '@User does jumping jacks'.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Emote {
        pub text: Text,
        pub nick: Option<Nickname>,
        pub time: Timestamp,
        /// We are uncertain that this always has had a trip
        pub trip: MaybeExist<Trip>,
        /// From server
        pub user_id: Option<UserId>,
    }
    impl Command for Emote {
        const CMD: &'static str = "emote";
    }
    impl ServerCommand for Emote {}
    impl FromJson for Emote {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            // TODO: should i strip name prefix?
            let text = json[id::TEXT]
                .take_string()
                .map(Text::from)
                .ok_or(FromJsonError::InvalidField(id::TEXT))?;
            let nick = json[id::NICK].take_string().map(Nickname::from);
            let time = Timestamp::from_json(&json[id::TIME])?;
            let trip = Trip::from_json(&mut json[id::TRIP]);
            let user_id = json[id::USER_ID].as_u64();
            Ok(Self {
                text,
                nick,
                time,
                trip,
                user_id,
            })
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Invite {
        /// NOTE: This is the channel it was sent in, not the one that is being invited to!
        pub channel: Option<Channel>,
        /// The id of the user inviting, may be self
        /// From Server
        pub from: UserId,
        /// The id of the user being invited, may be self
        /// From Server
        pub to: UserId,
        /// The channel that is being invited to.
        pub invite_channel: Channel,
        /// Time of the message being sent
        pub time: Timestamp,
    }
    impl Command for Invite {
        const CMD: &'static str = "invite";
    }
    impl ServerCommand for Invite {}
    impl FromJson for Invite {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            const INVITE_CHANNEL: &str = "inviteChannel";
            const FROM: &str = "from";
            const TO: &str = "to";

            let channel = json[id::CHANNEL].take_string().map(Channel::from);
            let from = json[FROM]
                .as_u64()
                .ok_or(FromJsonError::InvalidField(FROM))?;
            let to = json[TO].as_u64().ok_or(FromJsonError::InvalidField(TO))?;
            let invite_channel = json[INVITE_CHANNEL]
                .take_string()
                .map(Channel::from)
                .ok_or(FromJsonError::InvalidField(INVITE_CHANNEL))?;
            let time = Timestamp::from_json(&json[id::TIME])?;
            Ok(Self {
                channel,
                from,
                to,
                invite_channel,
                time,
            })
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct OnlineAdd {
        pub channel: Option<Channel>,
        pub color: Option<Color>,
        pub hash: Option<Hash>,
        pub is_bot: Option<bool>,
        pub level: Option<UserLevel>,
        pub nick: Nickname,
        pub time: Timestamp,
        pub trip: MaybeExist<Trip>,
        pub user_type: Option<UserType>,
        pub user_id: Option<UserId>,
    }
    impl Command for OnlineAdd {
        const CMD: &'static str = "onlineAdd";
    }
    impl ServerCommand for OnlineAdd {}
    impl FromJson for OnlineAdd {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            let channel = json[id::CHANNEL].take_string().map(Channel::from);
            let color = json[id::COLOR]
                .as_str()
                .and_then(|x| Color::try_from(x).ok());
            let hash = json[id::HASH].take_string().map(Hash::from);
            let is_bot = json[id::IS_BOT].as_bool();
            let level = json[id::LEVEL].as_u64();
            let nick = json[id::NICK]
                .take_string()
                .map(Nickname::from)
                .ok_or(FromJsonError::InvalidField(id::NICK))?;
            let time = Timestamp::from_json(&json[id::TIME])?;
            let trip = Trip::from_json(&mut json[id::TRIP]);
            let user_type = UserType::from_json(&json[id::USER_TYPE]);
            let user_id = json[id::USER_ID].as_u64();
            Ok(Self {
                channel,
                color,
                hash,
                is_bot,
                level,
                nick,
                time,
                trip,
                user_type,
                user_id,
            })
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct OnlineRemove {
        pub channel: Option<Channel>,
        pub nick: Nickname,
        pub time: Timestamp,
        pub user_id: Option<UserId>,
    }
    impl Command for OnlineRemove {
        const CMD: &'static str = "onlineRemove";
    }
    impl ServerCommand for OnlineRemove {}
    impl FromJson for OnlineRemove {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            if json[id::CMD].as_str() != Some(Self::CMD) {
                return Err(FromJsonError::InvalidCommandField(Self::CMD));
            }

            let channel = json[id::CHANNEL].take_string().map(Channel::from);
            let nick = json[id::NICK]
                .take_string()
                .map(Nickname::from)
                .ok_or(FromJsonError::InvalidField(id::NICK))?;
            let time = Timestamp::from_json(&json[id::TIME])?;
            let user_id = json[id::USER_ID].as_u64();
            Ok(Self {
                channel,
                nick,
                time,
                user_id,
            })
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Warn {
        pub channel: Option<Channel>,
        pub text: Text,
        pub time: Timestamp,
    }
    impl Command for Warn {
        const CMD: &'static str = "warn";
    }
    impl ServerCommand for Warn {}
    impl FromJson for Warn {
        fn from_json(mut json: JsonValue, _server_api: ServerApi) -> Result<Self, FromJsonError> {
            let channel = json[id::CHANNEL].take_string().map(Channel::from);
            let text = json[id::TEXT]
                .take_string()
                .map(Text::from)
                .ok_or(FromJsonError::InvalidField(id::TEXT))?;
            let time = Timestamp::from_json(&json[id::TIME])?;
            Ok(Self {
                channel,
                text,
                time,
            })
        }
    }

    /// Structures of commands that are joined together
    pub mod synthetic {
        use crate::{AccessUserId, Channel, Text, Timestamp, Users};

        #[derive(Debug, Clone)]
        pub enum InviteConversionError {
            /// There was not even a beginning user that it was from
            NoFrom,
            /// There was no 'invited' text.
            NoInvited,
            /// There was no user to invite
            NoTo,
            /// There was no 'to' text
            NoToJoiner,
            /// There was no channel
            NoChannel,
            /// Invalid channel somehow
            InvalidChannel,
            /// We did not know a user.
            UnknownNick,
            /// We don't know the id of self.
            UnknownSelf,
        }

        #[derive(Debug, Clone, Eq, PartialEq)]
        pub struct Invite {
            /// The channel that you are invited to.
            pub invite_channel: Channel,
            /// User id of inviter, may be self.
            pub from: AccessUserId,
            /// User id of invited, may be self.
            pub to: AccessUserId,
            pub time: Timestamp,
        }
        impl Invite {
            pub fn from_invite(_users: &Users, invite: super::Invite) -> Self {
                let from = invite.from;
                let to = invite.to;
                let invite_channel = invite.invite_channel;
                let time = invite.time;
                Self {
                    from: AccessUserId::Server(from),
                    invite_channel,
                    to: AccessUserId::Server(to),
                    time,
                }
            }

            pub fn from_info(
                users: &Users,
                info: &super::Info,
            ) -> Result<Self, InviteConversionError> {
                // TODO: handle empty parts of the text
                let mut split = info.text.splitn(4, ' ');
                let from = split.next().ok_or(InviteConversionError::NoFrom)?;

                if split.next() != Some("invited") {
                    return Err(InviteConversionError::NoInvited);
                }

                let to = split.next().ok_or(InviteConversionError::NoTo)?;

                if split.next() != Some("to") {
                    return Err(InviteConversionError::NoToJoiner);
                }

                // The channel in text includes a question mark at the start, since the site uses
                // that for identifying.
                let channel = split
                    .next()
                    .ok_or(InviteConversionError::NoChannel)?
                    .strip_prefix('?')
                    .ok_or(InviteConversionError::InvalidChannel)?;
                let (to, from) = if to == "you" {
                    // We are being invited.
                    let to = users.ourself().ok_or(InviteConversionError::UnknownSelf)?;
                    let from = users
                        .find_online_nick(from)
                        .map(|x| x.0)
                        .ok_or(InviteConversionError::UnknownNick)?;
                    (to, from)
                } else {
                    // We are inviting.
                    let to = users
                        .find_online_nick(to)
                        .map(|x| x.0)
                        .ok_or(InviteConversionError::UnknownNick)?;
                    let from = users.ourself().ok_or(InviteConversionError::UnknownSelf)?;
                    (to, from)
                };
                Ok(Self {
                    from,
                    to,
                    invite_channel: channel.to_owned(),
                    time: info.time,
                })
            }
        }

        #[derive(Debug, Clone)]
        pub enum EmoteConversionError {
            NoUserFound,
        }

        #[derive(Debug, Clone, Eq, PartialEq)]
        pub struct Emote {
            pub text: Text,
            pub user_id: AccessUserId,
            pub time: Timestamp,
        }
        impl Emote {
            // TODO: it would be nice to consume Emote.
            pub fn from_emote(
                users: &Users,
                emote: &super::Emote,
            ) -> Result<Self, EmoteConversionError> {
                // TODO: should i strip name prefix
                let user_id = emote
                    .user_id
                    .map(AccessUserId::Server)
                    .or_else(|| {
                        emote
                            .nick
                            .as_ref()
                            .map(|nick| users.find_online_nick(&nick).map(|x| x.0))
                            .flatten()
                    })
                    .ok_or(EmoteConversionError::NoUserFound)?;
                let time = emote.time;
                Ok(Self {
                    text: emote.text.clone(),
                    user_id,
                    time,
                })
            }

            // TODO: convert from info structure
        }
    }
}

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
