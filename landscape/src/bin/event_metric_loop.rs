use landscape_common::{metric::MetricData, LANDSCAPE_METRIC_DIR_NAME};
use landscape_ebpf::metric::new_metric;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::oneshot;

// cargo run --package landscape-ebpf --bin event_metric_loop
#[tokio::main]
async fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .unwrap();

    let metric_path = PathBuf::from("/root/.landscape-router").join(LANDSCAPE_METRIC_DIR_NAME);

    let (tx, rx) = oneshot::channel::<()>();
    let (other_tx, other_rx) = oneshot::channel::<()>();

    let metric_service = MetricData::new(metric_path).await;
    let metric_service_clone = metric_service.clone();
    std::thread::spawn(move || {
        new_metric(rx, metric_service_clone);
        let _ = other_tx.send(());
    });

    while running.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::new(1, 0)).await;
        println!("data: {:?}", metric_service.connect_metric.connect_infos().await);
    }

    let _ = tx.send(());
    let _ = other_rx.await;
}
