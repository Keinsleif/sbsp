// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The browser's side of the backend boundary: a WebSocket to the host that
//! served the page (or any host, by address), with the same names as the
//! native remote's `backend.rs` so the app uses either the same way.
//!
//! The browser cannot link the backend, so this speaks the host's protocol
//! itself (`port::wire`): it authenticates, asks for the full state, and
//! then runs one task that writes the queued commands and reads what the
//! host sends, putting events, file lists and levels where the UI reads
//! them (`port::Link`).

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use gloo_net::websocket::{Message, futures::WebSocket};
use gloo_timers::future::TimeoutFuture;
use iced::futures::{
    FutureExt, SinkExt, StreamExt,
    channel::{mpsc, oneshot},
    future::{self, Either, Shared},
};
use sbsp_backend::{
    api::{FileList, Permissions, WsCommand, WsError, WsFeedback, client::ServiceEntry},
    event::{BackendError, BackendEvent},
};
use crate::settings::GlobalRemoteSettings;

use crate::port::link::{Handoff, Levels, subscribe};
use crate::port::{FullState, Link, PortEvent, wire};

/// Numbers the connections, so each has its own event subscription.
static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);

/// Connecting and authenticating (the browser reports a failed connection
/// only as a closed socket), then the first state.
const AUTH_TIMEOUT_MS: u32 = 15_000;
const STATE_TIMEOUT_MS: u32 = 10_000;

#[derive(Clone)]
pub struct RemotePort {
    link: Link,
    address: String,
    permission: Permissions,
    files: Handoff<Vec<FileList>>,
    disconnect_tx: mpsc::UnboundedSender<()>,
    closed: Shared<oneshot::Receiver<()>>,
}

impl std::ops::Deref for RemotePort {
    type Target = Link;

    fn deref(&self) -> &Link {
        &self.link
    }
}

impl RemotePort {
    /// Connects to `address` (`host:port`), authenticates with `password`
    /// and fetches the host's full state.
    pub async fn connect(
        address: String,
        password: Option<String>,
    ) -> anyhow::Result<(Self, FullState)> {
        let url = format!("{}://{address}/ws", scheme());
        let socket = WebSocket::open(&url).map_err(|e| anyhow::anyhow!("Failed to open {url}: {e}"))?;
        let (mut write, mut read) = socket.split();

        let permission = with_timeout(AUTH_TIMEOUT_MS, "connecting to the host", async {
            loop {
                let message = read
                    .next()
                    .await
                    .ok_or_else(|| anyhow::anyhow!("Connection closed during authentication."))?
                    .map_err(|e| anyhow::anyhow!("WebSocket error during authentication: {e}"))?;
                let Message::Text(text) = message else {
                    continue;
                };
                match wire::decode(&text)? {
                    WsFeedback::Hello { auth } => {
                        let command = wire::authenticate(password.as_deref(), &auth);
                        let text = wire::encode(&command)
                            .ok_or_else(|| anyhow::anyhow!("Failed to encode a command."))?;
                        write
                            .send(Message::Text(text))
                            .await
                            .map_err(|e| anyhow::anyhow!("WebSocket send failed: {e}"))?;
                    }
                    WsFeedback::Authenticated { perm } => return Ok(perm),
                    WsFeedback::Error(error) => {
                        anyhow::bail!("Authentication rejected by server: {error:?}")
                    }
                    _ => anyhow::bail!("Unexpected message during authentication."),
                }
            }
        })
        .await?;

        let request = wire::encode(&WsCommand::RequestFullShowState)
            .ok_or_else(|| anyhow::anyhow!("Failed to encode a command."))?;
        write
            .send(Message::Text(request))
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket send failed: {e}"))?;
        let state = with_timeout(STATE_TIMEOUT_MS, "waiting for the show", async {
            loop {
                let message = read
                    .next()
                    .await
                    .ok_or_else(|| anyhow::anyhow!("Connection closed before the show arrived."))?
                    .map_err(|e| anyhow::anyhow!("WebSocket error: {e}"))?;
                let Message::Text(text) = message else {
                    continue;
                };
                match wire::decode(&text)? {
                    WsFeedback::FullShowState(state) => return Ok(*state),
                    WsFeedback::Error(error) => anyhow::bail!("The host refused: {error:?}"),
                    // Events before the snapshot are older than it.
                    _ => {}
                }
            }
        })
        .await?;

        let (commands_tx, commands_rx) = mpsc::unbounded::<String>();
        let (events_tx, events_rx) = mpsc::unbounded::<PortEvent>();
        let (files_tx, files_rx) = mpsc::unbounded::<Vec<FileList>>();
        let (disconnect_tx, disconnect_rx) = mpsc::unbounded::<()>();
        let (closed_tx, closed_rx) = oneshot::channel::<()>();
        let level = Arc::new(Levels::default());

        wasm_bindgen_futures::spawn_local(pump(
            write,
            read,
            commands_rx,
            disconnect_rx,
            events_tx,
            files_tx,
            level.clone(),
            closed_tx,
        ));

        let id = NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed);
        let port = Self {
            link: Link::new(commands_tx, events_rx, level, id),
            address,
            permission,
            files: Arc::new(Mutex::new(Some(files_rx))),
            disconnect_tx,
            closed: closed_rx.shared(),
        };
        Ok((port, state))
    }

    /// What the host lets this connection do.
    pub fn permission(&self) -> Permissions {
        self.permission
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Asks the connection to close.
    pub fn disconnect(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let tx = self.disconnect_tx.clone();
        async move {
            let _ = tx.unbounded_send(());
        }
    }

    /// Finishes when the connection has ended, whoever ended it.
    pub fn closed(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let closed = self.closed.clone();
        async move {
            let _ = closed.await;
        }
    }

    /// Asks the host for the list of files it can offer.
    pub fn request_file_list(
        &self,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let result = self.link.push(vec![WsCommand::RequestAssetList]);
        async move { result }
    }

    /// The lists of files the host sends after `request_file_list`.
    pub fn file_lists(&self) -> iced::Subscription<Vec<FileList>> {
        subscribe(self.files.clone(), self.link.id(), "sbsp-web-file-lists")
    }
}

