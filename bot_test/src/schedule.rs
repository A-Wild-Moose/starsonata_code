use std::sync::Arc;
// use poise::serenity_prelude as serenity;

use crate::runs::RunInfo;
use crate::{Error, Context};

#[poise::command(slash_command)]
pub async fn schedule(
    ctx: Context<'_>,
    #[description = "Run name"] name: String,
    #[description = "Squad size limit"]
    #[min = 1]
    #[max = 10]
    squad_size: usize,
    #[description = "Run time"] time: String,
) -> Result<(), Error> {
    // get various information we need for the main structure for interaction
    let author = ctx.author();

    // create the run info structure
    let run = Arc::new(RunInfo::new(
        ctx.channel_id(),
        author.clone(),
        name,
        squad_size,
        time,
    ));

    run.initialize();

    // send a reply indicating run post is being created
    ctx.send(poise::CreateReply::default()
        .content("Creating run...")
        .ephemeral(true)
    ).await.unwrap();

    // send the message with the sign-up
    run.make_run_msg(ctx, &ctx.data().ss_classes).await;

    let ctx1 = ctx.clone();
    let ctx2 = ctx.clone();

    // handle edit modal
    tokio::spawn(async move {
        run.handle_edit_modal(ctx1).await
    });
    tokio::spawn(async move {
        run.handle_class_select(ctx2, &ctx1.data().ss_classes).await
    });

    Ok(())
}















//
// trait JoinSet {
//     fn join(&self) -> String;
// }

// impl JoinSet for IndexSet<EmojiData> {
//     fn join(&self) -> String {
//         let mut a = String::from("");
//         for v in self.iter() {
//             a.push_str(format!("<:{}:{}>", v.name, v.id).as_str());
//         }
//         a
//     }
// }

// trait JoinMap {
//     fn join(&self) -> String;
// }

// impl<T> JoinMap for IndexMap<serenity::User, T>
// where
//     T: JoinSet
// {
//     fn join(&self) -> String {
//         let mut a = String::from("");
//         for (k, v) in self.iter() {
//             a.push_str(format!("{}: {}\n", k, v.join()).as_str());
//         }
//         a
//     }
// }

// impl JoinMap for IndexMap<usize, SpotData> {
//     fn join(&self) -> String {
//         let mut a = String::from("");
//         for (_, v) in self.iter() {
//             a.push_str(format!("{}: {}\n", v.emoji, v.user).as_str())
//         }
//         a
//     }
// }


// #[derive(Debug, PartialEq, Eq, Hash, Clone)]
// struct EmojiData {
//     name: String,
//     id: u64
// }

// #[derive(Debug)]
// enum OpenOrUser {
//     Open(String),
//     User(serenity::User)
// }

// impl std::fmt::Display for OpenOrUser {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             OpenOrUser::Open(a) => write!(f, "{}", a),
//             OpenOrUser::User(a) => write!(f, "{}", a)
//         }
//     }
// }

// #[derive(Debug)]
// struct SpotData {
//     user: OpenOrUser,
//     emoji: serenity::ReactionType,
// }

// impl SpotData {
//     fn default() -> Self {
//         Self{
//             user: OpenOrUser::Open("<open>".to_string()),
//             emoji: '👤'.into(),
//         }
//     }
// }

// #[derive(Debug, poise::Modal)]
// #[allow(dead_code)] // fields only used for Debug print
// struct EditModal {
//     #[name="Run name"]
//     run_name: String,
//     #[name="Squad size"]
//     squad_size: String,
//     #[name="Time"]
//     time: String,
// }
// fn update_number_spots(line_up: &mut IndexMap<usize, SpotData>, new_size: usize, old_size: usize) -> usize {
//     if new_size > old_size {
//         for i in old_size..new_size {
//             line_up.insert(i, SpotData::default());
//         }
//     } else if new_size < old_size {
//         for i in new_size..old_size {
//             // can swap remove here since the last index is getting removed anwyays, so position perturbation doesnt matter
//             line_up.swap_remove(&i);
//         }
//     }
//     new_size
// }

// fn update_available(avail: &mut IndexMap<serenity::User, IndexSet<EmojiData>>, emoji_data: EmojiData, user: serenity::User) {
//     match avail.get_mut(&user) {
//         Some(classes) => {
//             if classes.contains(&emoji_data) {
//                 classes.shift_remove(&emoji_data);
//                 if classes.len() == 0 {
//                     avail.shift_remove(&user);
//                 }
//             } else {
//                 classes.insert(emoji_data);
//             }
//         },
//         None => {let _ = avail.insert(user, IndexSet::from([emoji_data]));},
//     }
// }

