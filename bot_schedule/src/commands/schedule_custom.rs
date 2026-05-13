use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

// use chrono_tz::Tz;

use crate::{DbConnection, RunData};
use crate::runs::RunInfo;
use crate::database::{get_timezone, insert_update_runinfo};

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    // handle the input options
    let (name, players, time) = if let [
        ResolvedOption {value: ResolvedValue::String(name), ..},
        ResolvedOption {value: ResolvedValue::Integer(players), ..},
        ResolvedOption {value: ResolvedValue::String(time), ..}
    ] = interaction.data.options()[..] {
        Some((name, players as usize, time))
    } else {
        None
    }.expect("Unable to get command options");

    let tz = {
        let data = ctx.data.read().await;
        let pool = data.get::<DbConnection>().unwrap();
        get_timezone(&pool, &interaction.user)
    };

    let mut ri = RunInfo::new(interaction, name.to_string(), players, time.to_string(), tz);

    let embed = ri.make_embed();

    let _ = interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Posting custom run...").ephemeral(true))
        )
        .await
        .unwrap();
    
    let msg = interaction
        .channel_id
        .send_message(
            ctx,
            CreateMessage::new()
                .embed(embed)
                .button(
                    CreateButton::new("edit_button")
                        .emoji('🔨')
                )
        )
        .await
        .unwrap();
    
    ri.set_message_id(msg.clone());

    {
        let data = ctx.data.read().await;
        let pool = data.get::<DbConnection>().unwrap();
        insert_update_runinfo(pool, &ri);
    }

    {
        let mut data = ctx.data.write().await;
        let imap = data.get_mut::<RunData>().unwrap();
        imap.insert(msg.id.get(), ri);
    }

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("custom")
        .description("Schedule a custom event")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "name", "Name of the event to schedule")
                .required(true)
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "players", "Number of player slots for the event")
                .min_int_value(1)
                .max_int_value(10)
                .required(true)
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "time", "Time of the event")
                .required(true)
        )
}