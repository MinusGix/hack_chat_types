use std::{collections::HashMap, convert::TryFrom};

#[cfg(feature = "json_parsing")]
use crate::util::{as_array, as_object, FromJson, FromJsonError, IntoJson};
#[cfg(feature = "json_parsing")]
use json::JsonValue;

use crate::{
    id, util::Color, util::Command, util::MaybeExist, util::ServerCommand, Channel, Hash, Nickname,
    ServerApi, SessionId, Text, Timestamp, Trip, UserId, UserLevel,
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
    #[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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

        pub fn from_info(users: &Users, info: &super::Info) -> Result<Self, InviteConversionError> {
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
        /// Failed to find the user
        NoUserFound,
    }

    #[derive(Debug, Clone)]
    pub enum EmoteInfoConversionError {
        /// No user specified.
        NoUser,
        /// There was no @ prefix
        NoAt,
        /// Failed to find the user
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

        pub fn from_info(
            users: &Users,
            info: &super::Info,
        ) -> Result<Self, EmoteInfoConversionError> {
            let mut split = info.text.splitn(2, ' ');
            let from = split.next().ok_or(EmoteInfoConversionError::NoUser)?;
            let from = from
                .strip_prefix('@')
                .ok_or(EmoteInfoConversionError::NoAt)?;

            let user_id = users
                .find_online_nick(from)
                .map(|x| x.0)
                .ok_or(EmoteInfoConversionError::NoUserFound)?;

            let text = split
                .next()
                .map(|x| x.to_string())
                .unwrap_or_else(String::new);

            Ok(Self {
                text,
                user_id,
                time: info.time,
            })
        }
    }
}
