use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

// use chrono_tz::Tz;

use crate::ClassData;
use crate::{RunData};
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

    let tz = get_timezone(&ctx.data, &interaction.user).await;

    let mut ri = RunInfo::new(interaction, name.to_string(), players, time.to_string(), tz);

    let embed = ri.make_embed();

    let _ = interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Posting custom run...").ephemeral(true))
        )
        .await
        .unwrap();
    
    let class_data = {
        let data = ctx.data.read().await;
        data.get::<ClassData>().unwrap().clone()
    };

    let mut content = CreateMessage::new()
        .embed(embed)
        .button(
            CreateButton::new("edit_button")
                .emoji('🔨')
        );
    for (k, v) in class_data.iter() {
        content = content.button(
            CreateButton::new(k)
                .emoji(EmojiId::new(v.id))
                .style(ButtonStyle::Secondary)
        );
    }
    
    let msg = interaction
        .channel_id
        .send_message(
            ctx,
            content
        )
        .await
        .unwrap();
    
    ri.set_message_id(msg.clone());

    insert_update_runinfo(&ctx.data, &ri).await;

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