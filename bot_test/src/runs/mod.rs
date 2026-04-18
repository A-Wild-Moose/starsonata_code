use std::sync::{Arc, Mutex};
use indexmap::{IndexMap, IndexSet};

use poise::serenity_prelude as serenity;

mod time;

use crate::{Context};


trait JoinSetExt {
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

trait JoinMapExt {
    fn join(&self) -> String;
}

impl<T> JoinMapExt for IndexMap<serenity::User, T>
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


#[derive(Debug)]
enum OpenOrUser {
    Open(String),
    User(serenity::User)
}


impl std::fmt::Display for OpenOrUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenOrUser::Open(a) => write!(f, "{}", a),
            OpenOrUser::User(a) => write!(f, "{}", a)
        }
    }
}


#[derive(Debug)]
struct SpotData {
    user: OpenOrUser,
    emoji: serenity::ReactionType,
}


impl SpotData {
    fn default() -> Self {
        Self{
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


#[derive(Debug, poise::Modal)]
#[allow(dead_code)] // fields only used for Debug print
struct EditModal {
    #[name="Run name"]
    run_name: String,
    #[name="Squad size"]
    squad_size: String,
    #[name="Time"]
    time: String,
}


// Main struct for setting up the a run
#[derive(Debug)]
pub struct RunInfo {
    // metadata
    msg: Mutex<Option<serenity::Message>>,
    channel_id: serenity::ChannelId,
    author: serenity::User,
    // main data for the embed
    name: Mutex<String>,
    time: Mutex<String>,
    squad_size: Mutex<usize>,
    embed: Mutex<serenity::CreateEmbed>,
    line_up: Mutex<IndexMap<usize, SpotData>>,
    available: Mutex<IndexMap<serenity::User, IndexSet<EmojiData>>>
}


impl RunInfo {
    pub fn new(channel_id: serenity::ChannelId, author: serenity::User, name: String, squad_size: usize, time: String) -> Self {
        Self{
            msg: Mutex::new(None),
            channel_id: channel_id,
            author: author,
            name: Mutex::new(name),
            time: Mutex::new(time),
            squad_size: Mutex::new(squad_size),
            embed: Mutex::new(serenity::CreateEmbed::new()),
            line_up: Mutex::new(IndexMap::with_capacity(squad_size)),
            available: Mutex::new(IndexMap::with_capacity(squad_size)),
        }
    }

    pub fn make_title_organizer(&self) {
        let mut embed = self.embed.lock().unwrap();
        *embed = embed
            .clone()
            .title(format!("{}\n<t:{}:f>", self.name.lock().unwrap(), self.get_timestamp()))
            .field("", format!("Organizer: {}", self.author), false);
    }

    pub fn initialize(&self) {
        // set the title/organizer so we have a base embed
        self.make_title_organizer();

        // allocate the open spots
        let mut line_up = self.line_up.lock().unwrap();
        for i in 0..*self.squad_size.lock().unwrap() {
            line_up.insert(i, SpotData::default());
        }
    }

    pub fn make_edit_class_buttons(&self, classes: &IndexMap<String, EmojiData>) -> Vec<serenity::CreateActionRow> {
        let m: usize = (classes.len() + 1) / 5usize + 1;
        let mut button_vec: Vec<Vec<serenity::CreateButton>> = (0..m).map(|_| Vec::with_capacity(5)).collect();
        let mut i: usize = 1;  // already +1 due to edit button going first

        // edit button first
        button_vec[0].push(serenity::CreateButton::new("edit_modal").emoji('🔨'));

        for (class, emoji_data) in classes.iter() {
            button_vec[i / 5usize].push(serenity::CreateButton::new(class)
                .emoji(serenity::EmojiId::new(emoji_data.id))
                .style(serenity::ButtonStyle::Secondary)
            );
            i += 1;
        }
        button_vec
            .iter()
            .cloned()
            .map(|x| serenity::CreateActionRow::Buttons(x))
            .collect::<Vec<serenity::CreateActionRow>>()
    }

    pub fn make_embed(&self) -> serenity::CreateEmbed {
        let embed = self.embed.lock().unwrap().clone();

        embed
            .field("Selected line up:", self.line_up.lock().unwrap().join(), false)
            .field("The following players are available:", self.available.lock().unwrap().join(), false)
    }

    pub async fn make_run_msg(&self, ctx: Context<'_>, classes: &IndexMap<String, EmojiData>) {
        let embed = self.make_embed();
        let buttons = self.make_edit_class_buttons(classes);

        let msg = self.channel_id.send_message(
            ctx,
            serenity::CreateMessage::new()
                .embed(embed)
                .components(buttons)
        ).await.unwrap();

        *self.msg.lock().unwrap() = Some(msg);
    }

    pub fn edit_modal_update(&self, data: EditModal) {
        *self.name.lock().unwrap() = data.run_name;
        *self.time.lock().unwrap() = data.time;
        *self.squad_size.lock().unwrap() = data.squad_size.parse::<usize>().unwrap();
    }

    pub fn update_number_spots(&self) {
        let squad_size = self.squad_size.lock().unwrap();
        let mut line_up = self.line_up.lock().unwrap();

        let old_size = line_up.len();

        if *squad_size > old_size {
            for i in old_size..*squad_size {
                line_up.insert(i, SpotData::default());
            }
        } else if *squad_size < old_size {
            for i in *squad_size..old_size {
                // can swap remove here since the last index is getting removed anyways
                // so position doesnt matter for these elements
                line_up.swap_remove(&i);
            }
        }
    }

    pub async fn handle_edit_modal(&self, ctx: Context<'_>) {
        let mut msg = self.msg.lock().unwrap().clone().unwrap();
        let orig_name = self.name.lock().unwrap().clone();
        let orig_size = self.squad_size.lock().unwrap().clone();
        let orig_time = self.time.lock().unwrap().clone();
        
        while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
            .message_id(msg.id)
            .custom_ids(vec!["edit_modal".to_string()])
            .await
        {
            let data = poise::execute_modal_on_component_interaction::<EditModal>(
                ctx,
                mci.clone(),
                Some(EditModal{
                    run_name: orig_name.clone(),
                    squad_size: (orig_size).to_string(),
                    time: orig_time.clone(),
                }),
                None
            ).await.unwrap().unwrap();

            // update using the data from the modal
            self.edit_modal_update(data);
            // update the line-up
            self.update_number_spots();

            // get the embed
            let embed = self.make_embed();

            // update the original message
            msg.edit(
                ctx,
                serenity::EditMessage::new()
                    .embed(embed)
            ).await.unwrap();

        }
    }

    async fn update_available(&self, emoji_data: EmojiData, user: serenity::User) {
        let mut avail = self.available.lock().unwrap();

        match avail.get_mut(&user) {
            Some(classes) => {
                if classes.contains(&emoji_data) {
                    classes.shift_remove(&emoji_data);
                    if classes.len() == 0 {
                        avail.shift_remove(&user);
                    }
                } else {
                    classes.insert(emoji_data);
                }
            },
            None => {let _ = avail.insert(user, IndexSet::from([emoji_data]));}
        }
    }

    pub async fn handle_class_select(&self, ctx: Context<'_>, classes: &IndexMap<String, EmojiData>) {
        let mut msg = self.msg.lock().unwrap().clone().unwrap();
        // list of classes
        let class_vec = ctx.data().ss_classes.clone().into_keys().collect::<Vec<String>>();
        
        while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
            .message_id(msg.id)
            .custom_ids(class_vec.clone())
            .await
        {
            // get the emoji to add
            let emoji_data = classes.get(&mci.data.custom_id).expect("Button ID does not match available classes");
            // update the available classes
            self.update_available(emoji_data.clone(), mci.user.clone()).await;

            // make the updated embed
            let embed = self.make_embed();

            // update the original message
            msg.edit(
                ctx,
                serenity::EditMessage::new()
                    .embed(embed)
            ).await.unwrap();
            mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge).await.unwrap();
        }
    }

}






// fn update_title(mut embed: serenity::CreateEmbed, data: &EditModal, name: Arc<Mutex<String>>, time: Arc<Mutex<String>>) -> serenity::CreateEmbed {
//     if data.run_name != *name.lock().unwrap() || data.time != *time.lock().unwrap() {
//         *name = data.run_name;
//         *time = data.time;
//     }
//     updated_embed.title(
//         format!("{}\n<t:{}:f>", data.run_name, get_timestamp(&data.time));
//     )
// }


// Function to handle edits to the scheduled run
// async fn update_scheduled_run(
//     ctx: Context,
//     msg: serenity::Message,
//     mut embed: serenity::CreateEmbed,
//     name: Arc<Mutex<String>>,
//     squad_size: Arc<Mutex<usize>>,
//     time: Arc<Mutex<String>>
// ) {
//     while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
//         .message_id(msg.id)
//         .timeout(std::time::Duration::from_secs(10))
//         .custom_ids(vec!["edit_modal".to_string()])
//         .await
//     {
//         let data = poise::execute_modal_on_component_interaction::<EditModal>(
//             ctx,
//             mci.clone(),
//             Some(EditModal{
//                 run_name: *name.lock().unwrap(),
//                 squad_size: squad_size.lock().unwrap()to_string(),
//                 time: time.lock().unwrap()
//             }),
//             None
//         ).await.unwrap().unwrap();  // Returns Result<Option<>,_>

//         // update the embed title
//         let mut updated_embed = update_title(embed, &data, name.clone(), time.clone());
//         // get updated squad size
//         let sq_size = data.squad_size.parse::<usize>().unwrap();


//     }
// }