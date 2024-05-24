use bevy::{
    ecs::{event::Event, system::SystemId},
    log::debug,
};
use bevy_crossbeam_event::CrossbeamEventSender;
use crossbeam::channel::{unbounded, Receiver, SendError, Sender};
use serde_json::Value;
use uuid::Uuid;

use super::client::ClientManager;
use crate::message::{
    payload::{
        AccessTokenPayload, BroadcastConfig, BroadcastPayload, JoinConfig, JoinPayload, Payload,
        PayloadStatus, PostgresChange, PostgresChangesEvent, PostgresChangesPayload,
        PresenceConfig,
    },
    postgres_change_filter::PostgresChangeFilter,
    realtime_message::{MessageEvent, RealtimeMessage},
};

use super::client::Client;
use crate::presence::{Presence, PresenceCallback, PresenceEvent, PresenceState};
use std::{collections::HashMap, error::Error};
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
struct CdcCallback(
    PostgresChangeFilter,
    Arc<dyn Fn(&PostgresChangesPayload) + Send + Sync>,
);

#[derive(Clone)]
struct BroadcastCallback(Arc<dyn Fn(&HashMap<String, Value>) + Send + Sync>);

/// Channel states
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ChannelState {
    Closed,
    Errored,
    Joined,
    Joining,
    Leaving,
}

#[derive(Clone)]
pub struct ChannelManager {
    pub tx: Sender<ChannelManagerMessage>,
}

pub enum ChannelManagerMessage {
    Broadcast { payload: BroadcastPayload },
    Subscribe,
    Track { payload: HashMap<String, Value> },
    Untrack,
    PresenceState { callback: SystemId<PresenceState> },
    ChannelState { callback: SystemId<ChannelState> },
}

impl ChannelManager {
    pub fn broadcast(
        &self,
        payload: BroadcastPayload,
    ) -> Result<(), SendError<ChannelManagerMessage>> {
        self.tx.send(ChannelManagerMessage::Broadcast { payload })
    }

    pub fn subscribe(&self) -> Result<(), SendError<ChannelManagerMessage>> {
        self.tx.send(ChannelManagerMessage::Subscribe)
    }

    pub fn track(
        &self,
        payload: HashMap<String, Value>,
    ) -> Result<(), SendError<ChannelManagerMessage>> {
        self.tx.send(ChannelManagerMessage::Track { payload })
    }

    pub fn untrack(&self) -> Result<(), SendError<ChannelManagerMessage>> {
        self.tx.send(ChannelManagerMessage::Untrack)
    }

    pub fn presence_state(
        &self,
        callback: SystemId<PresenceState>,
    ) -> Result<(), SendError<ChannelManagerMessage>> {
        self.tx
            .send(ChannelManagerMessage::PresenceState { callback })
    }

    pub fn channel_state(
        &self,
        callback: SystemId<ChannelState>,
    ) -> Result<(), SendError<ChannelManagerMessage>> {
        self.tx
            .send(ChannelManagerMessage::ChannelState { callback })
    }
}

#[derive(Event, Clone)]
pub struct PresenceStateCallbackEvent(pub (SystemId<PresenceState>, PresenceState));

#[derive(Event, Clone)]
pub struct ChannelStateCallbackEvent(pub (SystemId<ChannelState>, ChannelState));

/// Channel structure
pub struct RealtimeChannel {
    pub(crate) topic: String,
    pub(crate) connection_state: ChannelState,
    pub(crate) id: Uuid,
    cdc_callbacks: HashMap<PostgresChangesEvent, Vec<CdcCallback>>,
    broadcast_callbacks: HashMap<String, Vec<BroadcastCallback>>,
    join_payload: JoinPayload,
    presence: Presence,
    // sync bridge
    tx: Sender<RealtimeMessage>,
    manager_rx: Receiver<ChannelManagerMessage>,
    presence_state_callback_event_sender: CrossbeamEventSender<PresenceStateCallbackEvent>,
    channel_state_callback_event_sender: CrossbeamEventSender<ChannelStateCallbackEvent>,
}

