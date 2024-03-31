use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Reminders::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Reminders::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Reminders::NoteId).string().not_null())
                    .col(ColumnDef::new(Reminders::UserPubkey).string().not_null())
                    .col(
                        ColumnDef::new(Reminders::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Reminders::RemindAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Reminders::RemindedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("reminder-event-user-idx")
                    .table(Reminders::Table)
                    .col(Reminders::NoteId)
                    .col(Reminders::UserPubkey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("reminder-event-user-idx")
                    .table(Reminders::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Reminders::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Reminders {
    Table,
    Id,
    NoteId,
    UserPubkey,
    CreatedAt,
    RemindAt,
    RemindedAt,
}
