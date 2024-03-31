use ::time::{format_description::well_known::Rfc2822, OffsetDateTime};
use anyhow::Result;
use nostr_sdk::prelude::*;

pub async fn new_reminder_creation(
    client: Client,
    event: &Event,
    remind_at: OffsetDateTime,
) -> Result<()> {
    let message = format!(
        "Will do. I will remind you of this note at {}.",
        remind_at.format(&Rfc2822)?
    );

    respond(client, event.id, event.pubkey, &message).await
}

pub async fn reminder_duration_reached(
    client: Client,
    event_id: EventId,
    pubkey: PublicKey,
) -> Result<()> {
    let message = format!(
        "Hey nostr:{}! You asked me to remind you about this. nost:{}",
        pubkey.to_bech32().unwrap(),
        event_id.to_bech32().unwrap(),
    );

    respond(client, event_id, pubkey, &message).await
}

pub async fn rate_limit_hit(client: Client, event: &Event) -> Result<()> {
    let message = "I'm sorry, but you've been rate-limited. Maybe wait a bit and try again later.";

    respond(client, event.id, event.pubkey, message).await
}

async fn respond(
    client: Client,
    event_id: EventId,
    pubkey: PublicKey,
    message: &str,
) -> Result<()> {
    let tags = vec![Tag::event(event_id), Tag::public_key(pubkey)];
    let builder = EventBuilder::new(Kind::TextNote, message, tags);

    client.send_event_builder(builder).await?;
    Ok(())
}
