// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The browser's counterpart of `link.rs`: the same calls for the UI, but
//! each one is turned into the host's wire messages (`wire`) and queued for
//! the connection, instead of going to a `BackendHandle`. The connection
//! itself (`remote::web_backend`) writes the queue to the WebSocket and
//! puts what the host sends into the event queue and the level here.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering},
};

use iced::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use sbsp_backend::api::WsCommand;

use super::{Control, ModelEdit, PortEvent, wire};

/// The host's output level as it last arrived (the bits of two `f32`s).
#[derive(Default)]
pub struct Levels {
    left: AtomicU32,
    right: AtomicU32,
}

impl Levels {
    pub fn set(&self, left: f32, right: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }

    fn get(&self) -> (f32, f32) {
        (
            f32::from_bits(self.left.load(Ordering::Relaxed)),
            f32::from_bits(self.right.load(Ordering::Relaxed)),
        )
    }
}

/// A queue's receiving end that is handed to its subscription once.
pub type Handoff<T> = Arc<Mutex<Option<UnboundedReceiver<T>>>>;

#[derive(Clone)]
pub struct Link {
    /// Text frames for the host, in order.
    commands: UnboundedSender<String>,
    events: Handoff<PortEvent>,
    level: Arc<Levels>,
    /// Which connection this is; the event subscription is keyed on it.
    id: u64,
}

impl Link {
    pub fn new(
        commands: UnboundedSender<String>,
        events: UnboundedReceiver<PortEvent>,
        level: Arc<Levels>,
        id: u64,
    ) -> Self {
        Self {
            commands,
            events: Arc::new(Mutex::new(Some(events))),
            level,
            id,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Queues the commands, in order. Fails when the connection is gone.
    pub(crate) fn push(&self, commands: Vec<WsCommand>) -> anyhow::Result<()> {
        for command in commands {
            let text = wire::encode(&command)
                .ok_or_else(|| anyhow::anyhow!("Failed to encode a command."))?;
            self.commands
                .unbounded_send(text)
                .map_err(|_| anyhow::anyhow!("The connection is closed."))?;
        }
        Ok(())
    }

    pub fn send(
        &self,
        controls: Vec<Control>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let result = self.push(wire::controls(controls));
        async move { result }
    }

    pub fn edit(
        &self,
        edit: ModelEdit,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let result = self.push(wire::edit(edit));
        async move { result }
    }

    pub fn request_asset(
        &self,
        path: std::path::PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let result = self.push(vec![wire::request_asset(path)]);
        async move { result }
    }

    pub fn request_state_sync(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        if let Err(e) = self.push(vec![WsCommand::RequestSyncState]) {
            log::debug!("state sync request dropped: {e}");
        }
        async {}
    }

    pub fn stream_level(
        &self,
        enabled: bool,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        if let Err(e) = self.push(vec![WsCommand::SubscribeLevel { enabled }]) {
            log::debug!("level stream request dropped: {e}");
        }
        async {}
    }

    pub fn level(&self) -> Option<(f32, f32)> {
        Some(self.level.get())
    }

    /// The browser only follows a show; the host owns it, and there is
    /// nothing here to save.
    pub fn is_modified(&self) -> bool {
        false
    }

    /// Every event the host sends, as they arrive.
    pub fn events(&self) -> iced::Subscription<PortEvent> {
        subscribe(self.events.clone(), self.id, "sbsp-web-events")
    }
}

/// A subscription over a queue that is handed over once. It is keyed on the
/// connection's number, so a new connection is listened to anew.
pub fn subscribe<T: Send + 'static>(
    queue: Handoff<T>,
    id: u64,
    name: &'static str,
) -> iced::Subscription<T> {
    struct Source<T>(Handoff<T>, u64, &'static str);

    impl<T> std::hash::Hash for Source<T> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.2.hash(state);
            self.1.hash(state);
        }
    }

    iced::Subscription::run_with(Source(queue, id, name), |Source(queue, _, _)| {
        let receiver = queue.lock().ok().and_then(|mut slot| slot.take());
        iced::stream::channel(64, async move |mut output| {
            use iced::futures::{SinkExt, StreamExt};

            let Some(mut receiver) = receiver else {
                // Already listened to; nothing more to hand out.
                return std::future::pending::<()>().await;
            };
            while let Some(item) = receiver.next().await {
                if output.send(item).await.is_err() {
                    break;
                }
            }
        })
    })
}