/// `ws` on a page served over HTTP, `wss` over HTTPS (the browser refuses
/// the other mix).
fn scheme() -> &'static str {
    let secure = web_sys::window()
        .and_then(|window| window.location().protocol().ok())
        .is_some_and(|protocol| protocol == "https:");
    if secure { "wss" } else { "ws" }
}

async fn with_timeout<T>(
    millis: u32,
    what: &str,
    work: impl std::future::Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    match future::select(Box::pin(work), Box::pin(TimeoutFuture::new(millis))).await {
        Either::Left((result, _)) => result,
        Either::Right(_) => anyhow::bail!("Timed out while {what}."),
    }
}

enum Wake {
    Frame(Option<Result<Message, gloo_net::websocket::WebSocketError>>),
    Command(Option<String>),
    Disconnect,
}

/// Runs for as long as the connection does: writes what the UI queued,
/// reads what the host sends.
#[allow(clippy::too_many_arguments)]
async fn pump(
    mut write: iced::futures::stream::SplitSink<WebSocket, Message>,
    mut read: iced::futures::stream::SplitStream<WebSocket>,
    mut commands: mpsc::UnboundedReceiver<String>,
    mut disconnect: mpsc::UnboundedReceiver<()>,
    events: mpsc::UnboundedSender<PortEvent>,
    files: mpsc::UnboundedSender<Vec<FileList>>,
    level: Arc<Levels>,
    closed: oneshot::Sender<()>,
) {
    loop {
        let wake = match future::select(
            read.next().map(Wake::Frame),
            future::select(
                commands.next().map(Wake::Command),
                disconnect.next().map(|_| Wake::Disconnect),
            ),
        )
        .await
        {
            Either::Left((wake, _)) => wake,
            Either::Right((Either::Left((wake, _)), _)) => wake,
            Either::Right((Either::Right((wake, _)), _)) => wake,
        };

        match wake {
            Wake::Frame(Some(Ok(Message::Text(text)))) => feed(&text, &events, &files, &level),
            Wake::Frame(Some(Ok(Message::Bytes(_)))) => {}
            Wake::Frame(Some(Err(e))) => {
                log::info!("WebSocket closed: {e}");
                break;
            }
            Wake::Frame(None) => {
                log::info!("WebSocket closed by the host.");
                break;
            }
            Wake::Command(Some(text)) => {
                if let Err(e) = write.send(Message::Text(text)).await {
                    log::info!("WebSocket send failed: {e}");
                    break;
                }
            }
            // Every handle to the connection is gone.
            Wake::Command(None) => break,
            Wake::Disconnect => {
                let _ = write.close().await;
                break;
            }
        }
    }
    let _ = closed.send(());
}

/// Puts one message from the host where the UI reads it.
fn feed(
    text: &str,
    events: &mpsc::UnboundedSender<PortEvent>,
    files: &mpsc::UnboundedSender<Vec<FileList>>,
    level: &Levels,
) {
    let failed = |id: usize, message: &str| {
        let _ = events.unbounded_send(BackendEvent::OperationFailed {
            error: BackendError::Custom {
                id,
                message: message.to_string(),
            },
        });
    };

    match wire::decode(text) {
        Ok(WsFeedback::Event(event)) => {
            let _ = events.unbounded_send(*event);
        }
        Ok(WsFeedback::AssetList(list)) => {
            let _ = files.unbounded_send(list);
        }
        Ok(WsFeedback::Level { left, right }) => level.set(left, right),
        Ok(WsFeedback::Error(WsError::AuthenticationFailed)) => failed(1, "Authentication Failed"),
        Ok(WsFeedback::Error(WsError::AuthenticationRateLimitExceeded)) => {
            failed(2, "Authentication rate limit exceeded.")
        }
        Ok(WsFeedback::Error(WsError::PermissionDenied)) => failed(3, "Permission denied."),
        // The snapshot is only asked for at connect time.
        Ok(_) => {}
        Err(e) => log::error!("Invalid message received: {e}"),
    }
}

/// The settings of this browser, kept in its `localStorage`.
#[derive(Clone)]
pub struct RemoteSettings;

const SETTINGS_KEY: &str = "sbsp.web.settings";

impl RemoteSettings {
    /// The saved settings, or the defaults (nothing saved, or not readable).
    pub async fn load() -> (Self, GlobalRemoteSettings) {
        let settings = crate::browser::get(SETTINGS_KEY)
            .and_then(|json| {
                serde_json::from_str(&json)
                    .map_err(|e| log::warn!("ignoring the saved settings: {e}"))
                    .ok()
            })
            .unwrap_or_default();
        (Self, settings)
    }

    pub fn save(
        &self,
        new: GlobalRemoteSettings,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let json = serde_json::to_string(&new).map_err(anyhow::Error::from);
        async move {
            crate::browser::set(SETTINGS_KEY, &json?);
            Ok(())
        }
    }
}

/// There is no looking for hosts from a browser.
pub fn discovery() -> iced::Subscription<Vec<ServiceEntry>> {
    iced::Subscription::none()
}
