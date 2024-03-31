use std::time::Duration;

use ::time::OffsetDateTime;
use anyhow::Result;
use entity::reminders::{
    self, ActiveModel as ActiveReminder, Entity as Reminders, Model as Reminder,
};
use nostr_sdk::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub async fn user_hit_rate_limit(db: &DatabaseConnection, pubkey: PublicKey) -> Result<bool> {
    let time_min = OffsetDateTime::now_utc() - Duration::from_secs(60 * 60);
    let count = Reminders::find()
        .filter(reminders::Column::UserPubkey.eq(pubkey.to_hex()))
        .filter(reminders::Column::CreatedAt.gte(time_min))
        .count(db)
        .await?;

    Ok(count >= 5)
}

pub async fn user_has_reminder(
    db: &DatabaseConnection,
    event_id: EventId,
    pubkey: PublicKey,
) -> Result<bool> {
    let count = Reminders::find()
        .filter(reminders::Column::NoteId.eq(event_id.to_hex()))
        .filter(reminders::Column::UserPubkey.eq(pubkey.to_hex()))
        .count(db)
        .await?;

    Ok(count != 0)
}

pub async fn insert_reminder(
    db: &DatabaseConnection,
    event_id: &EventId,
    pubkey: &PublicKey,
    created_at: Timestamp,
    remind_at: OffsetDateTime,
) -> Result<()> {
    let created_at = OffsetDateTime::from_unix_timestamp(created_at.as_i64())?;
    let reminder = ActiveReminder {
        note_id: Set(event_id.to_hex()),
        user_pubkey: Set(pubkey.to_hex()),
        created_at: Set(created_at),
        remind_at: Set(remind_at),
        ..Default::default()
    };
    reminder.insert(db).await?;

    Ok(())
}

pub async fn get_open_reminders(db: &DatabaseConnection) -> Result<Vec<Reminder>> {
    Ok(Reminders::find()
        .filter(reminders::Column::RemindedAt.is_null())
        .filter(reminders::Column::RemindAt.lte(OffsetDateTime::now_utc()))
        .all(db)
        .await?)
}
