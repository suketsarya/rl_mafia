use poise::serenity_prelude as serenity;
use rand::prelude::*;
use serenity::all::{
    ButtonStyle, ChannelId, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, EditMessage,
    MessageId, UserId,
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// ---------------------------------------------------------
// Configuration & Constants
// ---------------------------------------------------------

pub mod config {
    use std::time::Duration;

    /// 1 minute voting timeout as specified.
    pub const VOTING_TIMEOUT: Duration = Duration::from_secs(60);
    /// 1 hour match duration timeout.
    pub const MATCH_TIMEOUT: Duration = Duration::from_secs(3600);
    /// 1 minute confirmation gate timeout.
    pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(60);
    /// 10 minutes intermission timeout between rounds.
    pub const INTERMISSION_TIMEOUT: Duration = Duration::from_secs(600);

    pub const COLOR_BLUE: u32 = 0x3498DB;
    pub const COLOR_ORANGE: u32 = 0xE67E22;
    pub const COLOR_MAFIA: u32 = 0xED4245;
    pub const COLOR_SUCCESS: u32 = 0x2ECC71;
    pub const COLOR_GOLD: u32 = 0xF1C40F;
    pub const COLOR_VOTING: u32 = 0xF39C12;
}

// ---------------------------------------------------------
// Domain Models
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Villager,
    Mafia,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Villager => write!(f, "Villager"),
            Self::Mafia => write!(f, "Mafia"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    Blue,
    Orange,
}

impl Team {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Orange => "Orange",
        }
    }

    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::Blue => "🟦",
            Self::Orange => "🟧",
        }
    }
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} Team", self.emoji(), self.name())
    }
}

#[derive(Debug, Clone)]
pub struct PlayerRoundInfo {
    pub id: UserId,
    pub role: Role,
    pub team: Team,
}

#[derive(Debug, Clone)]
pub struct RoundResult {
    pub round_number: u32,
    pub mafia_id: UserId,
    pub winning_team: Team,
    pub votes: HashMap<UserId, Option<UserId>>,
    pub points_awarded: HashMap<UserId, u32>,
    pub reasons: HashMap<UserId, Vec<String>>,
}

enum MatchOutcome {
    Winner(Team),
    EndGame,
    TimedOut,
}

enum VotingOutcome {
    Votes(HashMap<UserId, Option<UserId>>),
    EndGame,
}

enum IntermissionOutcome {
    NextRound,
    EndGame,
    TimedOut,
}

// ---------------------------------------------------------
// View & Component Builders
// ---------------------------------------------------------

pub fn build_leaderboard_embed(
    player_ids: &[UserId],
    scores: &HashMap<UserId, u32>,
    is_final: bool,
) -> CreateEmbed {
    let mut ranked: Vec<UserId> = player_ids.to_vec();
    ranked.sort_by(|a, b| {
        let sa = scores.get(a).copied().unwrap_or(0);
        let sb = scores.get(b).copied().unwrap_or(0);
        sb.cmp(&sa)
    });

    let mut player_col = String::new();
    let mut points_col = String::new();

    for (i, uid) in ranked.iter().enumerate() {
        let score = scores.get(uid).copied().unwrap_or(0);
        player_col.push_str(&format!("<@{uid}>\n"));
        points_col.push_str(&format!(
            "{score} pt{}\n",
            if score == 1 { "" } else { "s" }
        ));
    }

    let title = if is_final {
        "🏆 Rocket League Mafia: Final Leaderboard 🏆"
    } else {
        "📊 Rocket League Mafia: Leaderboard"
    };

    let footer_text = if is_final {
        "Game finished. Thank you for playing!"
    } else {
        "Leaderboard updates automatically after every round."
    };

    CreateEmbed::new()
        .title(title)
        .field("Player", player_col, true)
        .field("Points", points_col, true)
        .color(if is_final {
            config::COLOR_GOLD
        } else {
            config::COLOR_BLUE
        })
        .footer(CreateEmbedFooter::new(footer_text))
}

pub fn build_match_embed(round: u32, players: &[PlayerRoundInfo]) -> CreateEmbed {
    let mut blue_members = Vec::new();
    let mut orange_members = Vec::new();

    for p in players {
        let entry = format!("• <@{}>", p.id);
        if p.team == Team::Blue {
            blue_members.push(entry);
        } else {
            orange_members.push(entry);
        }
    }

    let blue_text = if blue_members.is_empty() {
        "None".to_string()
    } else {
        blue_members.join("\n")
    };

    let orange_text = if orange_members.is_empty() {
        "None".to_string()
    } else {
        orange_members.join("\n")
    };

    CreateEmbed::new()
        .title(format!("⚽ Round {round}: Match in Progress"))
        .description(
            "🎭 **Roles have been secretly sent via Direct Message!**\n\
             One player is the **Mafia**; everyone else is a **Villager**.\n\n\
             🎮 Hop into Rocket League, play the match, and once finished, click the winning team below!",
        )
        .field("🟦 Blue Team", blue_text, true)
        .field("🟧 Orange Team", orange_text, true)
        .color(config::COLOR_BLUE)
        .footer(CreateEmbedFooter::new(
            "Click a button below once the Rocket League match is over.",
        ))
}

