use std::time::Duration;

use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::{RunData, ClassData};
use crate::runs::{SpotData, OpenOrUser};
use crate::database::insert_update_runinfo;



pub async fn handle_assign_spots(ctx: &Context, interaction: &ComponentInteraction) {
    let (mut rinfo, class_data) = {
        let data = ctx.data.read().await;
        let runs = data.get::<RunData>().unwrap();
        let class_data = data.get::<ClassData>().unwrap();
        (runs.get(&interaction.message.id.get()).unwrap().clone(), class_data.clone())
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

    // setup the select menus
    let menu_spot = CreateSelectMenu::new(
        "spot_n",
        CreateSelectMenuKind::String {
            options: (0..rinfo.size).map(|v| CreateSelectMenuOption::new((v+1).to_string(), v.to_string())).collect()
        }
    );
    let menu_player = CreateSelectMenu::new(
        "player",
        CreateSelectMenuKind::String {
            options: rinfo.available.iter().map(
                |(k, _)| CreateSelectMenuOption::new(k.display_name().to_string(), k.id.get().to_string())
            ).collect()
        }
    );

    // base statement in the response
    let base_msg = "Select the spot, then player, then class to assign. Message will disappear after 60s of no interaction.".to_string();

    let _ = interaction.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(base_msg.clone())
                .select_menu(menu_spot.clone())
                .ephemeral(true)
        )
    ).await.unwrap();

    let msg = interaction.get_response(ctx).await.unwrap();

    // create base variables to modify/save state
    let mut spot: Option<usize> = None;
    let mut player: Option<User> = None;
    let mut class_name: Option<String>;

    while let Some(msg_int) = msg.await_component_interactions(ctx).timeout(Duration::from_secs(60)).await {
        if &msg_int.data.custom_id == "spot_n" {
            spot = match &msg_int.data.kind {
                ComponentInteractionDataKind::StringSelect{values: a} => a[0].parse::<usize>().ok(),
                _ => None
            };

            // acknowledge the select menu response
            let _ = &msg_int
            .create_response(
                ctx,
                CreateInteractionResponse::Acknowledge
            ).await.unwrap();

            // edit the interaction response
            interaction.edit_response(
                ctx,
                EditInteractionResponse::new()
                    .content(format!("{}\n\n**Spot**: {}", base_msg.clone(), spot.unwrap() + 1))
                    .select_menu(menu_player.clone())
            ).await
            .unwrap();
        } else if &msg_int.data.custom_id == "player" {
            player = match &msg_int.data.kind {
                ComponentInteractionDataKind::StringSelect{values: a} => UserId::new(a[0].parse::<u64>().unwrap()).to_user(ctx).await.ok(),
                _ => None
            };

            // create the class selection menu, first have to get the user/id
            let avail_classes = &rinfo.available.get(&player.clone().unwrap()).unwrap();
            let menu_class = CreateSelectMenu::new(
                "class",
                CreateSelectMenuKind::String {
                    options: avail_classes.iter().map(
                        |(k, v)| CreateSelectMenuOption::new(k.to_string(), k.to_string()).emoji(EmojiId::new(v.id))
                    ).collect()
                }
            );

            // acknowledge the select menu response
            let _ = &msg_int
            .create_response(
                ctx,
                CreateInteractionResponse::Acknowledge
            ).await.unwrap();
            // edit the interaction response
            interaction.edit_response(
                ctx,
                EditInteractionResponse::new()
                    .content(format!("{}\n\n**Spot**: {}\n**Player**: {}", base_msg.clone(), spot.unwrap() + 1, player.clone().unwrap()))
                    .select_menu(menu_class)
            ).await
            .unwrap();
        } else if &msg_int.data.custom_id == "class" {
            class_name = match &msg_int.data.kind {
                ComponentInteractionDataKind::StringSelect{values: a} => Some(a[0].to_string()),
                _ => None
            };

            // get the class data
            let edata = class_data.get(&class_name.unwrap()).unwrap();

            // update the lineup
            rinfo.line_up.insert(
                spot.unwrap(),
                SpotData{
                    user: OpenOrUser::User(player.clone().unwrap()),
                    emoji: ReactionType::Custom{animated: false, id: EmojiId::new(edata.id), name: Some(edata.name.clone())}
                }
            );
            // make the embed for updating the message
            let new_embed = rinfo.make_embed();
            // update the message
            rinfo.channel_id.edit_message(
                ctx,
                &rinfo.msg_id.unwrap(),
                EditMessage::new()
                    .embed(new_embed)
            ).await.unwrap();

            // update the database and memory storage for the run
            insert_update_runinfo(&ctx.data, &rinfo).await;
            {
                let mut data = ctx.data.write().await;
                let runs = data.get_mut::<RunData>().unwrap();
                let _ = runs.insert(rinfo.msg_id.unwrap().get(), rinfo.clone());
            }

            // acknowledge the select menu response
            let _ = &msg_int
            .create_response(
                ctx,
                CreateInteractionResponse::Acknowledge
            ).await.unwrap();
            // reset the stored values
            spot = None;
            player = None;
            // edit the interaction response so that another spot can be assigned
            interaction.edit_response(
                ctx,
                EditInteractionResponse::new()
                    .content(base_msg.clone())
                    .select_menu(menu_spot.clone())
            ).await
            .unwrap();
        } else {
            // TODO: update to a message saying interaction not understood
            let _ = &msg_int
            .create_response(
                ctx,
                CreateInteractionResponse::Acknowledge
            ).await.unwrap();
        }
        
    }
}
