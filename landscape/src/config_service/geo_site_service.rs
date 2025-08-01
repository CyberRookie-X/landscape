use landscape_common::{
    config::{
        dns::{DNSRuleConfig, DNSRuntimeRule, RuleSource},
        geo::{GeoDomainConfig, GeoFileCacheKey, GeoSiteFileConfig},
    },
    database::LandscapeDBTrait,
    service::controller_service::ConfigController,
    store::storev3::LandscapeStoreTrait,
    utils::time::{get_f64_timestamp, MILL_A_DAY},
};
use uuid::Uuid;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use landscape_common::{
    args::LAND_HOME_PATH, config::geo::GeoSiteSourceConfig, event::dns::DnsEvent,
    store::storev3::StoreFileManager, LANDSCAPE_GEO_CACHE_TMP_DIR,
};
use landscape_database::{
    geo_site::repository::GeoSiteConfigRepository, provider::LandscapeDBServiceProvider,
};
use reqwest::Client;
use tokio::sync::{mpsc, Mutex};

const A_DAY: u64 = 60 * 60 * 24;

pub type GeoDomainCacheStore = Arc<Mutex<StoreFileManager<GeoFileCacheKey, GeoDomainConfig>>>;

#[derive(Clone)]
pub struct GeoSiteService {
    store: GeoSiteConfigRepository,
    file_cache: GeoDomainCacheStore,
    dns_events_tx: mpsc::Sender<DnsEvent>,
}

impl GeoSiteService {
    pub async fn new(
        store: LandscapeDBServiceProvider,
        dns_events_tx: mpsc::Sender<DnsEvent>,
    ) -> Self {
        let store = store.geo_site_rule_store();

        let file_cache = Arc::new(Mutex::new(StoreFileManager::new(
            LAND_HOME_PATH.join(LANDSCAPE_GEO_CACHE_TMP_DIR),
            "site".to_string(),
        )));

        let service = Self { store, file_cache, dns_events_tx };
        let service_clone = service.clone();
        tokio::spawn(async move {
            //
            let mut ticker = tokio::time::interval(Duration::from_secs(A_DAY));
            loop {
                service_clone.refresh(false).await;
                // 等待下一次 tick
                ticker.tick().await;
            }
        });
        service
    }

    pub async fn convert_config_to_runtime_rule(
        &self,
        configs: Vec<DNSRuleConfig>,
    ) -> Vec<DNSRuntimeRule> {
        let time = Instant::now();
        let mut lock = self.file_cache.lock().await;
        let mut result = Vec::with_capacity(configs.len());
        for config in configs.into_iter() {
            let mut usage_keys = HashSet::new();
            let mut source = vec![];

            let mut inverse_keys: HashMap<String, HashSet<String>> = HashMap::new();
            for each in config.source.into_iter() {
                match each {
                    RuleSource::GeoKey(k) if k.inverse => {
                        inverse_keys.entry(k.name).or_default().insert(k.key);
                    }
                    RuleSource::GeoKey(k) => {
                        let file_cache_key = k.get_file_cache_key();
                        let predicate: Box<dyn Fn(&GeoSiteFileConfig) -> bool> =
                            if let Some(attr) = k.attribute_key {
                                let attr = attr.clone();
                                Box::new(move |config: &GeoSiteFileConfig| {
                                    config.attributes.contains(&attr)
                                })
                            } else {
                                Box::new(move |_: &GeoSiteFileConfig| true)
                            };
                        if let Some(domains) = lock.get(&file_cache_key) {
                            source.extend(
                                domains.values.into_iter().filter(predicate).map(Into::into),
                            );
                        }
                        usage_keys.insert(file_cache_key);
                    }
                    RuleSource::Config(c) => {
                        source.push(c);
                    }
                }
            }

            if inverse_keys.len() > 0 {
                let all_keys: Vec<_> = lock.keys();
                tracing::debug!("all_keys {:?}", all_keys.len());
                tracing::debug!("{:?}", inverse_keys);
                for (inverse_key, excluded_names) in inverse_keys {
                    for key in all_keys.iter().filter(|k| k.name == inverse_key) {
                        if !excluded_names.contains(&key.key) {
                            if let Some(domains) = lock.get(key) {
                                if !usage_keys.contains(key) {
                                    usage_keys.insert(key.clone());
                                    source.extend(domains.values.into_iter().map(Into::into));
                                }
                            }
                            // } else {
                            //     tracing::debug!("excluded_names: {:#?}", key);
                        }
                    }
                }
                tracing::debug!("using key len: {:#?}", usage_keys.len());
            }

            result.push(DNSRuntimeRule {
                source,
                id: config.id,
                name: config.name,
                index: config.index,
                enable: config.enable,
                filter: config.filter,
                resolve_mode: config.resolve_mode,
                mark: config.mark,
                flow_id: config.flow_id,
            });
        }
        tracing::debug!("covert config time: {:?}s", time.elapsed().as_secs());
        result
    }

