mod db;
mod responses;

use std::{io::ErrorKind, time::Duration};

use ::time::OffsetDateTime;
use anyhow::{anyhow, bail, Result};
use config::{Case, Environment};
use humantime::parse_duration;
use lazy_static::lazy_static;
use log::{debug, error, info, trace};
use migration::{Migrator, MigratorTrait};
use nostr_sdk::prelude::*;
use regex::Regex;
use sea_orm::entity::*;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
use serde::Deserialize;
use tokio::{fs, select, signal};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

const PRIVATE_KEY_FILE: &str = ".privatekey";

lazy_static! {
    static ref REGEX: Regex = Regex::new(r"in (\d+\s?[A-Za-z]+)").unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = dotenvy::dotenv() {
        if !e.not_found() {
            bail!(e)
        }
    }
    env_logger::init();

    let cfg = get_config().await?;

    let db = get_db_and_migrate(&cfg.database_url).await?;

    let keys = get_keys().await?;
    let public_key = keys.public_key();
    info!("bot public key: {}", public_key.to_bech32()?);

    let client = get_client(&keys).await?;
    info!("client connected to relays");

    create_nostr_metadata(client.clone(), cfg.bot).await?;
    debug!("metadata for bot broadcasted");

    let tracker = TaskTracker::new();
    let cancel_token = CancellationToken::new();

    // kick off notification task
    tracker.spawn(process_reminder_notifications(
        cancel_token.clone(),
        client.clone(),
        keys.public_key(),
        db.clone(),
    ));
    // kick off reminder task
    tracker.spawn(process_reminders(cancel_token.clone(), client, db));
    tracker.close();

    if let Err(err) = signal::ctrl_c().await {
        error!("Unable to listen for shutdown signal: {}", err)
    }
    info!("shutting down");

    cancel_token.cancel();
    tracker.wait().await;
    info!("successfully shut down");

    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    pub database_url: String,
    pub bot: BotConfig,
}

#[derive(Clone, Debug, Deserialize)]
struct BotConfig {
    pub name: String,
    pub about: String,
    pub address: Option<String>,
    pub website: Option<String>,
    pub profile_pic: Option<String>,
}

async fn get_config() -> Result<Config> {
    let cfg = config::Config::builder()
        .add_source(
            Environment::default()
                .prefix("remind")
                .prefix_separator("_")
                .convert_case(Case::UpperSnake)
                .separator("__"),
        )
        .set_default("bot.name", "RemindMe")?
        .set_default(
            "bot.about",
            "Simple bot for reminding about events on nostr",
        )?
        .build()?;

    Ok(cfg.try_deserialize()?)
}

async fn get_db_and_migrate(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;

    Migrator::up(&db, None).await?;

    Ok(db)
}

async fn get_keys() -> Result<Keys> {
    match fs::read_to_string(PRIVATE_KEY_FILE).await {
        Ok(key) => Ok(Keys::parse(key)?),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let keys = Keys::generate();
            let private_key = keys.secret_key()?.display_secret().to_string();
            fs::write(PRIVATE_KEY_FILE, private_key).await?;
            Ok(keys)
        }
        Err(e) => Err(anyhow!(e)),
    }
}

async fn get_client(keys: &Keys) -> Result<Client> {
    let client = Client::new(keys);
    // add reader relays
    for relay in [
        "wss://relay.damus.io",
        "wss://nostr.plebchain.org/",
        "wss://bitcoiner.social/",
        "wss://relay.snort.social",
        "wss://relayable.org",
        "wss://nos.lol",
        "wss://nostr.mom",
        "wss://e.nos.lol",
        "wss://nostr.bitcoiner.social",
    ] {
        client
            .add_relay_with_opts(relay, RelayOptions::default().write(false))
            .await?;
    }
    // add blastr as writer for ultimate annoyance
    client
        .add_relay_with_opts(
            "wss://nostr.mutinywallet.com",
            RelayOptions::default().read(false),
        )
        .await?;

    client.connect().await;

    Ok(client)
}

async fn create_nostr_metadata(client: Client, bot_config: BotConfig) -> Result<()> {
    let mut metadata = Metadata::new()
        .name(&bot_config.name)
        .display_name(bot_config.name)
        .about(bot_config.about);

    if let Some(address) = bot_config.address {
        metadata = metadata.nip05(address);
    }

    if let Some(website) = bot_config.website {
        metadata = metadata.website(Url::parse(&website)?);
    }

    if let Some(pfp) = bot_config.profile_pic {
        metadata = metadata.picture(Url::parse(&pfp)?);
    }

    let builder = EventBuilder::metadata(&metadata);

    client.send_event_builder(builder).await?;

    Ok(())
}

