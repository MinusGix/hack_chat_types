#[cfg(feature = "json_parsing")]
use crate::util::{FromJson, FromJsonError, IntoJson};
#[cfg(feature = "json_parsing")]
use json::{object, JsonValue};

use crate::util::{ClientCommand, Command};

use super::{id, Channel, Nickname, Password, ServerApi, SessionId, Text};

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
#[cfg(feature = "json_parsing")]
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
#[cfg(feature = "json_parsing")]
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
                ServerApi::HackChatV2 | ServerApi::HackChatPreV2 => value[PASS] = password.into(),
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
#[cfg(feature = "json_parsing")]
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
