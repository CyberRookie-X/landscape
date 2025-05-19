use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use landscape::service::flow_wan_service::FlowWanServiceManagerService;
use landscape_common::service::controller_service::ControllerService;
use landscape_common::{
    config::flow::FlowWanServiceConfig, observer::IfaceObserverAction,
    service::DefaultWatchServiceStatus,
};
use landscape_database::provider::LandscapeDBServiceProvider;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::{error::LandscapeApiError, SimpleResult};

pub async fn get_iface_flow_wan_paths(
    store: LandscapeDBServiceProvider,
    dev_observer: broadcast::Receiver<IfaceObserverAction>,
) -> Router {
    let share_state = FlowWanServiceManagerService::new(store, dev_observer).await;

    Router::new()
        .route("/packet_marks/status", get(get_all_nat_status))
        .route("/packet_marks", post(handle_iface_nat_status))
        .route(
            "/packet_marks/:iface_name",
            get(get_iface_nat_conifg).delete(delete_and_stop_iface_nat),
        )
        // .route("/packet_marks/:iface_name/restart", post(restart_mark_service_status))
        .with_state(share_state)
}

async fn get_all_nat_status(State(state): State<FlowWanServiceManagerService>) -> Json<Value> {
    let result = serde_json::to_value(state.get_all_status().await);
    Json(result.unwrap())
}

async fn get_iface_nat_conifg(
    State(state): State<FlowWanServiceManagerService>,
    Path(iface_name): Path<String>,
) -> Result<Json<FlowWanServiceConfig>, LandscapeApiError> {
    if let Some(iface_config) = state.get_config_by_name(iface_name).await {
        Ok(Json(iface_config))
    } else {
        Err(LandscapeApiError::NotFound("can not find".into()))
    }
}

async fn handle_iface_nat_status(
    State(state): State<FlowWanServiceManagerService>,
    Json(config): Json<FlowWanServiceConfig>,
) -> Json<SimpleResult> {
    let result = SimpleResult { success: true };
    state.handle_service_config(config).await;
    Json(result)
}

async fn delete_and_stop_iface_nat(
    State(state): State<FlowWanServiceManagerService>,
    Path(iface_name): Path<String>,
) -> Json<Option<DefaultWatchServiceStatus>> {
    Json(state.delete_and_stop_iface_service(iface_name).await)
}
