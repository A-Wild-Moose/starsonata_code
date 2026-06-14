use indexmap::{IndexMap, IndexSet};
use serenity::model::application::CommandInteraction;
use serenity::model::user::User;
use serenity::model::channel::Message;
use serenity::builder::CreateEmbed;
use chrono_tz::Tz;

use crate::runs::{RunInfo, SpotData, EmojiData};
use crate::runs::time::get_timestamp;


pub trait JoinSetExt {
    fn join(&self) -> String;
}

impl JoinSetExt for IndexSet<EmojiData> {
    fn join(&self) -> String {
        let mut a = String::from("");
        for v in self.iter() {
            a.push_str(format!("<:{}:{}>", v.name, v.id).as_str());
        }
        a
    }
}

pub trait JoinMapExt {
    fn join(&self) -> String;
}

impl<T> JoinMapExt for IndexMap<User, T>
where
    T: JoinSetExt
{
    fn join(&self) -> String {
        let mut a = String::from("");
        for (k, v) in self.iter() {
            a.push_str(format!("{}: {}\n", k, v.join()).as_str());
        }
        a
    }
}

impl JoinMapExt for IndexMap<usize, SpotData> {
    fn join(&self) -> String {
        let mut a = String::from("");
        for (_, v) in self.iter() {
            a.push_str(format!("{}: {}\n", v.emoji, v.user).as_str())
        }
        a
    }
}


impl RunInfo {
    pub fn new(interaction: &CommandInteraction, name: String, size: usize, time: String, timezone: Tz) -> Self {
        let mut line_up: IndexMap<usize, SpotData> = IndexMap::with_capacity(size);
        for i in 0..size {
            line_up.insert(i, SpotData::default());
        }
        
        Self {
            msg_id: None,
            channel_id: interaction.channel_id.clone(),
            guild_id: interaction.guild_id.clone(),
            organizer: interaction.user.clone(),
            name: name,
            time: get_timestamp(time, timezone).expect("Unable to parse date in either YYYY-MM-DD HH:MM or HH:MM format"),
            size: size,
            line_up: line_up,
            available: IndexMap::with_capacity(size),
        }
    }

    pub fn update_size(&mut self, new_size: usize) {
        // handle adding or removing line-up spots
        if new_size > self.size {
            for i in self.size..new_size {
                self.line_up.insert(i, SpotData::default());
            }
        } else if new_size < self.size {
            for _ in new_size..self.size {
                let _ = self.line_up.pop();
            }
        }

        // set size after compared
        self.size = new_size;
    }

    pub fn make_embed(&self) -> CreateEmbed {
        CreateEmbed::new()
            .title(format!("{}\n<t:{}:f>", self.name, self.time))
            .field("", format!("Organizer: {}", self.organizer), false)
            .field("Selected line up:", self.line_up.join(), false)
            .field("The following players are available:", self.available.join(), false)
    }

    pub fn set_message_id(&mut self, msg: Message) {
        self.msg_id = Some(msg.id);
    }
}