pub fn build_match_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new("report_blue")
            .label("🟦 Blue Won")
            .style(ButtonStyle::Secondary),
        CreateButton::new("report_orange")
            .label("🟧 Orange Won")
            .style(ButtonStyle::Secondary),
        CreateButton::new("request_end")
            .label("🛑 End Game")
            .style(ButtonStyle::Secondary),
    ])]
}

pub fn build_confirm_end_embed() -> CreateEmbed {
    CreateEmbed::new()
        .title("⚠️ End Game Confirmation")
        .description(
            "Are you sure you want to end the game?\n\n\
             This will immediately conclude the session and finalise the leaderboard.",
        )
        .color(config::COLOR_MAFIA)
}

pub fn build_confirm_end_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new("confirm_end")
            .label("✅ Yes, End Game")
            .style(ButtonStyle::Secondary),
        CreateButton::new("cancel_end")
            .label("❌ Cancel (Keep Playing)")
            .style(ButtonStyle::Secondary),
    ])]
}

pub fn build_voting_embed(
    round: u32,
    winning_team: Team,
    player_ids: &[UserId],
    votes: &HashMap<UserId, Option<UserId>>,
) -> CreateEmbed {
    let mut status_lines = Vec::new();
    let voted_count = votes.len();
    let total_count = player_ids.len();

    for uid in player_ids {
        if votes.contains_key(uid) {
            status_lines.push(format!("✅ <@{uid}>: *Voted*"));
        } else {
            status_lines.push(format!("⏳ <@{uid}>: *Waiting...*"));
        }
    }

    CreateEmbed::new()
        .title(format!("🗳️ Round {round}: Voting Phase"))
        .description(format!(
            "🏆 **Match Winner:** {winning_team}\n\n\
             Check your **Direct Messages** to cast your vote for who you think the Mafia is!\n\
             *(The Mafia must also cast a dummy vote to avoid suspicion).*\n\n\
             **Voting Progress ({voted_count} / {total_count}):**\n\
             {}\n\n\
             ⏱️ *Voting closes in 1 minute!*",
            status_lines.join("\n")
        ))
        .color(config::COLOR_VOTING)
        .footer(CreateEmbedFooter::new(
            "Waiting for all players to submit their vote in DMs.",
        ))
}

pub fn build_voting_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![CreateButton::new(
        "request_end",
    )
    .label("🛑 End Game")
    .style(ButtonStyle::Secondary)])]
}

pub fn build_results_embed(
    result: &RoundResult,
    total_scores: &HashMap<UserId, u32>,
) -> CreateEmbed {
    let mut votes_text = String::new();
    for (voter_id, suspect_opt) in &result.votes {
        if *voter_id == result.mafia_id {
            votes_text.push_str(&format!("• <@{voter_id}>: *Submitted dummy vote*\n"));
        } else if let Some(suspect_id) = suspect_opt {
            let correct = *suspect_id == result.mafia_id;
            votes_text.push_str(&format!(
                "• <@{voter_id}> voted for <@{suspect_id}> ({})\n",
                if correct {
                    "✅ Correct!"
                } else {
                    "❌ Incorrect"
                }
            ));
        } else {
            votes_text.push_str(&format!("• <@{voter_id}>: *Did not vote*\n"));
        }
    }

    let mut points_text = String::new();
    for (pid, pts) in &result.points_awarded {
        let total = total_scores.get(pid).copied().unwrap_or(0);
        let reasons = result
            .reasons
            .get(pid)
            .map(|r| r.join(", "))
            .unwrap_or_default();
        points_text.push_str(&format!(
            "• <@{pid}>: **+{pts} pt{}** ({reasons}) | Total: **{total} pt{}**\n",
            if *pts == 1 { "" } else { "s" },
            if total == 1 { "" } else { "s" }
        ));
    }

    CreateEmbed::new()
        .title(format!("🎉 Round {}: Results", result.round_number))
        .description(format!(
            "🕵️ **The Mafia was:** <@{}>!\n\
             🏆 **Match Winner:** {}",
            result.mafia_id, result.winning_team
        ))
        .field("🗳️ Votes Cast", votes_text, false)
        .field("⭐ Points Earned This Round", points_text, false)
        .color(config::COLOR_SUCCESS)
        .footer(CreateEmbedFooter::new(
            "Leaderboard has been updated above! Click Next Round to continue.",
        ))
}