    pub async fn refresh(&self, force: bool) {
        // 读取当前规则
        let mut configs: Vec<GeoSiteSourceConfig> = self.store.list().await.unwrap();

        if !force {
            let now = get_f64_timestamp();
            configs = configs.into_iter().filter(|e| e.next_update_at < now).collect();
        }

        let client = Client::new();
        let mut config_names = HashSet::new();
        for mut config in configs {
            let url = config.url.clone();
            config_names.insert(config.name.clone());

            tracing::debug!("download file: {}", url);
            let time = Instant::now();

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                    Ok(bytes) => {
                        let result = landscape_protobuf::read_geo_sites_from_bytes(bytes).await;
                        // tracing::debug!("get response file: {:?}", result);

                        let mut file_cache_lock = self.file_cache.lock().await;
                        let mut exist_keys = file_cache_lock
                            .keys()
                            .into_iter()
                            .filter(|k| k.name == config.name)
                            .collect::<HashSet<GeoFileCacheKey>>();

                        for (key, values) in result {
                            let info = GeoDomainConfig {
                                name: config.name.clone(),
                                key: key.to_ascii_uppercase(),
                                values,
                            };
                            exist_keys.remove(&info.get_store_key());
                            file_cache_lock.set(info);
                        }

                        for key in exist_keys {
                            file_cache_lock.del(&key);
                        }

                        drop(file_cache_lock);

                        config.next_update_at = get_f64_timestamp() + MILL_A_DAY as f64;
                        let _ = self.store.set(config).await;

                        tracing::debug!(
                            "handle file done: {}, time: {}s",
                            url,
                            time.elapsed().as_secs()
                        );

                        let _ = self.dns_events_tx.send(DnsEvent::GeositeUpdated).await;
                    }
                    Err(e) => tracing::error!("read {} response error: {}", url, e),
                },
                Ok(resp) => {
                    tracing::error!("download {} error, HTTP status: {}", url, resp.status());
                }
                Err(e) => {
                    tracing::error!("request {} error: {}", url, e);
                }
            }
        }

        if force {
            let mut file_cache_lock = self.file_cache.lock().await;
            let need_to_remove = file_cache_lock
                .keys()
                .into_iter()
                .filter(|k| !config_names.contains(&k.name))
                .collect::<HashSet<GeoFileCacheKey>>();
            for key in need_to_remove {
                file_cache_lock.del(&key);
            }
        }
    }
}

impl GeoSiteService {
    pub async fn list_all_keys(&self) -> Vec<GeoFileCacheKey> {
        let lock = self.file_cache.lock().await;
        lock.keys()
    }

    pub async fn get_cache_value_by_key(&self, key: &GeoFileCacheKey) -> Option<GeoDomainConfig> {
        let mut lock = self.file_cache.lock().await;
        lock.get(key)
    }

    pub async fn query_geo_by_name(&self, name: Option<String>) -> Vec<GeoSiteSourceConfig> {
        self.store.query_by_name(name).await.unwrap()
    }

    pub async fn update_geo_config_by_bytes(&self, name: String, file_bytes: impl Into<Vec<u8>>) {
        let result = landscape_protobuf::read_geo_sites_from_bytes(file_bytes).await;
        {
            let mut file_cache_lock = self.file_cache.lock().await;
            for (key, values) in result {
                let info = GeoDomainConfig {
                    name: name.clone(),
                    key: key.to_ascii_uppercase(),
                    values,
                };
                file_cache_lock.set(info);
            }
        }
        let _ = self.dns_events_tx.send(DnsEvent::GeositeUpdated).await;
    }
}

#[async_trait::async_trait]
impl ConfigController for GeoSiteService {
    type Id = Uuid;

    type Config = GeoSiteSourceConfig;

    type DatabseAction = GeoSiteConfigRepository;

    fn get_repository(&self) -> &Self::DatabseAction {
        &self.store
    }
}
