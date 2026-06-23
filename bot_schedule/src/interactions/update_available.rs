use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use indexmap::IndexMap;


use crate::{RunData, ClassData};
use crate::database::insert_update_runinfo;
use crate::runs::EmojiData;


pub async fn handle_update_available(ctx: &Context, interaction: &ComponentInteraction) {
    // try to get the data for the run
    let (mut rinfo, class_data) = {
        let data = ctx.data.read().await;
        let runs = data.get::<RunData>().unwrap();
        let cdata = data.get::<ClassData>().unwrap().clone();
        (runs.get(&interaction.message.id.get()).unwrap().clone(), cdata)
    };
    // short var name for full class name
    let full_name = &interaction.data.custom_id.to_string();
    // get the emoji information for class clicked
    let class_emoji = class_data.get(full_name).unwrap().clone();
    // get user
    let user = interaction.user.clone();
    

    // handle update/add
    match rinfo.available.get_mut(&user) {
        Some(emoji_set) => {
            match emoji_set.insert(full_name.clone(), class_emoji.clone()) {
                Some(_) => {
                    emoji_set.swap_remove(full_name);
                    if emoji_set.len() == 0 {
                        rinfo.available.swap_remove(&user);
                    }
                },
                None => {}
            }
        },
        None => {
            let mut iset: IndexMap<String, EmojiData> = IndexMap::new();
            iset.insert(full_name.clone(), class_emoji);
            let _ = rinfo.available.insert(user, iset);
        }
    }

    // update the embed and stored info
    insert_update_runinfo(&ctx, &rinfo).await;
    let embed = rinfo.make_embed();
    {
        let mut data = ctx.data.write().await;
        let runs = data.get_mut::<RunData>().unwrap();
        let _ = runs.insert(rinfo.msg_id.unwrap().get(), rinfo.clone());
    }

    let mut message = interaction.message.clone();

    message
        .edit(
            ctx,
            EditMessage::new()
                .embed(embed)
        ).await.unwrap();

    // acknowledge interaction
    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Acknowledge
        ).await.unwrap();
}