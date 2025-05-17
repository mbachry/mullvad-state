use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::error::Error;
use futures_util::stream::StreamExt;
use zbus::{zvariant::OwnedObjectPath, proxy, Connection};
use serde::Deserialize;
use tokio_retry::strategy::ExponentialBackoff;

const MULLVAD_API_URL: &str = "https://ipv4.am.i.mullvad.net/json";

#[derive(Clone, Copy, Eq, PartialEq, strum_macros::Display)]
enum ConnectionState {
    Uninitialized,
    Connected,
    Disconnected,
    Unknown,
}

#[derive(Deserialize)]
struct MullvadResponse {
    mullvad_exit_ip: bool,
}

async fn __get_vpn_state() -> Result<bool, Box<dyn Error>> {
    let client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(2)).build()?;
    let request =  client.get(MULLVAD_API_URL).build()?;
    let resp = client.execute(request).await?.json::<MullvadResponse>().await?;
    Ok(resp.mullvad_exit_ip)
}

async fn _get_vpn_state() -> Result<bool, Box<dyn Error>> {
    let retry_strategy = ExponentialBackoff::from_millis(500).take(5);
    tokio_retry::Retry::spawn(retry_strategy, __get_vpn_state).await
}

async fn get_vpn_state() -> ConnectionState {
    match _get_vpn_state().await {
        Ok(is_connected) => if is_connected { ConnectionState::Connected } else { ConnectionState::Disconnected },
        Err(err) => {
            eprintln!("Failed to check mullvad state: {}", err);
            ConnectionState::Unknown
        }
    }
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager",
    interface = "org.freedesktop.NetworkManager"
)]
trait NetworkManager {
    #[zbus(signal)]
    fn device_added(&self, dev: OwnedObjectPath) -> zbus::Result<()>;
    #[zbus(signal)]
    fn device_removed(&self, dev: OwnedObjectPath) -> zbus::Result<()>;
}

struct MullvadState {
    vpn_connected: RwLock<ConnectionState>,
}

impl MullvadState {
    fn new() -> Self {
        MullvadState{vpn_connected: RwLock::new(ConnectionState::Uninitialized)}
    }

    async fn check(&self) {
        let state = get_vpn_state().await;
        let mut cell = self.vpn_connected.write().unwrap();
        *cell = state;
    }

    fn vpn_connected(&self) -> ConnectionState {
        let cell = self.vpn_connected.read().unwrap();
        *cell
    }
}

async fn handle_device_change(state: &MullvadState) {
    // sleep for a bit to let network reconfigure
    tokio::time::sleep(Duration::from_millis(500)).await;
    let old_state = state.vpn_connected();
    state.check().await;
    let new_state = state.vpn_connected();
    if old_state != new_state {
        println!("vpn status changed to {}", new_state);
    }
}

async fn watch_network_state(state: &MullvadState) -> Result<(), Box<dyn Error>> {
    let connection = Connection::system().await?;
    let systemd_proxy = NetworkManagerProxy::new(&connection).await?;
    let mut device_added_stream = systemd_proxy.receive_device_added().await?;
    let mut device_removed_stream = systemd_proxy.receive_device_removed().await?;

    loop {
        let got_msg = futures_util::select! {
            msg = device_added_stream.next() => msg.is_some(),
            msg = device_removed_stream.next() => msg.is_some(),
            complete => panic!("Stream ended unexpectedly"),
        };
        if got_msg {
            handle_device_change(state).await;
        }
    }
}

struct DbusServer {
    state: Arc<MullvadState>
}

#[zbus::interface(name = "org.mbachry.Mullvad")]
impl DbusServer {
    async fn get_vpn_state(&self) -> String {
        self.state.vpn_connected().to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let state = Arc::new(MullvadState::new());
    state.check().await;

    println!("Initial connection state = {}", state.vpn_connected());

    let cloned_state = state.clone();
    let server = DbusServer{state: cloned_state};

    let connection = Connection::session().await?;
    connection
        .object_server()
        .at("/org/mbachry/Mullvad", server)
        .await?;
    connection
        .request_name("org.mbachry.Mullvad")
        .await?;

    watch_network_state(&state).await?;
    Ok(())
}