// TODO channel options with broadcast + presence settings

impl RealtimeChannel {
    pub(crate) fn manager_recv(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(message) = self.manager_rx.try_recv() {
            match message {
                ChannelManagerMessage::Broadcast { payload } => self.broadcast(payload)?,
                ChannelManagerMessage::Subscribe => self.subscribe()?,
                ChannelManagerMessage::Track { payload } => self.track(payload)?,
                ChannelManagerMessage::Untrack => self.untrack()?,
                ChannelManagerMessage::PresenceState { callback } => self
                    .presence_state_callback_event_sender
                    .send(PresenceStateCallbackEvent((
                        callback,
                        self.presence_state(),
                    ))),
                ChannelManagerMessage::ChannelState { callback } => self
                    .channel_state_callback_event_sender
                    .send(ChannelStateCallbackEvent((callback, self.channel_state()))),
            }
        }

        Ok(())
    }
    /// Returns the channel's connection state
    fn channel_state(&self) -> ChannelState {
        self.connection_state
    }

    /// Send a join request to the channel
    /// Does not block, for blocking behaviour use [RealtimeClient::block_until_subscribed()]
    pub(crate) fn subscribe(&mut self) -> Result<(), SendError<RealtimeMessage>> {
        let join_message = RealtimeMessage {
            event: MessageEvent::PhxJoin,
            topic: self.topic.clone(),
            payload: Payload::Join(self.join_payload.clone()),
            message_ref: Some(self.id.into()),
        };

        self.connection_state = ChannelState::Joining;

        self.tx.send(join_message)
    }

    /// Leave the channel
    pub(crate) fn unsubscribe(&mut self) -> Result<ChannelState, SendError<RealtimeMessage>> {
        if self.connection_state == ChannelState::Closed
            || self.connection_state == ChannelState::Leaving
        {
            return Ok(self.connection_state);
        }

        let message = RealtimeMessage {
            event: MessageEvent::PhxLeave,
            topic: self.topic.clone(),
            payload: Payload::Empty {},
            message_ref: Some(format!("{}+leave", self.id)),
        };

        match self.send(message) {
            Ok(()) => {
                self.connection_state = ChannelState::Leaving;
                Ok(self.connection_state)
            }
            Err(e) => Err(e),
        }
    }

    /// Returns the current [PresenceState] of the channel
    fn presence_state(&self) -> PresenceState {
        self.presence.state.clone()
    }

    /// Track provided state in Realtime Presence
    fn track(&mut self, payload: HashMap<String, Value>) -> Result<(), SendError<RealtimeMessage>> {
        self.send(RealtimeMessage {
            event: MessageEvent::Presence,
            topic: self.topic.clone(),
            payload: Payload::PresenceTrack(payload.into()),
            message_ref: None,
        })
    }

    /// Sends a message to stop tracking this channel's presence
    fn untrack(&mut self) -> Result<(), SendError<RealtimeMessage>> {
        self.send(RealtimeMessage {
            event: MessageEvent::Untrack,
            topic: self.topic.clone(),
            payload: Payload::Empty {},
            message_ref: None,
        })
    }

    /// Send a [RealtimeMessage] on this channel
    fn send(&mut self, message: RealtimeMessage) -> Result<(), SendError<RealtimeMessage>> {
        // inject channel topic to message here
        let mut message = message.clone();
        message.topic = self.topic.clone();

        if self.connection_state == ChannelState::Leaving {
            return Err(SendError(message));
        }

        self.tx.send(message)
    }

    /// Helper function for sending broadcast messages
    fn broadcast(&mut self, payload: BroadcastPayload) -> Result<(), SendError<RealtimeMessage>> {
        self.send(RealtimeMessage {
            event: MessageEvent::Broadcast,
            topic: "".into(),
            payload: Payload::Broadcast(payload),
            message_ref: None,
        })
    }