pub fn build_results_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new("next_round")
            .label("▶️ Next Round")
            .style(ButtonStyle::Success),
        CreateButton::new("request_end")
            .label("🏁 End Game")
            .style(ButtonStyle::Secondary),
    ])]
}

pub fn build_final_game_embed(player_ids: &[UserId], scores: &HashMap<UserId, u32>) -> CreateEmbed {
    let mut ranked: Vec<UserId> = player_ids.to_vec();
    ranked.sort_by(|a, b| {
        let sa = scores.get(a).copied().unwrap_or(0);
        let sb = scores.get(b).copied().unwrap_or(0);
        sb.cmp(&sa)
    });

    let winner_text = if let Some(winner_id) = ranked.first() {
        let win_score = scores.get(winner_id).copied().unwrap_or(0);
        format!("🏆 Congratulations <@{winner_id}> for winning with **{win_score} points**!")
    } else {
        "Game concluded!".to_string()
    };

    CreateEmbed::new()
        .title("🏁 Game Over: Rocket League Mafia")
        .description(format!(
            "{winner_text}\n\nCheck the finalised leaderboard above for full standings."
        ))
        .color(config::COLOR_GOLD)
        .footer(CreateEmbedFooter::new(
            "Thank you for playing Rocket League Mafia!",
        ))
}

pub fn build_dm_vote_components(
    voter_id: UserId,
    candidates: &[UserId],
    names: &HashMap<UserId, String>,
) -> Vec<CreateActionRow> {
    let mut rows = Vec::new();
    let eligible: Vec<UserId> = candidates
        .iter()
        .copied()
        .filter(|&id| id != voter_id)
        .collect();

    for chunk in eligible.chunks(5) {
        let mut buttons = Vec::new();
        for &cid in chunk {
            let label = names
                .get(&cid)
                .cloned()
                .unwrap_or_else(|| format!("Player {cid}"));
            let safe_label: String = label.chars().take(80).collect();
            buttons.push(
                CreateButton::new(format!("vote:{cid}"))
                    .label(safe_label)
                    .style(ButtonStyle::Secondary),
            );
        }
        rows.push(CreateActionRow::Buttons(buttons));
    }

    rows
}

// ---------------------------------------------------------
// Scoring Logic
// ---------------------------------------------------------

/// Pure function to calculate round points according to Rocket League Mafia rules.
///
/// Rules:
/// - Villagers:
///   * Win match with Mafia on team => +1
///   * Correctly vote for Mafia => +1
/// - Mafia:
///   * Lose match with NO votes => +3
/// - All other outcomes: 0 points.
pub fn calculate_round_points(
    mafia_id: UserId,
    mafia_team: Team,
    winning_team: Team,
    player_ids: &[UserId],
    player_teams: &HashMap<UserId, Team>,
    votes: &HashMap<UserId, Option<UserId>>,
) -> (HashMap<UserId, u32>, HashMap<UserId, Vec<String>>) {
    let mut points = HashMap::new();
    let mut reasons = HashMap::new();

    let mafia_lost = mafia_team != winning_team;
    // Count how many villagers voted for the mafia (excluding mafia's dummy vote)
    let votes_for_mafia = votes
        .iter()
        .filter(|(voter, suspect)| **voter != mafia_id && **suspect == Some(mafia_id))
        .count();

    for &pid in player_ids {
        let mut p_points = 0;
        let mut p_reasons = Vec::new();

        if pid == mafia_id {
            if mafia_lost && votes_for_mafia == 0 {
                p_points += 3;
                p_reasons.push("Mafia lost match with 0 votes (+3)".to_string());
            } else if mafia_lost {
                p_reasons.push(format!(
                    "Mafia lost match but received {votes_for_mafia} vote(s) (+0)"
                ));
            } else {
                p_reasons.push("Mafia's team won match (+0)".to_string());
            }
        } else {
            let p_team = player_teams.get(&pid).copied().unwrap_or(Team::Blue);

            // 1) Win the game with Mafia on your team => +1
            if p_team == mafia_team && p_team == winning_team {
                p_points += 1;
                p_reasons.push("Won match with Mafia on team (+1)".to_string());
            }

            // 2) Correctly vote for the Mafia => +1
            if votes.get(&pid) == Some(&Some(mafia_id)) {
                p_points += 1;
                p_reasons.push("Correctly voted for Mafia (+1)".to_string());
            }

            if p_reasons.is_empty() {
                p_reasons.push("0 points".to_string());
            }
        }

        points.insert(pid, p_points);
        reasons.insert(pid, p_reasons);
    }

    (points, reasons)
}

