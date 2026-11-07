pub mod mafia;

use mafia::GameSession;
use poise::{serenity_prelude as serenity, PrefixFrameworkOptions};
use serenity::all::{ChannelId, CreateMessage, UserId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state across command invocations.
pub struct Data {
    pub active_channels: Arc<Mutex<HashSet<ChannelId>>>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// RAII Guard ensuring active channels are removed from the active set upon completion or error.
struct ActiveGameGuard {
    channel_id: ChannelId,
    active_channels: Arc<Mutex<HashSet<ChannelId>>>,
}

impl Drop for ActiveGameGuard {
    fn drop(&mut self) {
        let active = self.active_channels.clone();
        let ch = self.channel_id;
        tokio::spawn(async move {
            let mut set = active.lock().await;
            set.remove(&ch);
        });
    }
}

/// Explains the rules, scoring, and gameplay of Rocket League Mafia.
#[poise::command(slash_command, prefix_command)]
async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title("⚽ Rocket League Mafia: Rules & Guide 🕵️")
        .description(
            "**Rocket League Mafia** brings the classic social deduction game into Rocket League!\n\n\
             **How It Works:**\n\
             1. Join a voice channel with **6 players** (3v3) or **2 players** (1v1 testing mode).\n\
             2. Run `/new` to launch the game.\n\
             3. Teams are randomly split into 🟦 **Blue** and 🟧 **Orange**.\n\
             4. **One player is secretly chosen as the Mafia**; everyone else is a **Villager** (roles sent via DM).\n\
             5. Play the Rocket League match while chatting together in voice!\n\
             6. Report the winning team using the buttons on the gameplay message.\n\
             7. Everyone casts a private vote via DM within **1 minute** on who they believe the Mafia is (the Mafia submits a dummy vote).\n\
             8. The Mafia is revealed, round points are tallied, and the leaderboard updates live!\n\n\
             **Point System:**\n\
             • **Villagers:**\n\
               - Win the match with the Mafia on your team: **+1 pt**\n\
               - Correctly vote for the Mafia: **+1 pt**\n\
             • **Mafia:**\n\
               - Lose the match with **ZERO votes**: **+3 pts**\n\n\
             *Every other scenario awards 0 points for both roles.*\n\n\
             **Commands:**\n\
             • `/new`: Start a new game with the people in your voice channel.\n\
             • `/cancel`: Force cancel an ongoing game in this channel.\n\
             • `/rules`: Quick summary of the scoring rules.\n\
             • `/help`: Displays this guide.",
        )
        .color(mafia::config::COLOR_BLUE);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Quick reference for the Rocket League Mafia scoring system.
#[poise::command(slash_command, prefix_command)]
async fn rules(ctx: Context<'_>) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title("📋 Rocket League Mafia: Scoring Rules")
        .description(
            "**For Villagers:**\n\
             • Win the match with Mafia on your team: **+1 pt**\n\
             • Correctly vote for the Mafia: **+1 pt**\n\n\
             **For the Mafia:**\n\
             • Lose the match with **NO votes**: **+3 pts**\n\n\
             *Every other scenario awards 0 points for both roles.*",
        )
        .color(mafia::config::COLOR_GOLD);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Force-cancels an active game in this channel if abandoned or stuck.
#[poise::command(slash_command, prefix_command)]
async fn cancel(ctx: Context<'_>) -> Result<(), Error> {
    let channel_id = ctx.channel_id();
    let mut active = ctx.data().active_channels.lock().await;
    if active.remove(&channel_id) {
        ctx.say("🛑 The active game in this channel has been cancelled.")
            .await?;
    } else {
        ctx.say("ℹ️ There is no active game in this channel.")
            .await?;
    }
    Ok(())
}

/// Create a new game of Rocket League Mafia with the players in your voice channel.
#[poise::command(slash_command, prefix_command)]
async fn new(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild) = ctx.guild().map(|g| g.clone()) else {
        ctx.say("❌ This command must be used in a server.").await?;
        return Ok(());
    };

    let user = ctx.author();

    // Check if the author is in a voice channel
    let user_vc = guild
        .voice_states
        .get(&user.id)
        .and_then(|vs| vs.channel_id);

    let Some(user_vc) = user_vc else {
        ctx.say("❌ You must be in a voice channel to start a game!")
            .await?;
        return Ok(());
    };

    // Filter voice states for users in the author's voice channel (excluding bots)
    let mut player_ids: Vec<UserId> = Vec::new();
    for (uid, vs) in &guild.voice_states {
        if vs.channel_id == Some(user_vc) {
            let cached_bot = ctx.cache().user(*uid).map(|u| u.bot);
            let is_bot = match cached_bot {
                Some(bot) => bot,
                None => {
                    if let Ok(member) = guild.id.member(&ctx.http(), *uid).await {
                        member.user.bot
                    } else {
                        false
                    }
                }
            };

            if !is_bot {
                player_ids.push(*uid);
            }
        }
    }

    let size = player_ids.len();
    if size != 2 && size != 6 {
        let mentions: Vec<String> = player_ids.iter().map(|id| format!("<@{id}>")).collect();
        let msg = format!(
            "❌ Rocket League Mafia requires exactly **6 players** (3v3) or **2 players** (1v1 testing mode).\n\
             Found **{size}** player(s) in your voice channel: {}",
            if mentions.is_empty() {
                "None".to_string()
            } else {
                mentions.join(", ")
            }
        );
        ctx.say(msg).await?;
        return Ok(());
    }

    let channel_id = ctx.channel_id();

    // Prevent overlapping games in the same channel
    {
        let mut active = ctx.data().active_channels.lock().await;
        if active.contains(&channel_id) {
            ctx.say("❌ A Rocket League Mafia game is already active in this channel! Use `/cancel` or finish the game first.")
                .await?;
            return Ok(());
        }
        active.insert(channel_id);
    }

    let _guard = ActiveGameGuard {
        channel_id,
        active_channels: ctx.data().active_channels.clone(),
    };

    // Gather display names safely
    let mut player_names = HashMap::with_capacity(player_ids.len());
    for &uid in &player_ids {
        let name = if let Ok(member) = guild.id.member(&ctx.http(), uid).await {
            member.display_name().to_string()
        } else if let Some(name) = ctx.cache().user(uid).map(|u| u.name.clone()) {
            name
        } else {
            format!("Player {uid}")
        };
        player_names.insert(uid, name);
    }

    // Message 1: Public Leaderboard
    let initial_scores: HashMap<UserId, u32> = player_ids.iter().map(|&id| (id, 0)).collect();
    let leaderboard_embed = mafia::build_leaderboard_embed(&player_ids, &initial_scores, false);
    let reply = ctx
        .send(poise::CreateReply::default().embed(leaderboard_embed))
        .await?;
    let leaderboard_msg = reply.message().await?;
    let leaderboard_msg_id = leaderboard_msg.id;

    // Message 2: Public Gameplay view
    let starting_embed = serenity::CreateEmbed::new()
        .title("⚽ Initialising Rocket League Mafia...")
        .description("Assigning secret roles and randomising teams...")
        .color(mafia::config::COLOR_BLUE);
    let gameplay_msg = channel_id
        .send_message(&ctx.http(), CreateMessage::new().embed(starting_embed))
        .await?;
    let gameplay_msg_id = gameplay_msg.id;

    // Run the full game session controller
    let mut session = GameSession::new(
        ctx,
        channel_id,
        leaderboard_msg_id,
        gameplay_msg_id,
        user.id,
        player_ids,
        player_names,
    );

    session.run().await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").unwrap_or_else(|_| {
        include_str!("BotToken.txt")
            .split_whitespace()
            .next()
            .expect("BotToken.txt is empty")
            .to_string()
    });

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_VOICE_STATES
        | serenity::GatewayIntents::GUILDS;

    let prefix_options = PrefixFrameworkOptions {
        prefix: Some("!!".into()),
        ..Default::default()
    };

    let active_channels = Arc::new(Mutex::new(HashSet::new()));
    let active_channels_clone = active_channels.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![help(), rules(), cancel(), new()],
            prefix_options,
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                println!("Rocket League Mafia bot is ready!");
                Ok(Data {
                    active_channels: active_channels_clone,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client
        .expect("Err creating client")
        .start()
        .await
        .expect("Client error");
}
