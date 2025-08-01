use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, RwLock};

use crate::store::storev2::LandscapeStore;

use super::service_code::{WatchService, WatchServiceTrait};

#[async_trait::async_trait]
pub trait ServiceStarterTrait: Clone + Send + Sync + 'static {
    type Status: WatchServiceTrait + Send + Sync + 'static;
    type Config: LandscapeStore + Send + Sync + 'static;

    /// 核心服务初始化逻辑
    async fn start(&self, config: Self::Config) -> WatchService<Self::Status>;
}

/// T: 定义被观察的状态
/// S：存储的配置
#[derive(Clone)]
pub struct ServiceManager<H: ServiceStarterTrait> {
    pub services: Arc<RwLock<HashMap<String, (WatchService<H::Status>, mpsc::Sender<H::Config>)>>>,
    pub starter: H,
}

impl<H: ServiceStarterTrait> ServiceManager<H> {
    pub async fn init(init_config: Vec<H::Config>, starter: H) -> Self {
        let services = HashMap::new();
        let manager = Self { services: Arc::new(RwLock::new(services)), starter };

        for config in init_config {
            manager.spawn_service(config).await;
        }
        manager
    }

    async fn spawn_service(&self, service_config: H::Config) {
        let key = service_config.get_store_key();
        let (tx, mut rx) = mpsc::channel(1);
        let _ = tx.send(service_config).await;
        let service_status = WatchService::new();

        // 插入到服务映射
        {
            self.services.write().await.insert(key.clone(), (service_status.clone(), tx));
        }

        let service_map = self.services.clone();
        let starter = self.starter.clone();
        tokio::spawn(async move {
            let mut iface_status: Option<WatchService<H::Status>> = Some(service_status);

            while let Some(config) = rx.recv().await {
                if let Some(exist_status) = iface_status.take() {
                    exist_status.wait_stop().await;
                    drop(exist_status);
                }

                let key = config.get_store_key();
                let status = starter.clone().start(config).await;

                iface_status = Some(status.clone());
                let mut write_lock = service_map.write().await;
                if let Some((target, _)) = write_lock.get_mut(&key) {
                    *target = status;
                } else {
                    tracing::error!("get service map lock error, break loop");
                    break;
                }
                drop(write_lock);
            }

            if let Some(exist_status) = iface_status.take() {
                tracing::error!("exist running service, stop it");
                exist_status.wait_stop().await;
            }
        });
    }

    pub async fn update_service(&self, config: H::Config) -> Result<(), ()> {
        let key = config.get_store_key();
        let read_lock = self.services.read().await;
        if let Some((_, sender)) = read_lock.get(&key) {
            let result = if let Err(e) = sender.try_send(config) {
                match e {
                    mpsc::error::TrySendError::Full(_) => {
                        tracing::error!("已经有配置在等待了");
                        Err(())
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        tracing::error!("内部错误");
                        Err(())
                    }
                }
            } else {
                Ok(())
            };
            drop(read_lock);
            result
        } else {
            drop(read_lock);
            self.spawn_service(config).await;
            Ok(())
        }
    }

    pub async fn get_all_status(&self) -> HashMap<String, WatchService<H::Status>> {
        let read_lock = self.services.read().await;
        let mut result = HashMap::new();
        for (key, (iface_status, _)) in read_lock.iter() {
            result.insert(key.clone(), iface_status.clone());
        }
        result
    }

    pub async fn stop_service(&self, name: String) -> Option<WatchService<H::Status>> {
        let mut write_lock = self.services.write().await;
        if let Some((iface_status, _)) = write_lock.remove(&name) {
            drop(write_lock);
            iface_status.wait_stop().await;
            Some(iface_status)
        } else {
            None
        }
    }
}
