use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::CreateQuickModal;

use crate::{DbConnection, RunData};
use crate::runs::time::{get_timestamp, get_datetime};
use crate::database::{get_timezone, insert_update_runinfo};


pub async fn handle_edit(ctx: &Context, interaction: &ComponentInteraction) {
    // try to get the data for the run
    let (mut rinfo, tz) = {
        let data = ctx.data.read().await;
        let runs = data.get::<RunData>().unwrap();

        let pool = data.get::<DbConnection>().unwrap();
        let tz = get_timezone(pool, &interaction.user);

        (runs.get(&interaction.message.id.get()).unwrap().clone(), tz)
    };
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

    {
        let mut data = ctx.data.write().await;

        let pool = data.get_mut::<DbConnection>().unwrap();
        let tz = get_timezone(pool, &interaction.user);

        rinfo.name = inputs[0].clone();
        rinfo.size = inputs[1].parse::<usize>().unwrap();
        rinfo.time = get_timestamp(inputs[2].clone(), tz).unwrap();

        let runs = data.get_mut::<RunData>().unwrap();
        insert_update_runinfo(pool, &rinfo);
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