use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use indexmap::IndexSet;


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
    // get the emoji information for class clicked
    let class_emoji = class_data.get(&interaction.data.custom_id).unwrap().clone();
    // get user
    let user = interaction.user.clone();

    // handle update/add
    match rinfo.available.get_mut(&user) {
        Some(emoji_set) => {
            if !emoji_set.insert(class_emoji.clone()) {
                emoji_set.swap_remove(&class_emoji);
                if emoji_set.len() == 0 {
                    rinfo.available.swap_remove(&user);
                }
            }
        },
        None => {
            let mut iset: IndexSet<EmojiData> = IndexSet::new();
            iset.insert(class_emoji);
            let _ = rinfo.available.insert(user, iset);
        }
    }

    // update the embed and stored info
    insert_update_runinfo(&ctx.data, &rinfo).await;
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