// ---------------------------------------------------------
// Game Session Controller
// ---------------------------------------------------------

pub struct GameSession<'a, D, E> {
    pub ctx: poise::Context<'a, D, E>,
    pub channel_id: ChannelId,
    pub leaderboard_msg_id: MessageId,
    pub gameplay_msg_id: MessageId,
    pub host: UserId,
    pub player_ids: Vec<UserId>,
    pub player_names: HashMap<UserId, String>,
    pub total_scores: HashMap<UserId, u32>,
    pub round: u32,
}

impl<'a, D, E> GameSession<'a, D, E> {
    pub fn new(
        ctx: poise::Context<'a, D, E>,
        channel_id: ChannelId,
        leaderboard_msg_id: MessageId,
        gameplay_msg_id: MessageId,
        host: UserId,
        player_ids: Vec<UserId>,
        player_names: HashMap<UserId, String>,
    ) -> Self {
        let total_scores = player_ids.iter().map(|&id| (id, 0)).collect();
        Self {
            ctx,
            channel_id,
            leaderboard_msg_id,
            gameplay_msg_id,
            host,
            player_ids,
            player_names,
            total_scores,
            round: 1,
        }
    }

    fn is_authorized(&self, user_id: UserId) -> bool {
        self.player_ids.contains(&user_id) || user_id == self.host
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            // Step 1: Assign roles & randomize teams
            let mafia_id = *self.player_ids.choose(&mut rand::thread_rng()).unwrap();

            let mut shuffled = self.player_ids.clone();
            shuffled.shuffle(&mut rand::thread_rng());
            let half = shuffled.len() / 2;
            let blue_ids = &shuffled[..half];

            let mut player_round_info = Vec::with_capacity(self.player_ids.len());
            let mut player_teams = HashMap::with_capacity(self.player_ids.len());

            for &pid in &self.player_ids {
                let role = if pid == mafia_id {
                    Role::Mafia
                } else {
                    Role::Villager
                };
                let team = if blue_ids.contains(&pid) {
                    Team::Blue
                } else {
                    Team::Orange
                };

                player_teams.insert(pid, team);
                player_round_info.push(PlayerRoundInfo {
                    id: pid,
                    role,
                    team,
                });
            }

            // Step 2: Send confidential role DMs
            self.send_role_dms(&player_round_info).await;

            // Step 3: Display match in progress and wait for report
            let match_embed = build_match_embed(self.round, &player_round_info);
            let match_components = build_match_components();
            self.edit_gameplay_msg(match_embed.clone(), match_components.clone())
                .await;

            let winning_team = match self
                .wait_for_match_outcome(&match_embed, &match_components)
                .await
            {
                MatchOutcome::Winner(team) => team,
                MatchOutcome::EndGame | MatchOutcome::TimedOut => {
                    self.finalize_game().await;
                    return Ok(());
                }
            };

            // Step 4: Voting phase (1 minute timeout)
            let votes = match self
                .run_voting_phase(self.round, winning_team, mafia_id, &player_teams)
                .await
            {
                VotingOutcome::Votes(v) => v,
                VotingOutcome::EndGame => {
                    self.finalize_game().await;
                    return Ok(());
                }
            };

            // Step 5: Scoring and results reveal
            let (points_awarded, reasons) = calculate_round_points(
                mafia_id,
                player_teams[&mafia_id],
                winning_team,
                &self.player_ids,
                &player_teams,
                &votes,
            );

            for (&pid, &pts) in &points_awarded {
                *self.total_scores.entry(pid).or_insert(0) += pts;
            }

            self.update_leaderboard(false).await;

            let result = RoundResult {
                round_number: self.round,
                mafia_id,
                winning_team,
                votes,
                points_awarded,
                reasons,
            };

            let results_embed = build_results_embed(&result, &self.total_scores);
            let results_components = build_results_components();
            self.edit_gameplay_msg(results_embed.clone(), results_components.clone())
                .await;

            // Step 6: Wait for Next Round or End Game
            match self
                .wait_for_intermission(&results_embed, &results_components)
                .await
            {
                IntermissionOutcome::NextRound => {
                    self.round += 1;
                }
                IntermissionOutcome::EndGame | IntermissionOutcome::TimedOut => {
                    self.finalize_game().await;
                    return Ok(());
                }
            }
        }
    }

    async fn send_role_dms(&self, players: &[PlayerRoundInfo]) {
        for p in players {
            let teammates: Vec<String> = players
                .iter()
                .filter(|other| other.team == p.team && other.id != p.id)
                .map(|other| format!("<@{}>", other.id))
                .collect();

            let opponents: Vec<String> = players
                .iter()
                .filter(|other| other.team != p.team)
                .map(|other| format!("<@{}>", other.id))
                .collect();

            let teammates_str = if teammates.is_empty() {
                "None (1v1)".to_string()
            } else {
                teammates.join(", ")
            };
            let opponents_str = opponents.join(", ");

            let dm_embed = if p.role == Role::Mafia {
                CreateEmbed::new()
                    .title(format!("🎭 Rocket League Mafia: Round {}", self.round))
                    .description(format!(
                        "🕵️ You are the **MAFIA**!\n\n\
                         Your Team: {}\n\
                         Teammates: **{teammates_str}**\n\
                         Opponents: **{opponents_str}**\n\n\
                         **Your Secret Goal:**\n\
                         Secretly make your team **LOSE** without getting caught!\n\n\
                         **Points for Mafia:**\n\
                         • Lose the match with **NO votes**: **+3 points**\n\
                         *(Every other scenario awards 0 points for both roles).*\n\n\
                         *(You will also be asked to cast a dummy vote during the voting phase so nobody suspects you).* ",
                        p.team
                    ))
                    .color(config::COLOR_MAFIA)
            } else {
                CreateEmbed::new()
                    .title(format!("🎭 Rocket League Mafia: Round {}", self.round))
                    .description(format!(
                        "👥 You are a **VILLAGER**!\n\n\
                         Your Team: {}\n\
                         Teammates: **{teammates_str}**\n\
                         Opponents: **{opponents_str}**\n\n\
                         **Your Goal:**\n\
                         Play normally and win the Rocket League match!\n\
                         Keep a sharp eye on everyone; one of you is the Mafia secretly throwing the game.\n\n\
                         **Points for Villagers:**\n\
                         • Win match with Mafia on your team: **+1 point**\n\
                         • Correctly vote for the Mafia: **+1 point**\n\
                         *(Every other scenario awards 0 points for both roles).*",
                        p.team
                    ))
                    .color(config::COLOR_BLUE)
            };

            let ctx_serenity = self.ctx.serenity_context();
            let _ =
                p.id.direct_message(ctx_serenity, CreateMessage::new().embed(dm_embed))
                    .await;
        }
    }

    async fn wait_for_match_outcome(
        &self,
        restore_embed: &CreateEmbed,
        restore_components: &[CreateActionRow],
    ) -> MatchOutcome {
        loop {
            let mci =
                match serenity::ComponentInteractionCollector::new(self.ctx.serenity_context())
                    .message_id(self.gameplay_msg_id)
                    .timeout(config::MATCH_TIMEOUT)
                    .await
                {
                    Some(interaction) => interaction,
                    None => return MatchOutcome::TimedOut,
                };

            if !self.is_authorized(mci.user.id) {
                let _ = mci
                    .create_response(
                        &self.ctx.serenity_context().http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("❌ You are not a player in this game!")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                continue;
            }

            match mci.data.custom_id.as_str() {
                "report_blue" => {
                    let _ = mci
                        .create_response(
                            &self.ctx.serenity_context().http,
                            CreateInteractionResponse::Acknowledge,
                        )
                        .await;
                    return MatchOutcome::Winner(Team::Blue);
                }
                "report_orange" => {
                    let _ = mci
                        .create_response(
                            &self.ctx.serenity_context().http,
                            CreateInteractionResponse::Acknowledge,
                        )
                        .await;
                    return MatchOutcome::Winner(Team::Orange);
                }
                "request_end"
                    if self
                        .ask_end_confirmation(&mci, restore_embed, restore_components)
                        .await =>
                {
                    return MatchOutcome::EndGame;
                }
                _ => {}
            }
        }
    }

    async fn run_voting_phase(
        &self,
        round: u32,
        winning_team: Team,
        mafia_id: UserId,
        player_teams: &HashMap<UserId, Team>,
    ) -> VotingOutcome {
        use serenity::futures::StreamExt;

        let mut votes: HashMap<UserId, Option<UserId>> = HashMap::new();
        let voting_components = build_voting_components();
        let voting_embed = build_voting_embed(round, winning_team, &self.player_ids, &votes);

        self.edit_gameplay_msg(voting_embed, voting_components.clone())
            .await;

        let (vote_tx, mut vote_rx) =
            tokio::sync::mpsc::channel::<(UserId, Option<UserId>)>(self.player_ids.len());

        // Dispatch voting DMs concurrently
        for &pid in &self.player_ids {
            let ctx_serenity = self.ctx.serenity_context().clone();
            let tx = vote_tx.clone();
            let candidates = self.player_ids.clone();
            let names_map = self.player_names.clone();
            let is_mafia = pid == mafia_id;
            let team_name = player_teams.get(&pid).unwrap().name();
            let team_emoji = player_teams.get(&pid).unwrap().emoji();
            let winner_name = winning_team.name();
            let winner_emoji = winning_team.emoji();

            tokio::spawn(async move {
                let dm_embed = if is_mafia {
                    CreateEmbed::new()
                        .title(format!("🗳️ Round {round}: Cast Your Dummy Vote"))
                        .description(format!(
                            "🕵️ You are the **MAFIA**!\n\
                             Your Team: **{team_name}** {team_emoji}\n\
                             Winning Team: **{winner_name}** {winner_emoji}\n\n\
                             Submit your **dummy vote** by selecting any player below.\n\
                             *(This vote won't count against anyone, but it marks you as voted in the public channel so nobody suspects you!)*"
                        ))
                        .color(config::COLOR_MAFIA)
                } else {
                    CreateEmbed::new()
                        .title(format!("🗳️ Round {round}: Vote for the Mafia"))
                        .description(format!(
                            "👥 You are a **VILLAGER**!\n\
                             Your Team: **{team_name}** {team_emoji}\n\
                             Winning Team: **{winner_name}** {winner_emoji}\n\n\
                             Who do you believe is secretly the Mafia?\n\
                             Click a player below to cast your vote:"
                        ))
                        .color(config::COLOR_BLUE)
                };

                let dm_components = build_dm_vote_components(pid, &candidates, &names_map);

                let dm_msg = match pid
                    .direct_message(
                        &ctx_serenity,
                        CreateMessage::new()
                            .embed(dm_embed)
                            .components(dm_components),
                    )
                    .await
                {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Failed to send voting DM to {pid}: {e:?}");
                        let _ = tx.send((pid, None)).await;
                        return;
                    }
                };

                // Wait for DM button interaction with 1-minute timeout
                if let Some(mci) = dm_msg
                    .await_component_interaction(&ctx_serenity)
                    .timeout(config::VOTING_TIMEOUT)
                    .await
                {
                    let custom_id = &mci.data.custom_id;
                    let target_id = custom_id
                        .strip_prefix("vote:")
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(UserId::new);

                    let response_text = if is_mafia {
                        "✅ **Dummy vote recorded!** You are marked as voted in the public channel."
                            .to_string()
                    } else if let Some(tid) = target_id {
                        let target_name = names_map
                            .get(&tid)
                            .cloned()
                            .unwrap_or_else(|| format!("<@{tid}>"));
                        format!("✅ **Vote recorded!** You voted for **{target_name}**.")
                    } else {
                        "✅ **Vote recorded!**".to_string()
                    };

                    let _ = mci
                        .create_response(
                            &ctx_serenity.http,
                            CreateInteractionResponse::UpdateMessage(
                                CreateInteractionResponseMessage::new()
                                    .embed(
                                        CreateEmbed::new()
                                            .title("Vote Confirmed")
                                            .description(response_text)
                                            .color(config::COLOR_SUCCESS),
                                    )
                                    .components(vec![]),
                            ),
                        )
                        .await;

                    let _ = tx.send((pid, target_id)).await;
                } else {
                    let _ = tx.send((pid, None)).await;
                }
            });
        }

        drop(vote_tx);

        let mut public_stream =
            serenity::ComponentInteractionCollector::new(self.ctx.serenity_context())
                .message_id(self.gameplay_msg_id)
                .timeout(config::VOTING_TIMEOUT)
                .stream();

        loop {
            if votes.len() >= self.player_ids.len() {
                break;
            }

            tokio::select! {
                Some((voter, target)) = vote_rx.recv() => {
                    if let Entry::Vacant(e) = votes.entry(voter) {
                        e.insert(target);
                        let updated_embed = build_voting_embed(
                            round,
                            winning_team,
                            &self.player_ids,
                            &votes,
                        );
                        self.edit_gameplay_msg(updated_embed, voting_components.clone()).await;
                    }
                }
                Some(pub_mci) = public_stream.next() => {
                    if !self.is_authorized(pub_mci.user.id) {
                        let _ = pub_mci
                            .create_response(
                                &self.ctx.serenity_context().http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .content("❌ Only players in this game can interact with the controls.")
                                        .ephemeral(true),
                                ),
                            )
                            .await;
                        continue;
                    }

                    if pub_mci.data.custom_id == "request_end" {
                        let current_voting_embed = build_voting_embed(
                            round,
                            winning_team,
                            &self.player_ids,
                            &votes,
                        );
                        if self
                            .ask_end_confirmation(
                                &pub_mci,
                                &current_voting_embed,
                                &voting_components,
                            )
                            .await
                        {
                            return VotingOutcome::EndGame;
                        }
                    }
                }
                _ = tokio::time::sleep(config::VOTING_TIMEOUT) => {
                    for &pid in &self.player_ids {
                        votes.entry(pid).or_insert(None);
                    }
                    break;
                }
            }
        }

        VotingOutcome::Votes(votes)
    }

    async fn wait_for_intermission(
        &self,
        restore_embed: &CreateEmbed,
        restore_components: &[CreateActionRow],
    ) -> IntermissionOutcome {
        loop {
            let mci =
                match serenity::ComponentInteractionCollector::new(self.ctx.serenity_context())
                    .message_id(self.gameplay_msg_id)
                    .timeout(config::INTERMISSION_TIMEOUT)
                    .await
                {
                    Some(interaction) => interaction,
                    None => return IntermissionOutcome::TimedOut,
                };

            if !self.is_authorized(mci.user.id) {
                let _ = mci
                    .create_response(
                        &self.ctx.serenity_context().http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(
                                    "❌ Only players in this game can interact with the controls.",
                                )
                                .ephemeral(true),
                        ),
                    )
                    .await;
                continue;
            }

            match mci.data.custom_id.as_str() {
                "next_round" => {
                    let _ = mci
                        .create_response(
                            &self.ctx.serenity_context().http,
                            CreateInteractionResponse::Acknowledge,
                        )
                        .await;
                    return IntermissionOutcome::NextRound;
                }
                "request_end"
                    if self
                        .ask_end_confirmation(&mci, restore_embed, restore_components)
                        .await =>
                {
                    return IntermissionOutcome::EndGame;
                }
                _ => {}
            }
        }
    }

    async fn ask_end_confirmation(
        &self,
        initial_mci: &serenity::ComponentInteraction,
        restore_embed: &CreateEmbed,
        restore_components: &[CreateActionRow],
    ) -> bool {
        let _ = initial_mci
            .create_response(
                &self.ctx.serenity_context().http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(build_confirm_end_embed())
                        .components(build_confirm_end_components()),
                ),
            )
            .await;

        let conf = serenity::ComponentInteractionCollector::new(self.ctx.serenity_context())
            .message_id(self.gameplay_msg_id)
            .timeout(config::CONFIRM_TIMEOUT)
            .await;

        if let Some(conf_mci) = conf {
            if !self.is_authorized(conf_mci.user.id) {
                let _ = conf_mci
                    .create_response(
                        &self.ctx.serenity_context().http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("❌ Only players can confirm ending the game.")
                                .ephemeral(true),
                        ),
                    )
                    .await;

                self.edit_gameplay_msg(restore_embed.clone(), restore_components.to_vec())
                    .await;
                return false;
            }

            if conf_mci.data.custom_id == "confirm_end" {
                let _ = conf_mci
                    .create_response(
                        &self.ctx.serenity_context().http,
                        CreateInteractionResponse::Acknowledge,
                    )
                    .await;
                true
            } else {
                let _ = conf_mci
                    .create_response(
                        &self.ctx.serenity_context().http,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .embed(restore_embed.clone())
                                .components(restore_components.to_vec()),
                        ),
                    )
                    .await;
                false
            }
        } else {
            self.edit_gameplay_msg(restore_embed.clone(), restore_components.to_vec())
                .await;
            false
        }
    }

    async fn update_leaderboard(&self, is_final: bool) {
        let _ = self
            .channel_id
            .edit_message(
                &self.ctx.serenity_context().http,
                self.leaderboard_msg_id,
                EditMessage::new().embed(build_leaderboard_embed(
                    &self.player_ids,
                    &self.total_scores,
                    is_final,
                )),
            )
            .await;
    }

    async fn edit_gameplay_msg(&self, embed: CreateEmbed, components: Vec<CreateActionRow>) {
        let _ = self
            .channel_id
            .edit_message(
                &self.ctx.serenity_context().http,
                self.gameplay_msg_id,
                EditMessage::new().embed(embed).components(components),
            )
            .await;
    }

    async fn finalize_game(&self) {
        self.update_leaderboard(true).await;
        let final_embed = build_final_game_embed(&self.player_ids, &self.total_scores);
        self.edit_gameplay_msg(final_embed, vec![]).await;
    }
}