    pub(crate) fn set_auth(
        &mut self,
        access_token: String,
    ) -> Result<(), SendError<RealtimeMessage>> {
        self.join_payload.access_token = access_token.clone();

        if self.connection_state != ChannelState::Joined {
            return Ok(());
        }

        let access_token_message = RealtimeMessage {
            event: MessageEvent::AccessToken,
            topic: self.topic.clone(),
            payload: Payload::AccessToken(AccessTokenPayload { access_token }),
            ..Default::default()
        };

        self.send(access_token_message)
    }

    pub(crate) fn recieve(&mut self, message: RealtimeMessage) {
        match &message.payload {
            Payload::Response(join_response) => {
                let target_id = message.message_ref.clone().unwrap_or("".to_string());
                if target_id != self.id.to_string() {
                    return;
                }
                if join_response.status == PayloadStatus::Ok {
                    self.connection_state = ChannelState::Joined;
                }
            }
            Payload::PresenceState(state) => self.presence.sync(state.clone().into()),
            Payload::PresenceDiff(raw_diff) => {
                self.presence.sync_diff(raw_diff.clone().into());
            }
            Payload::PostgresChanges(payload) => {
                let event = &payload.data.change_type;

                for cdc_callback in self.cdc_callbacks.get_mut(event).unwrap_or(&mut vec![]) {
                    let filter = &cdc_callback.0;

                    // TODO REFAC pointless message clones when not using result; filter.check
                    // should borrow and return bool/result
                    if let Some(_message) = filter.check(message.clone()) {
                        cdc_callback.1(&payload);
                    }
                }

                for cdc_callback in self
                    .cdc_callbacks
                    .get_mut(&PostgresChangesEvent::All)
                    .unwrap_or(&mut vec![])
                {
                    let filter = &cdc_callback.0;

                    if let Some(_message) = filter.check(message.clone()) {
                        cdc_callback.1(&payload);
                    }
                }
            }
            Payload::Broadcast(payload) => {
                if let Some(callbacks) = self.broadcast_callbacks.get_mut(&payload.event) {
                    for cb in callbacks {
                        cb.0(&payload.payload);
                    }
                }
            }
            _ => {}
        }

        match &message.event {
            MessageEvent::PhxClose => {
                if let Some(message_ref) = message.message_ref {
                    if message_ref == self.id.to_string() {
                        self.connection_state = ChannelState::Closed;
                        debug!("Channel Closed! {:?}", self.id);
                    }
                }
            }
            MessageEvent::PhxReply => {
                if message.message_ref.clone().unwrap_or("#NOREF".to_string())
                    == format!("{}+leave", self.id)
                {
                    self.connection_state = ChannelState::Closed;
                    debug!("Channel Closed! {:?}", self.id);
                }
            }
            _ => {}
        }
    }
}

impl Debug for RealtimeChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "RealtimeChannel {{ name: {:?}, callbacks: [TODO DEBUG]}}",
            self.topic
        ))
    }
}

/// Builder struct for [RealtimeChannel]
///
/// Get access to this through [RealtimeClient::channel()]
#[derive(Event, Clone)]
pub struct ChannelBuilder {
    topic: String,
    access_token: String,
    broadcast: BroadcastConfig,
    presence: PresenceConfig,
    id: Uuid,
    postgres_changes: Vec<PostgresChange>,
    cdc_callbacks: HashMap<PostgresChangesEvent, Vec<CdcCallback>>,
    broadcast_callbacks: HashMap<String, Vec<BroadcastCallback>>,
    presence_callbacks: HashMap<PresenceEvent, Vec<PresenceCallback>>,
    tx: Sender<RealtimeMessage>,
}

impl ChannelBuilder {
    pub(crate) fn new(client: &mut Client) -> Self {
        Self {
            topic: "no_topic".into(),
            access_token: client.access_token.clone(),
            broadcast: Default::default(),
            presence: Default::default(),
            id: Uuid::new_v4(),
            postgres_changes: Default::default(),
            cdc_callbacks: Default::default(),
            broadcast_callbacks: Default::default(),
            presence_callbacks: Default::default(),
            tx: client.get_channel_tx(),
        }
    }

