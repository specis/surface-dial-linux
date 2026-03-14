use std::sync::mpsc;
use std::thread;

use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::Result;

enum DbusEvent {
    Rotated(i32),
    Pressed,
    Released,
}

struct DialInterface;

#[zbus::interface(name = "com.dialmenu.Daemon")]
impl DialInterface {
    #[zbus(signal)]
    async fn dial_rotated(
        ctx: &zbus::object_server::SignalContext<'_>,
        delta: i32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn dial_pressed(
        ctx: &zbus::object_server::SignalContext<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn dial_released(
        ctx: &zbus::object_server::SignalContext<'_>,
    ) -> zbus::Result<()>;
}

pub struct DbusMode {
    tx: mpsc::SyncSender<DbusEvent>,
}

impl DbusMode {
    pub fn new() -> DbusMode {
        let (tx, rx) = mpsc::sync_channel::<DbusEvent>(32);

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("DbusMode: failed to create tokio runtime: {}", e);
                    return;
                }
            };

            rt.block_on(async move {
                let conn = match zbus::Connection::session().await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("DbusMode: failed to connect to session bus: {}", e);
                        return;
                    }
                };

                if let Err(e) = conn
                    .object_server()
                    .at("/com/dialmenu/Daemon", DialInterface)
                    .await
                {
                    eprintln!("DbusMode: failed to register D-Bus interface: {}", e);
                    return;
                }

                if let Err(e) = conn.request_name("com.dialmenu.Daemon").await {
                    eprintln!(
                        "DbusMode: failed to request service name \
                         (another instance may be running): {}",
                        e
                    );
                    // Continue — signals can still be emitted without owning the name.
                }

                let signal_ctx = match zbus::object_server::SignalContext::new(
                    &conn,
                    "/com/dialmenu/Daemon",
                ) {
                    Ok(ctx) => ctx,
                    Err(e) => {
                        eprintln!("DbusMode: failed to create signal context: {}", e);
                        return;
                    }
                };

                while let Ok(evt) = rx.recv() {
                    match evt {
                        DbusEvent::Rotated(delta) => {
                            if let Err(e) =
                                DialInterface::dial_rotated(&signal_ctx, delta).await
                            {
                                eprintln!("DbusMode: failed to emit DialRotated: {}", e);
                            }
                        }
                        DbusEvent::Pressed => {
                            if let Err(e) = DialInterface::dial_pressed(&signal_ctx).await {
                                eprintln!("DbusMode: failed to emit DialPressed: {}", e);
                            }
                        }
                        DbusEvent::Released => {
                            if let Err(e) = DialInterface::dial_released(&signal_ctx).await {
                                eprintln!("DbusMode: failed to emit DialReleased: {}", e);
                            }
                        }
                    }
                }
            });
        });

        DbusMode { tx }
    }

    fn send(&self, evt: DbusEvent) {
        if let Err(e) = self.tx.try_send(evt) {
            eprintln!("DbusMode: failed to queue D-Bus event: {}", e);
        }
    }
}

impl ControlMode for DbusMode {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: "DbusMode".into(),
            icon: "network-transmit".into(),
            haptics: false,
            steps: 36,
        }
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> {
        self.send(DbusEvent::Pressed);
        Ok(())
    }

    fn on_btn_release(&mut self, _: &DialHaptics) -> Result<()> {
        self.send(DbusEvent::Released);
        Ok(())
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        self.send(DbusEvent::Rotated(delta));
        Ok(())
    }
}