async fn process_reminder_notifications(
    cancel_token: CancellationToken,
    client: Client,
    pubkey: PublicKey,
    db: DatabaseConnection,
) {
    let mut notifications = client.notifications();
    let start = Timestamp::now() - Duration::from_secs(60 * 60 * 24 * 2);
    let filter = Filter::new()
        .kind(Kind::TextNote)
        .since(start)
        .pubkey(pubkey);
    client.subscribe(vec![filter], None).await;
    info!("listening for notifications");

    loop {
        let notification = select! {
            _ = cancel_token.cancelled() => {
                info!("notification task is exiting");
                return;
            }
            Ok(notification) = notifications.recv() => notification
        };

        trace!("raw notification received: {:?}", &notification);

        if let RelayPoolNotification::Event { event: reply, .. } = notification {
            debug!("event received: {}", reply.content);
            let Some(Tag::Event { event_id, .. }) = reply.tags().iter().find(|e| e.is_reply())
            else {
                debug!("event {} not a reply", reply.id);
                continue;
            };

            let Some(caps) = REGEX.captures(reply.content()) else {
                debug!("event {} does not match the expected message", reply.id);
                continue;
            };

            let Some(timeframe) = caps.get(1) else {
                debug!("event {} does not have a timeframe", reply.id);
                continue;
            };

            let timeframe = timeframe.as_str().replace(' ', "");
            let Ok(remind_in) = parse_duration(&timeframe) else {
                debug!("event {} does not have a valid timeframe", reply.id);
                continue;
            };

            info!(
                "received reminder request note. reply id: {}, event id: {}, timeframe: {}, remind_in: {}",
                reply.id,
                event_id,
                timeframe,
                remind_in.as_secs()
            );
            match db::user_hit_rate_limit(&db, reply.pubkey).await {
                Ok(true) => {
                    info!("user has hit rate limit. skipping.");
                    _ = responses::rate_limit_hit(client.clone(), &reply).await;
                    continue;
                }
                Err(e) => {
                    error!("error checking rate limit: {}", e);
                    continue;
                }
                _ => {}
            }

            match db::user_has_reminder(&db, *event_id, reply.pubkey).await {
                Ok(true) => {
                    info!("user already has a reminder for this note");
                    continue;
                }
                Err(e) => {
                    error!("error checking for open reminder: {}", e);
                    continue;
                }
                _ => {}
            }

            debug!("inserting reminder into db");
            let remind_at = OffsetDateTime::now_utc() + remind_in;
            if let Err(e) =
                db::insert_reminder(&db, event_id, &reply.pubkey, reply.created_at, remind_at).await
            {
                error!("error inserting the reminder in the db: {}", e);
                continue;
            }

            if let Err(e) =
                responses::new_reminder_creation(client.clone(), &reply, remind_at).await
            {
                error!("error notifying the user of the insert: {}", e);
                continue;
            }
            debug!("inserted reminder into db");
        }
    }
}

async fn process_reminders(
    cancel_token: CancellationToken,
    client: Client,
    db: DatabaseConnection,
) {
    let mut duration = Duration::from_secs(0);
    loop {
        select! {
            _ = cancel_token.cancelled() => {
                info!("reminder task is exiting");
                break;
            }
            _ = tokio::time::sleep(duration) => {}
        }

        info!("checking for open reminders to notify on");
        let reminders = match db::get_open_reminders(&db).await {
            Ok(reminders) => reminders,
            Err(e) => {
                error!("error getting open reminders: {}", e);
                continue;
            }
        };
        debug!("{} reminders found to notify on", reminders.len());

        for reminder in reminders {
            let event_id = EventId::from_hex(&reminder.note_id).unwrap();
            let pubkey = PublicKey::from_hex(&reminder.user_pubkey).unwrap();

            debug!(
                "reminding {} about event {}",
                pubkey.to_bech32().unwrap(),
                event_id.to_bech32().unwrap()
            );
            if let Err(e) =
                responses::reminder_duration_reached(client.clone(), event_id, pubkey).await
            {
                error!("error reminding user: {}", e);
                continue;
            }
            debug!("reminder successfully sent");

            let id = reminder.id;
            let mut reminder = reminder.into_active_model();
            reminder.reminded_at = Set(Some(OffsetDateTime::now_utc()));
            if let Err(e) = reminder.update(&db).await {
                error!("error updating the reminder reminded_at time: {}", e);
                continue;
            }
            debug!("reminder {} successfully updated", id);
        }

        duration = Duration::from_secs(60);
    }
}
