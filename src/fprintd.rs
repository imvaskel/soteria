//! Optional fprintd integration.
//!
//! Soteria only talks to fprintd as a D-Bus *client* (fprintd itself is left
//! untouched) to detect whether the fingerprint reader is currently claimed
//! by another process. This matters for the biometric flow: when a Polkit
//! authentication is cancelled while a fingerprint scan is running, the
//! socket-activated `polkit-agent-helper-1` keeps the reader claimed until
//! the PAM conversation finishes (up to its full timeout). Starting another
//! scan during that window would just fail with `AlreadyInUse`, so Soteria
//! falls back to the password prompt instead.

use std::time::Duration;

use zbus::zvariant::OwnedObjectPath;

const FPRINTD_SERVICE: &str = "net.reactivated.Fprint";

#[zbus::proxy(
    interface = "net.reactivated.Fprint.Manager",
    default_service = "net.reactivated.Fprint",
    default_path = "/net/reactivated/Fprint/Manager"
)]
trait FprintManager {
    fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[zbus::proxy(interface = "net.reactivated.Fprint.Device")]
trait FprintDevice {
    fn claim(&self, username: &str) -> zbus::Result<()>;
    fn release(&self) -> zbus::Result<()>;
}

/// Returns `true` when at least one fingerprint reader exists and is already
/// claimed by another process.
///
/// The probe temporarily claims the reader for the current user and releases
/// it again right away when it is free. Any failure to reach fprintd is
/// treated as "not busy" so the probe itself can never block authentication.
pub async fn fingerprint_device_busy() -> bool {
    let connection = match zbus::conn::Builder::system() {
        Ok(builder) => match builder.build().await {
            Ok(connection) => connection,
            Err(error) => {
                tracing::debug!(
                    "could not connect to the system bus for the fprintd probe: {error}"
                );
                return false;
            }
        },
        Err(error) => {
            tracing::debug!("could not connect to the system bus for the fprintd probe: {error}");
            return false;
        }
    };

    let manager = match FprintManagerProxy::new(&connection).await {
        Ok(manager) => manager,
        Err(error) => {
            tracing::debug!("could not reach the fprintd manager: {error}");
            return false;
        }
    };

    let devices = match manager.get_devices().await {
        Ok(devices) => devices,
        Err(error) => {
            tracing::debug!("could not enumerate fingerprint devices: {error}");
            return false;
        }
    };

    for path in devices {
        let Some(device) = device_proxy(&connection, path.as_str()).await else {
            continue;
        };

        match device.claim("").await {
            Ok(()) => {
                // The reader is free: release the temporary claim immediately.
                if let Err(error) = device.release().await {
                    tracing::debug!("could not release the temporary fprintd claim: {error}");
                }
            }
            Err(error) => {
                if let zbus::Error::MethodError(name, _, _) = &error {
                    if name.as_str().ends_with("AlreadyInUse") {
                        tracing::debug!("fingerprint reader is busy");
                        return true;
                    }
                }
                tracing::debug!("unexpected fprintd claim error: {error}");
            }
        }
    }

    false
}

async fn device_proxy<'a>(
    connection: &'a zbus::Connection,
    path: &'a str,
) -> Option<FprintDeviceProxy<'a>> {
    let builder = FprintDeviceProxy::builder(connection);
    let builder = builder.destination(FPRINTD_SERVICE).ok()?;
    let builder = builder.path(path).ok()?;
    builder.build().await.ok()
}

/// Bounds the whole probe to a short timeout so a hung fprintd can never
/// stall the authentication dialog.
pub async fn fingerprint_device_busy_with_timeout() -> bool {
    tokio::time::timeout(Duration::from_secs(3), fingerprint_device_busy())
        .await
        .unwrap_or(false)
}
