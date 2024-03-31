//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "reminders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub note_id: String,
    pub user_pubkey: String,
    pub created_at: TimeDateTimeWithTimeZone,
    pub remind_at: TimeDateTimeWithTimeZone,
    pub reminded_at: Option<TimeDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