// ---------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_villager_wins_with_mafia_and_votes_correctly() {
        let mafia = UserId::new(1);
        let v1 = UserId::new(2);
        let v2 = UserId::new(3);
        let players = vec![mafia, v1, v2];

        let mut teams = HashMap::new();
        teams.insert(mafia, Team::Blue);
        teams.insert(v1, Team::Blue); // Teammate of Mafia
        teams.insert(v2, Team::Orange);

        let mut votes = HashMap::new();
        votes.insert(v1, Some(mafia)); // Correct vote
        votes.insert(v2, Some(v1)); // Incorrect vote
        votes.insert(mafia, Some(v2)); // Dummy vote

        // Blue wins
        let (points, reasons) =
            calculate_round_points(mafia, Team::Blue, Team::Blue, &players, &teams, &votes);

        // v1 won with mafia (+1) and correctly voted mafia (+1) = 2 pts
        assert_eq!(points[&v1], 2);
        assert_eq!(reasons[&v1].len(), 2);

        // v2 lost, voted incorrectly = 0 pts
        assert_eq!(points[&v2], 0);

        // Mafia won the game (did not lose) = 0 pts
        assert_eq!(points[&mafia], 0);
    }

    #[test]
    fn test_mafia_loses_with_zero_votes_earns_3_pts() {
        let mafia = UserId::new(1);
        let v1 = UserId::new(2);
        let v2 = UserId::new(3);
        let players = vec![mafia, v1, v2];

        let mut teams = HashMap::new();
        teams.insert(mafia, Team::Blue);
        teams.insert(v1, Team::Blue);
        teams.insert(v2, Team::Orange);

        let mut votes = HashMap::new();
        votes.insert(v1, Some(v2)); // Guessed v2 (wrong)
        votes.insert(v2, Some(v1)); // Guessed v1 (wrong)
        votes.insert(mafia, Some(v1)); // Mafia dummy vote for v1 (ignored)

        // Orange wins (Blue lost!)
        let (points, reasons) =
            calculate_round_points(mafia, Team::Blue, Team::Orange, &players, &teams, &votes);

        // Mafia lost with NO votes => +3
        assert_eq!(points[&mafia], 3);
        assert!(reasons[&mafia][0].contains("+3"));

        // v1 lost, wrong vote => 0
        assert_eq!(points[&v1], 0);

        // v2 won, but Mafia was not on Orange => 0
        assert_eq!(points[&v2], 0);
    }

    #[test]
    fn test_mafia_loses_but_received_votes_earns_0_pts() {
        let mafia = UserId::new(1);
        let v1 = UserId::new(2);
        let v2 = UserId::new(3);
        let players = vec![mafia, v1, v2];

        let mut teams = HashMap::new();
        teams.insert(mafia, Team::Blue);
        teams.insert(v1, Team::Blue);
        teams.insert(v2, Team::Orange);

        let mut votes = HashMap::new();
        votes.insert(v1, Some(mafia)); // Correctly voted mafia
        votes.insert(v2, Some(v1)); // Incorrect
        votes.insert(mafia, Some(v2)); // Dummy

        // Orange wins (Blue lost)
        let (points, _reasons) =
            calculate_round_points(mafia, Team::Blue, Team::Orange, &players, &teams, &votes);

        // Mafia lost, but received 1 vote => 0 pts!
        assert_eq!(points[&mafia], 0);

        // v1 lost with mafia, but correctly voted mafia => +1
        assert_eq!(points[&v1], 1);

        // v2 won without mafia on team, wrong vote => 0
        assert_eq!(points[&v2], 0);
    }

    #[test]
    fn test_1v1_testing_mode() {
        let p1 = UserId::new(1); // Mafia
        let p2 = UserId::new(2); // Villager
        let players = vec![p1, p2];

        let mut teams = HashMap::new();
        teams.insert(p1, Team::Blue);
        teams.insert(p2, Team::Orange);

        // Case A: Villager (p2) wins match and correctly votes p1 (Mafia)
        let mut votes_a = HashMap::new();
        votes_a.insert(p2, Some(p1));
        votes_a.insert(p1, Some(p2));

        let (points_a, _) =
            calculate_round_points(p1, Team::Blue, Team::Orange, &players, &teams, &votes_a);
        assert_eq!(points_a[&p2], 1); // Correct vote = 1
        assert_eq!(points_a[&p1], 0); // Mafia lost but got 1 vote = 0

        // Case B: Villager (p2) wins match, but abstains or fails to vote for Mafia
        let mut votes_b = HashMap::new();
        votes_b.insert(p2, None); // didn't vote
        votes_b.insert(p1, Some(p2));

        let (points_b, _) =
            calculate_round_points(p1, Team::Blue, Team::Orange, &players, &teams, &votes_b);
        assert_eq!(points_b[&p2], 0);
        assert_eq!(points_b[&p1], 3); // Mafia lost with NO votes = +3!
    }
}
