use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;

use std::time::Instant;

struct Data {}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn open_db() -> rusqlite::Result<rusqlite::Connection> {
    rusqlite::Connection::open("roles.db3")
}

/// display the bot's latency to Discord
#[poise::command(slash_command)]
async fn ping(
    ctx: Context<'_>
) -> Result<(), Error> {
    let start = Instant::now();

    let msg = ctx.reply("Pong! \u{1F3D3}").await?;

    let duration = Instant::now() - start;

    let ping = ctx.ping().await;

    msg.edit(ctx, CreateReply {
        content: Some(
            format!("Pong! \u{1F3D3}\n WS: {}ms\n POST: {}ms",
                ping.as_millis(), duration.as_millis())
        ),
        ..Default::default()
    }).await?;

    Ok(())
}

#[poise::command(slash_command)]
async fn create_role(
    ctx: Context<'_>,
    #[description = "The name of the role"] name: String,
    #[description = "The color of your role"] color: String,
) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_ROLES")]
async fn force_associate_role(
    ctx: Context<'_>,
    #[description = "the user"] user: serenity::User,
    #[description = "the role"] role: serenity::Role,
) -> Result<(), Error> {
    let conn = open_db()?;

    conn.execute("INSERT INTO roles (id, role) VALUES (?1, ?2)",
        rusqlite::params![user.id.get(), role.id.get()])?;

    ctx.reply("ok!").await?;

    Ok(())
}

fn get_user_role(uid: u64) -> Result<u64, Error> {
    let conn = open_db()?;

    let mut stmt = conn.prepare("SELECT role FROM roles WHERE id=?1")?;
    let mut rows = stmt.query(rusqlite::params![uid])?;
    let mut roleids = vec![];

    while let Some(row) = rows.next()? {
        roleids.push(row.get::<_, u64>(0)?);
    }

    if roleids.len() == 0 {
        return Err(Error::from("no user role registered"));
    }

    Ok(roleids[0])
}

fn parse_color(input: &str) -> Option<u32> {
    use hex_color::HexColor;

    if let Ok(color) = HexColor::parse_rgb(input) {
        Some(color.to_u24())
    } else {
        None
    }
}

/// change the name or color of your role
#[poise::command(slash_command)]
async fn edit_role(
    ctx: Context<'_>,
    #[description = "The name of the role"] name: Option<String>,
    #[description = "The color of your role"] color: Option<String>,
) -> Result<(), Error> {
    let roleid = get_user_role(ctx.author().id.get())?;
    let guild = ctx.guild_id().unwrap();

    let mut roles = guild.roles(ctx.http()).await?;
    let role = roles.get_mut(&serenity::RoleId::new(roleid)).unwrap();

    let mut edit = serenity::EditRole::new();

    if let Some(name) = name {
        edit = edit.name(&name);
    }

    if let Some(color) = color {
        let color = parse_color(&color).unwrap();
        edit = edit.colour(color);
    }

    role.edit(ctx.http(), edit).await?;

    ctx.reply("successfully updated role!").await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping(), force_associate_role(), create_role(), edit_role()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();

    Ok(())
}
