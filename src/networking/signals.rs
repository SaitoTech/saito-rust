use tokio::signal;

#[cfg(target_os = "unix")]
pub async fn signal_for_shutdown() {
    let mut interrupt_signal = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("Error setting up interrupt signal");

    let mut terminate_signal = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Error setting up terminate signal");

    let mut quit_signal = signal::unix::signal(signal::unix::SignalKind::quit())
        .expect("Error setting up quit signal");

    tokio::select! {
        _ = signal::ctrl_c() => (),
        _ = interrupt_signal.recv() => (),
        _ = terminate_signal.recv() => (),
        _ = quit_signal.recv() => (),
    }
}

#[cfg(not(target_os = "unix"))]
pub async fn signal_for_shutdown() {
    signal::ctrl_c().await.ok();
}
