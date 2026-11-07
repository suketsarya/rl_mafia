# Rocket League Mafia: Discord Bot ⚽🕵️

A Discord bot for playing **Rocket League Mafia**, built with Rust, [Poise](https://github.com/serenity-rs/poise), and [Serenity](https://github.com/serenity-rs/serenity).

---

## 🎮 How the Game Works

1. **Start the Game**:
   * Gather players into a voice channel (**6 players** for standard 3v3, or **2 players** for 1v1 testing mode).
   * Run the `/new` slash command (or `!!new`).
2. **Two Messages Only**:
   * **Message 1 (Leaderboard)**: Displays the live standings in clean columns ordered by total points with medals (`🥇`, `🥈`, `🥉`).
   * **Message 2 (Gameplay)**: Displays the active game state and interactive buttons.
   * All updates occur strictly by editing these two messages to keep the channel clean.
3. **Secret Roles & Random Teams**:
   * Players are randomly divided into 🟦 **Blue Team** and 🟧 **Orange Team**.
   * One random player is secretly chosen as the **Mafia**; everyone else is a **Villager**.
   * Roles, objectives, teammates, and opponents are sent to each player via **Direct Message**.
4. **Play the Match**:
   * Hop into Rocket League and play the match while chatting together in voice.
   * The Mafia's goal is to subtly sabotage their team so they lose without getting caught!
5. **Report Winner**:
   * Once finished, a player clicks `[🟦 Blue Won]` or `[🟧 Orange Won]` on the gameplay message.
6. **Private Voting (1 Minute Timeout)**:
   * Each player receives a private DM with buttons to vote on who they believe the Mafia was.
   * The Mafia receives the same buttons to submit a **dummy vote** so their identity is never given away.
   * The public gameplay message updates live with checkmarks (`✅`) as players cast their votes.
7. **Scoring & Leaderboard**:
   * Once voting finishes, the Mafia is revealed and round points are awarded:
     * **Villagers:**
       * Win the match with Mafia on your team: **+1 pt**
       * Correctly vote for the Mafia: **+1 pt**
     * **Mafia:**
       * Lose the match with **NO votes** against them: **+3 pts**
     * *Every other scenario awards 0 points for both roles.*
   * The Leaderboard updates automatically with new cumulative scores in aligned columns.
8. **Next Round or End Game**:
   * Click `[▶️ Next Round]` to immediately roll into another game with new teams and roles.
   * Click `[🏁 End Game]` (protected by an *"Are you sure?"* confirmation gate) to finalise the session and display the winner.

---

## ⌨️ Bot Commands

| Command | Description |
|---|---|
| `/new` | Start a new game with the users in your current voice channel. |
| `/cancel` | Force-cancel an ongoing game in the channel (clears active lock). |
| `/rules` | View a quick summary of the points system. |
| `/help` | Complete walkthrough and guide for Rocket League Mafia. |
| `/age` | Displays account creation date for a user. |

---

## ⚙️ Setup & Configuration

### Prerequisites
* [Rust](https://rustup.rs/) (edition 2021 or newer)
* Discord Bot Token (with Gateway Intents enabled)

### Discord Developer Portal Intents
Make sure the following **Privileged Gateway Intents** are enabled in the [Discord Developer Portal](https://discord.com/developers/applications):
* **Server Members Intent** (`GUILD_MEMBERS`)
* **Message Content Intent** (`MESSAGE_CONTENT`)

### Environment Variable / Token File
You can provide your Discord Bot Token in either of two ways:
1. Environment variable:
   ```bash
   export DISCORD_TOKEN="your_bot_token_here"
   ```
2. Or in `src/BotToken.txt`:
   ```text
   your_bot_token_here
   ```

### Running the Bot
```bash
# Check code & warnings
cargo check

# Run automated tests
cargo test

# Build and run
cargo run
```

