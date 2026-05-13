use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::CreateQuickModal;

use crate::{DbConnection, RunData};
use crate::runs::time::get_datetime;
use crate::database::{get_timezone, insert_update_runinfo};


pub async fn handle_edit(ctx: &Context, interaction: &ComponentInteraction) {
    // try to get the data for the run
    let (rinfo_ro, tz) = {
        let data = ctx.data.read().await;
        let runs = data.get::<RunData>().unwrap();

        let pool = data.get::<DbConnection>().unwrap();
        let tz = get_timezone(pool, &interaction.user);

        (runs.get(&interaction.message.id.get()).unwrap().clone(), tz)
    };
    // get the datetime for the edit
    let dt = get_datetime(rinfo_ro.time, tz);

    let modal = CreateQuickModal::new("Edit run")
        .timeout(std::time::Duration::from_secs(60))
        .field(
            CreateInputText::new(InputTextStyle::Short, "Name", "")  // id overwritten by quick modal
                .value(rinfo_ro.name)
        )
        .field(
            CreateInputText::new(InputTextStyle::Short, "# Players", "")
                .value(rinfo_ro.size.to_string())
        )
        .field(
            CreateInputText::new(InputTextStyle::Short, "Time", "")
                .value(dt)
        );
    
    let response = interaction.quick_modal(ctx, modal).await.unwrap().unwrap();
    let inputs = &response.inputs;
    // ["Steve", "4", "2026-05-13 20:00"]
    println!("{:?}", inputs);

    // acknowledge modal responses
    &response
        .interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Acknowledge
        ).await;
}