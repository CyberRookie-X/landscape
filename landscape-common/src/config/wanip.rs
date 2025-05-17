use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::iface::NetworkIfaceConfig;
use crate::config::iface::IfaceZoneType;
use crate::net_proto::udp::dhcp::DhcpOption;
use crate::store::storev2::LandscapeStore;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "common/wanip.d.ts")]
pub struct IfaceIpServiceConfig {
    pub iface_name: String,
    pub enable: bool,
    pub ip_model: IfaceIpModelConfig,
}

impl LandscapeStore for IfaceIpServiceConfig {
    fn get_store_key(&self) -> String {
        self.iface_name.clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, TS)]
#[ts(export, export_to = "common/wanip.d.ts")]
#[serde(tag = "t")]
#[serde(rename_all = "lowercase")]
pub enum IfaceIpModelConfig {
    #[default]
    Nothing,
    Static {
        #[serde(default)]
        default_router_ip: Option<Ipv4Addr>,
        #[serde(default)]
        default_router: bool,
        #[serde(default)]
        ipv4: Option<Ipv4Addr>,
        #[serde(default)]
        ipv4_mask: u8,
        #[serde(default)]
        ipv6: Option<Ipv6Addr>,
    },
    PPPoE {
        #[serde(default)]
        default_router: bool,
        username: String,
        password: String,
        mtu: u32,
    },
    DhcpClient {
        #[serde(default)]
        default_router: bool,
        hostname: Option<String>,
        /// Custome Options
        #[serde(default)]
        #[ts(type = "Array<any>")]
        custome_opts: Vec<DhcpOption>,
    },
}

impl IfaceIpModelConfig {
    /// 检查当前的 zone 设置是否满足 IP 配置的要求
    pub fn check_iface_status(&self, iface_config: &NetworkIfaceConfig) -> bool {
        match self {
            IfaceIpModelConfig::PPPoE { .. } => {
                matches!(iface_config.zone_type, IfaceZoneType::Wan)
            }
            IfaceIpModelConfig::DhcpClient { .. } => {
                matches!(iface_config.zone_type, IfaceZoneType::Wan)
            }
            _ => true,
        }
    }
}
