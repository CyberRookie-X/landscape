use landscape_common::{config::wifi::WifiServiceConfig, database::repository::UpdateActiveModel};
use sea_orm::{entity::prelude::*, ActiveValue::Set};
use serde::{Deserialize, Serialize};

use crate::DBTimestamp;

pub type WifiServiceConfigModel = Model;
pub type WifiServiceConfigEntity = Entity;
pub type WifiServiceConfigActiveModel = ActiveModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wifi_service_configs")]
#[cfg_attr(feature = "postgres", sea_orm(schema_name = "public"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub iface_name: String,
    pub enable: bool,

    /// hostapd config file
    #[sea_orm(column_type = "Text")]
    pub config: String,

    pub update_at: DBTimestamp,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WifiServiceConfig {
    fn from(entity: Model) -> Self {
        WifiServiceConfig {
            iface_name: entity.iface_name,
            enable: entity.enable,
            config: entity.config,
            update_at: entity.update_at,
        }
    }
}

impl Into<ActiveModel> for WifiServiceConfig {
    fn into(self) -> ActiveModel {
        let mut active = ActiveModel {
            iface_name: Set(self.iface_name.clone()),
            ..Default::default()
        };
        self.update(&mut active);
        active
    }
}

impl UpdateActiveModel<ActiveModel> for WifiServiceConfig {
    fn update(self, active: &mut ActiveModel) {
        active.enable = Set(self.enable);
        active.config = Set(self.config);
        active.update_at = Set(self.update_at);
    }
}
