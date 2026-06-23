use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::CreateQuickModal;

use crate::{RunData};
use crate::runs::time::{get_timestamp, get_datetime};
use crate::database::{get_timezone, insert_update_runinfo};


pub async fn handle_edit(ctx: &Context, interaction: &ComponentInteraction) {
    // try to get the data for the run
    let tz = get_timezone(&ctx, &interaction.user).await;
    let mut rinfo = {
        let data = ctx.data.read().await;
        let runs = data.get::<RunData>().unwrap();
        runs.get(&interaction.message.id.get()).unwrap().clone()
    };
    // check that this user has permission to edit
    if interaction.user != rinfo.organizer {
        let _ = interaction.create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Only the organizer can assign run spots.")
                    .ephemeral(true)
            )
        ).await.unwrap();
        return
    }

    // get the datetime for the edit
    let dt = get_datetime(rinfo.time, tz);

    let modal = CreateQuickModal::new("Edit run")
        .timeout(std::time::Duration::from_secs(60))
        .field(
            CreateInputText::new(InputTextStyle::Short, "Name", "")  // id overwritten by quick modal
                .value(rinfo.name)
        )
        .field(
            CreateInputText::new(InputTextStyle::Short, "# Players", "")
                .value(rinfo.size.to_string())
        )
        .field(
            CreateInputText::new(InputTextStyle::Short, "Time", "")
                .value(dt)
        );
    
    let response = interaction.quick_modal(ctx, modal).await.unwrap().unwrap();
    let inputs = &response.inputs;

    rinfo.name = inputs[0].clone();
    // rinfo.size = inputs[1].parse::<usize>().unwrap();
    rinfo.update_size(inputs[1].parse::<usize>().unwrap());
    rinfo.time = get_timestamp(inputs[2].clone(), tz).unwrap();

    // make the embed for updating the message
    let new_embed = rinfo.make_embed();
    // update the message
    rinfo.channel_id.edit_message(
        ctx,
        &rinfo.msg_id.unwrap(),
        EditMessage::new()
            .embed(new_embed)
    ).await;

    insert_update_runinfo(&ctx, &rinfo).await;
    {
        let mut data = ctx.data.write().await;
        let runs = data.get_mut::<RunData>().unwrap();
        let _ = runs.insert(rinfo.msg_id.unwrap().get(), rinfo);
    }

    // acknowledge modal responses
    &response
        .interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Acknowledge
        ).await;
}