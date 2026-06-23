use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

use chrono_tz::Tz;

use crate::database::add_update_timezone;


pub fn register() -> CreateCommand {
    CreateCommand::new("set_timezone")
        .description("Set your timezone to use when creating events")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "tz", "Timezon name in IANA format, e.g. `America/New_York`")
                .required(true)
        )
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    // handle the input options
    let tz_str = match interaction.data.options().first() {
        Some(ResolvedOption {value: ResolvedValue::String(tz), ..}) => Some(tz.to_string()),
        _ => None
    };
    let tz_str = tz_str.expect("Unable to get command options");

    // attempt to parse the result
    let msg = match tz_str.parse::<Tz>() {
        Ok(tz) => {
            add_update_timezone(&ctx, &interaction.user, &tz).await;

            format!("Parsed and saved timezone: {}", tz)
        },
        Err(e) => format!("Could not parse timezone [{}]: {}", &tz_str, e),
    };
    
    interaction.create_response(
        ctx,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
            .content(msg)
            .ephemeral(true)
        )
    ).await.unwrap();
    

    Ok(())
}