// /// Displays your or another user's account creation date
// #[poise::command(slash_command)]
// async fn _schedule(
//     ctx: Context<'_>,
//     #[description = "Run name"] mut name: String,
//     #[description = "Squad size limit"]
//     #[min = 1]
//     #[max = 10] 
//     mut squad_size: usize,
//     #[description = "Run time"] mut time: String,
// ) -> Result<(), Error> {
//     let timestamp = get_timestamp(&time);


//     let mut line_up: IndexMap<usize, SpotData> = IndexMap::with_capacity(squad_size);
//     for i in 0..squad_size {
//         line_up.insert(i, SpotData::default());
//     }

//     let mut available: IndexMap<serenity::User, IndexSet<EmojiData>> = IndexMap::new();

//     // edit button
//     let edit_button = serenity::CreateButton::new("edit_modal")
//         .emoji('🔨');

//     // list of classes
//     let classes = ctx.data().ss_classes.clone().into_keys().collect::<Vec<String>>();
//     // buttons
//     let mut button_vec: Vec<Vec<serenity::CreateButton>> = Vec::from([Vec::with_capacity(4), Vec::with_capacity(4)]);
//     let mut i: usize = 0;
//     for (class, emoji_data) in &ctx.data().ss_classes {
//         let b = serenity::CreateButton::new(class)
//             .emoji(serenity::EmojiId::new(emoji_data.id))
//             .style(serenity::ButtonStyle::Secondary);
//         button_vec[i / 4usize].push(b);
//         i += 1;
//     }

//     // TODO: send this as a normal message, not a reply
//     let base_embed = serenity::CreateEmbed::new()
//         .title(format!("{}\n<t:{}:f>", name, timestamp))
//         // .timestamp(timestamp)
//         .field("", format!("Organiser: {}", ctx.author()), false);
    
//     let starting_embed = base_embed.clone()
//         .field("Selected line up:", line_up.join(), false)
//         .field("The following players are available:", "", false);

//     let reply_handle = ctx.send(poise::CreateReply::default()
//         .embed(starting_embed)
//         .components(vec![
//             serenity::CreateActionRow::Buttons(vec![edit_button]),
//             serenity::CreateActionRow::Buttons(button_vec[0].clone()),
//             serenity::CreateActionRow::Buttons(button_vec[1].clone())
//         ])
//     ).await.unwrap();
//     let msg_id = reply_handle.into_message().await.unwrap().id;

//     while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
//         .message_id(msg_id.clone())
//         .timeout(std::time::Duration::from_secs(10))
//         .custom_ids(vec!["edit_modal".to_string()])
//         .await
//     {
//         let data = poise::execute_modal_on_component_interaction::<EditModal>(
//             ctx,
//             mci.clone(),
//             Some(EditModal{run_name: name.clone(), squad_size: squad_size.to_string(), time: time.clone()}),
//             None
//         ).await.unwrap().unwrap();
//         info!("Got data: {:?}", data);

//         let mut msg = mci.message.clone();
//         let mut updated_embed = base_embed.clone();
        
//         if data.time != time || data.run_name != name {
//             updated_embed = updated_embed.title(
//                 format!("{}\n<t:{}:f>", data.run_name, get_timestamp(&data.time))
//             );
//             time = data.time;
//             name = data.run_name;
//         }

//         // get updated squad_size
//         let sq_size = data.squad_size.parse::<usize>().unwrap();
//         squad_size = update_number_spots(&mut line_up, sq_size, squad_size);

//         updated_embed = updated_embed
//             .field("Selected line up:", &line_up.join(), false)
//             .field("The following players are available:", &available.join(), false);

//         msg.edit(
//             ctx,
//             serenity::EditMessage::new()
//                 .embed(updated_embed)
//         ).await.unwrap();
//     }

//     while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
//         .message_id(msg_id.clone())
//         .timeout(std::time::Duration::from_secs(10))
//         .custom_ids(classes.clone())
//         .await
//     {
//         // get the emoji to add
//         let emoji_data = ctx.data().ss_classes.get(&mci.data.custom_id).expect("Button ID does not match available classes");
//         // check if already exists, update if does, otherwise set
//         update_available(&mut available, emoji_data.clone(), mci.user.clone());

//         let mut msg = mci.message.clone();
//         let updated_embed = base_embed.clone()
//             .field("Selected line up:", &line_up.join(), false)
//             .field("The following players are available:", &available.join(), false);
        
//         msg.edit(
//             ctx,
//             serenity::EditMessage::new()
//                 .embed(updated_embed)
//         ).await.unwrap();
//         mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge).await.unwrap();
//     }

//     Ok(())
// }