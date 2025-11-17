# SlackMUD

A Multi-User Dungeon (MUD) game server that integrates with Slack, written in Rust.

## Features

- **Slack Integration**: Play the game directly from Slack using slash commands or DMs
- **Persistent State**: Player data, classes, races, and rooms stored in PostgreSQL database
- **Public/Private Actions**: Some actions are visible to everyone, others are private
- **Character Customization**: Choose your class, race, and gender
- **Room System**: Each Slack channel is a room in the game with persistent player locations
- **DM Interface**: Send commands directly to the bot via DMs for a conversational experience
- **Wizard System**: Level 50+ players can create exits between rooms using the dig command
- **Connected Rooms**: Wizards can create directional exits (north, south, east, west, up, down) linking rooms together

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
   - Usage Hint: `look | character | dig | help`
4. Save

### 3. Configure Events API (for DM support)

1. In your Slack app settings, go to "Event Subscriptions"
2. Enable Events
3. Set Request URL: `https://your-render-url.onrender.com/slack/events`
   - Slack will verify this URL when you save (it must be publicly accessible)
4. Under "Subscribe to bot events", add:
   - `message.im` - Listen for DM messages to the bot
5. Save Changes
6. Go to "App Home"
7. Under "Show Tabs", enable "Messages Tab"
8. Check "Allow users to send Slash commands and messages from the messages tab"

### 4. Local Development

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
   WIZARDS=U01ABC123DE,U02XYZ789FG  # Optional: Comma-separated Slack user IDs
   ```

5. (Optional) Configure wizards for local development:
   ```bash
   cp wizards.txt.example wizards.txt
   ```

   Edit `wizards.txt` and add Slack user IDs (one per line). These users will be promoted to level 50 (wizard status) and can use the `/mud dig` command to create exits between rooms.

6. Run the server (migrations will run automatically):
   ```bash
   cargo run
   ```

The server will start on `http://localhost:3000`.

For local testing with Slack, you'll need to expose your local server using a tool like [ngrok](https://ngrok.com/):

```bash
ngrok http 3000
```

Then update your Slack app's slash command URL and Events API URL to the ngrok URLs.

### 5. Deploy to Render

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
   - `WIZARDS` (optional): Comma-separated list of Slack user IDs to promote to wizard (level 50)
     - Example: `U01ABC123DE,U02XYZ789FG`
     - Wizards can use the `/mud dig` command to create exits between rooms

5. Deploy! Render will build and deploy your app

6. Update your Slack app URLs to your Render URL:
   - Slash command: `https://your-app-name.onrender.com/slack/commands`
   - Events API: `https://your-app-name.onrender.com/slack/events`

## Game Commands

### General Commands

- `/mud look` or `/mud l` - Look around the current room
  - Sends room description privately to you, including exits and other players
  - Posts public message that you looked around
- `/mud character` or `/mud char` - View your character info
  - Shows your level, XP, class, race, gender
  - Lists available classes and races
- `/mud help` - Show help message

### Wizard Commands (Level 50+)

- `/mud dig <direction> #channel` - Create an exit from your current room to another channel
  - Valid directions: `north`, `south`, `east`, `west`, `up`, `down`
  - Example: `/mud dig north #tavern`
  - Creates a one-way exit in the specified direction
  - Posts atmospheric public message when exit is created

### DM Commands

You can also send commands directly to the bot via DM (without the `/mud` prefix):
- `look` or `l` - Look around your current room
- `character` or `c` - View your character
- `dig <direction> #channel` - (Wizards only) Create an exit
- `help` or `h` - Show help message

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
- Players have a persistent current room location (stored in database)
- When you first use `/mud look` in a channel, that becomes your starting room
- Slack channels act as "windows" - you can see public actions in any channel you're in, regardless of your game location
- Use `/mud look` to see your current room description, other players present, and available exits
- Wizards (level 50+) can create directional exits between rooms using `/mud dig`
- Movement between rooms will be implemented with the `/mud go <direction>` command (coming soon)

### DM Interface

SlackMUD supports a conversational DM interface for a more immersive experience:

- Send messages directly to the bot without the `/mud` prefix
- Type `look`, `character`, `dig north #tavern`, etc.
- All responses are sent privately to you via DM
- Public actions (like looking around) still post to the channel where your character is located
- Enable the "Messages Tab" in your Slack app's App Home settings to use DMs

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
