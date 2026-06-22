use serenity::model::user::User;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::model::channel::ReactionType;
use indexmap::{IndexMap, IndexSet};

pub mod run_info;
pub mod time;


#[derive(Debug, Clone)]
pub enum OpenOrUser {
    Open(String),
    User(User),
}

impl std::fmt::Display for OpenOrUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenOrUser::Open(a) => write!(f, "{}", a),
            OpenOrUser::User(a) => write!(f, "{}", a)
        }
    }
}


#[derive(Debug, Clone)]
pub struct SpotData {
    pub user: OpenOrUser,
    pub emoji: ReactionType,
}

impl SpotData {
    pub fn default() -> Self {
        Self {
            user: OpenOrUser::Open("<open>".to_string()),
            emoji: '👤'.into(),
        }
    }
}


#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct EmojiData {
    pub name: String,
    pub id: u64
}


#[derive(Debug, Clone)]
pub struct RunInfo {
    pub msg_id: Option<MessageId>,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub organizer: User,
    pub name: String,
    pub time: i64,
    pub size: usize,
    pub line_up: IndexMap<usize, SpotData>,
    pub available: IndexMap<User, IndexMap<String, EmojiData>>
}