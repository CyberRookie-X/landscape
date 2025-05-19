use landscape_common::config::nat::NatServiceConfig;
use landscape_common::database::repository::UpdateActiveModel;
use sea_orm::{entity::prelude::*, ActiveValue::Set};
use serde::{Deserialize, Serialize};

use crate::DBTimestamp;

pub type NatServiceConfigModel = Model;
pub type NatServiceConfigEntity = Entity;
pub type NatServiceConfigActiveModel = ActiveModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "nat_service_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub iface_name: String,
    pub enable: bool,

    pub tcp_range_start: u16,
    pub tcp_range_end: u16,

    pub udp_range_start: u16,
    pub udp_range_end: u16,

    pub icmp_in_range_start: u16,
    pub icmp_in_range_end: u16,

    pub update_at: DBTimestamp,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for NatServiceConfig {
    fn from(model: Model) -> Self {
        NatServiceConfig {
            iface_name: model.iface_name,
            enable: model.enable,
            nat_config: landscape_common::config::nat::NatConfig {
                tcp_range: model.tcp_range_start..model.tcp_range_end,
                udp_range: model.udp_range_start..model.udp_range_end,
                icmp_in_range: model.icmp_in_range_start..model.icmp_in_range_end,
            },
            update_at: model.update_at,
        }
    }
}

impl Into<ActiveModel> for NatServiceConfig {
    fn into(self) -> ActiveModel {
        let mut active = ActiveModel {
            iface_name: Set(self.iface_name.clone()),
            ..Default::default()
        };
        self.update(&mut active);
        active
    }
}

impl UpdateActiveModel<ActiveModel> for NatServiceConfig {
    fn update(self, active: &mut ActiveModel) {
        active.enable = Set(self.enable);

        active.tcp_range_start = Set(self.nat_config.tcp_range.start);
        active.tcp_range_end = Set(self.nat_config.tcp_range.end);

        active.udp_range_start = Set(self.nat_config.udp_range.start);
        active.udp_range_end = Set(self.nat_config.udp_range.end);

        active.icmp_in_range_start = Set(self.nat_config.icmp_in_range.start);
        active.icmp_in_range_end = Set(self.nat_config.icmp_in_range.end);

        active.update_at = Set(self.update_at);
    }
}
