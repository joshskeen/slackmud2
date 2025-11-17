# SlackMUD

A Multi-User Dungeon (MUD) game server that integrates with Slack, written in Rust.

## Features

- **Slack Integration**: Play the game directly from Slack using slash commands
- **Persistent State**: Player data, classes, races, and rooms stored in SQLite database
- **Public/Private Actions**: Some actions are visible to everyone, others are private
- **Character Customization**: Choose your class, race, and gender
- **Room System**: Each Slack channel is a room in the game

## Architecture

- **Rust**: High-performance, safe systems programming language
- **Axum**: Modern async web framework for handling Slack webhooks
- **SQLx**: Async SQL toolkit with compile-time query verification
- **PostgreSQL**: Robust relational database for persistent game state
- **Reqwest**: HTTP client for Slack API calls

## Setup

### Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- PostgreSQL 12+ ([Install PostgreSQL](https://www.postgresql.org/download/))
- A Slack workspace where you can install apps
- (Optional) Docker for deployment

### 1. Create a Slack App

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Click "Create New App" → "From scratch"
3. Name it "SlackMUD" and select your workspace
4. Under "OAuth & Permissions", add these Bot Token Scopes:
   - `chat:write` - Send messages
   - `im:write` - Send DMs
   - `users:read` - Get user info
   - `channels:read` - Read channel info
5. Install the app to your workspace
6. Copy the "Bot User OAuth Token" (starts with `xoxb-`)
7. Under "Basic Information", copy the "Signing Secret"

### 2. Configure Slash Commands

1. In your Slack app settings, go to "Slash Commands"
2. Click "Create New Command"
3. Set:
   - Command: `/mud`
   - Request URL: `https://your-render-url.onrender.com/slack/commands`
   - Short Description: `Play SlackMUD`
   - Usage Hint: `look | character | help`
4. Save

### 3. Local Development

1. Clone this repository

2. Set up PostgreSQL database:
   ```bash
   # Create database
   createdb slackmud

   # Or using psql:
   psql -c "CREATE DATABASE slackmud;"
   ```

3. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

4. Edit `.env` and add your tokens:
   ```
   SLACK_BOT_TOKEN=xoxb-your-token-here
   SLACK_SIGNING_SECRET=your-signing-secret-here
   DATABASE_URL=postgresql://localhost/slackmud
   HOST=0.0.0.0
   PORT=3000
   ```

5. Run the server (migrations will run automatically):
   ```bash
   cargo run
   ```

The server will start on `http://localhost:3000`.

For local testing with Slack, you'll need to expose your local server using a tool like [ngrok](https://ngrok.com/):

```bash
ngrok http 3000
```

Then update your Slack app's slash command URL to the ngrok URL.

### 4. Deploy to Render

1. Push this code to a Git repository (GitHub, GitLab, etc.)

2. Create PostgreSQL database on Render:
   - Go to [render.com](https://render.com/) dashboard
   - Click "New +" → "PostgreSQL"
   - Name it `slackmud-database`
   - Choose your plan and region
   - Click "Create Database"
   - Copy the "Internal Database URL" for the next step

3. Create the web service:
   - Click "New +" → "Web Service"
   - Connect your repository
   - Render will detect the `render.yaml` and `Dockerfile`

4. Add environment variables in Render dashboard:
   - `SLACK_BOT_TOKEN`: Your bot token from Slack app settings
   - `SLACK_SIGNING_SECRET`: Your signing secret from Slack app settings
   - `DATABASE_URL`: The Internal Database URL from step 2

5. Deploy! Render will build and deploy your app

6. Update your Slack app's slash command URL to your Render URL:
   - Format: `https://your-app-name.onrender.com/slack/commands`

## Game Commands

- `/mud look` or `/mud l` - Look around the current room (channel)
  - Sends room description privately to you
  - Posts public message that you looked around
- `/mud character` or `/mud char` - View your character info
  - Shows your level, XP, class, race, gender
  - Lists available classes and races
- `/mud help` - Show help message

## How It Works

### Public vs Private Actions

The game implements two types of messaging:

1. **Private (DM)**: Information sent only to the user who triggered the command
   - Example: Room descriptions when you `/mud look`
   - Example: Character sheet when you `/mud character`

2. **Public (Channel)**: Messages posted in the channel for everyone to see
   - Example: "Alice looks around the room carefully." when Alice uses `/mud look`
   - Example: Combat actions, movement notifications (to be implemented)

### Room System

- Each Slack channel is a "room" in the game
- When you use `/mud look` in a channel, you see that room's description
- Room descriptions can be customized (future feature)
- Moving between channels = moving between rooms

### Player Progression

- Players have levels and experience points
- Players can choose a class (Warrior, Mage, Rogue, Cleric)
- Players can choose a race (Human, Elf, Dwarf, Halfling)
- Character attributes affect gameplay (to be implemented)

## Development

### Project Structure

```
src/
├── main.rs           # Application entry point, web server setup
├── models/           # Data models (Player, Class, Race, Room)
├── db/               # Database layer (repositories)
├── slack/            # Slack API client and types
└── handlers/         # Slash command handlers
    ├── look.rs       # /mud look command
    └── character.rs  # /mud character command

migrations/           # Database migrations
```

### Adding New Commands

1. Create a new handler in `src/handlers/`
2. Add the handler to `src/handlers/mod.rs`
3. Update the command router in `handle_slash_command()`

### Database Migrations

Migrations are in the `migrations/` directory and run automatically on startup.

To create a new migration:
1. Create a new file: `migrations/00N_description.sql`
2. Write your SQL
3. Restart the server

## Future Features

- [ ] Interactive character customization with Slack modals
- [ ] Combat system
- [ ] Inventory and items
- [ ] Quests and NPCs
- [ ] Party system
- [ ] Movement between channels
- [ ] Custom room descriptions set by channel admins
- [ ] Leaderboards
- [ ] Skills and abilities

## License

MIT
