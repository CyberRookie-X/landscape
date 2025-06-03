use crate::{
    database::repository::LandscapeDBStore, store::storev2::LandscapeStore,
    utils::time::get_f64_timestamp,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::net::MacAddr;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "common/dhcp_v6_client.d.ts")]
pub struct IPV6PDServiceConfig {
    pub iface_name: String,
    pub enable: bool,
    pub config: IPV6PDConfig,
    #[serde(default = "get_f64_timestamp")]
    pub update_at: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, export_to = "common/dhcp_v6_client.d.ts")]
pub struct IPV6PDConfig {
    pub mac: MacAddr,
}

impl LandscapeDBStore<String> for IPV6PDServiceConfig {
    fn get_id(&self) -> String {
        self.iface_name.clone()
    }
}

impl LandscapeStore for IPV6PDServiceConfig {
    fn get_store_key(&self) -> String {
        self.iface_name.clone()
    }
}