    /// Set the topic of the channel
    pub fn topic(&mut self, topic: impl Into<String>) -> &mut Self {
        self.topic = format!("realtime:{}", topic.into());
        self
    }

    /// Set the broadcast config for this channel
    pub fn set_broadcast_config(&mut self, broadcast_config: BroadcastConfig) -> &mut Self {
        self.broadcast = broadcast_config;
        self
    }

    /// Set the presence config for this channel
    pub fn set_presence_config(&mut self, presence_config: PresenceConfig) -> &mut Self {
        self.presence = presence_config;
        self
    }

    /// Add a postgres changes callback to this channel
    pub fn on_postgres_change(
        &mut self,
        event: PostgresChangesEvent,
        filter: PostgresChangeFilter,
        callback: impl Fn(&PostgresChangesPayload) + 'static + Send + Sync,
    ) -> &mut Self {
        self.postgres_changes.push(PostgresChange {
            event: event.clone(),
            schema: filter.schema.clone(),
            table: filter.table.clone().unwrap_or("".into()),
            filter: filter.filter.clone(),
        });

        if self.cdc_callbacks.get_mut(&event).is_none() {
            self.cdc_callbacks.insert(event.clone(), vec![]);
        }

        self.cdc_callbacks
            .get_mut(&event)
            .unwrap_or(&mut vec![])
            .push(CdcCallback(filter, Arc::new(callback)));

        self
    }

    /// Add a presence callback to this channel
    ///```
    pub fn on_presence(
        &mut self,
        event: PresenceEvent,
        // TODO callback type alias
        callback: impl Fn(String, PresenceState, PresenceState) + 'static + Send + Sync,
    ) -> &mut Self {
        if self.presence_callbacks.get_mut(&event).is_none() {
            self.presence_callbacks.insert(event.clone(), vec![]);
        }

        self.presence_callbacks
            .get_mut(&event)
            .unwrap_or(&mut vec![])
            .push(PresenceCallback(Arc::new(callback)));

        self
    }

    /// Add a broadcast callback to this channel
    pub fn on_broadcast(
        &mut self,
        event: impl Into<String>,
        callback: impl Fn(&HashMap<String, Value>) + 'static + Send + Sync,
    ) -> &mut Self {
        let event: String = event.into();

        if self.broadcast_callbacks.get_mut(&event).is_none() {
            self.broadcast_callbacks.insert(event.clone(), vec![]);
        }

        self.broadcast_callbacks
            .get_mut(&event)
            .unwrap_or(&mut vec![])
            .push(BroadcastCallback(Arc::new(callback)));

        self
    }

    // TODO on_message handler for sys messages

    /// Create the channel and pass ownership to provided [RealtimeClient], returning the channel
    /// id for later access through the client
    pub fn build(
        &self,
        client: &ClientManager,
        presence_state_callback_event_sender: CrossbeamEventSender<PresenceStateCallbackEvent>,
        channel_state_callback_event_sender: CrossbeamEventSender<ChannelStateCallbackEvent>,
    ) -> ChannelManager {
        let manager_channel = unbounded();

        client
            .add_channel(RealtimeChannel {
                topic: self.topic.clone(),
                cdc_callbacks: self.cdc_callbacks.clone(),
                broadcast_callbacks: self.broadcast_callbacks.clone(),
                tx: self.tx.clone(),
                manager_rx: manager_channel.1,
                connection_state: ChannelState::Closed,
                id: self.id,
                join_payload: JoinPayload {
                    config: JoinConfig {
                        broadcast: self.broadcast.clone(),
                        presence: self.presence.clone(),
                        postgres_changes: self.postgres_changes.clone(),
                    },
                    access_token: self.access_token.clone(),
                },
                presence: Presence::from_channel_builder(self.presence_callbacks.clone()),
                presence_state_callback_event_sender,
                channel_state_callback_event_sender,
            })
            .unwrap();

        ChannelManager {
            tx: manager_channel.0,
        }
    }
}
