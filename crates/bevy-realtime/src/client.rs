#![allow(dead_code)]

use std::error::Error;
use std::fmt::{Debug, Display};
use std::thread::sleep;
use std::time::SystemTime;
use std::{
    collections::HashMap,
    io,
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use bevy::ecs::system::SystemId;
use bevy::log::{debug, info};
use bevy::prelude::*;
use bevy_crossbeam_event::CrossbeamEventSender;
use crossbeam::channel::{unbounded, Receiver, SendError, Sender, TryRecvError};
use native_tls::TlsConnector;
use tungstenite::{client, Message};
use tungstenite::{
    client::{uri_mode, IntoClientRequest},
    handshake::MidHandshake,
    http::{HeaderMap, HeaderValue, Response as HttpResponse, StatusCode, Uri},
    stream::{MaybeTlsStream, Mode, NoDelay},
    ClientHandshake, Error as TungsteniteError, HandshakeError, WebSocket as WebSocketWrapper,
};
use uuid::Uuid;

use super::channel::{ChannelState, RealtimeChannel};
use crate::message::payload::Payload;
use crate::message::realtime_message::RealtimeMessage;

use super::channel::ChannelBuilder;

pub type Response = HttpResponse<Option<Vec<u8>>>;
pub type WebSocket = WebSocketWrapper<MaybeTlsStream<TcpStream>>;

/// Connection state of [RealtimeClient]
#[derive(PartialEq, Debug, Default, Clone, Copy, Event)]
pub enum ConnectionState {
    /// Client wants to reconnect
    Reconnect,
    /// Client is mid-reconnect
    Reconnecting,
    Connecting,
    Open,
    Closing,
    #[default]
    Closed,
}

/// Error returned by [RealtimeClient::next_message()].
/// Can be WouldBlock
#[derive(PartialEq, Debug)]
pub enum NextMessageError {
    WouldBlock,
    TryRecvError(TryRecvError),
    NoChannel,
    ChannelClosed,
    ClientClosed,
    SocketError(SocketError),
    MonitorError(MonitorError),
}

impl Error for NextMessageError {}
impl Display for NextMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

/// Error returned by the internal monitor of [RealtimeClient]
#[derive(PartialEq, Debug)]
pub enum MonitorError {
    ReconnectError,
    MaxReconnects,
    WouldBlock,
    Disconnected,
}

/// Error type for internal socket related errors in [RealtimeClient]
#[derive(Debug, PartialEq)]
pub enum SocketError {
    NoSocket,
    NoRead,
    NoWrite,
    Disconnected,
    WouldBlock,
    TooManyRetries,
    HandshakeError,
}

/// Error returned by [RealtimeClient::connect()]
#[derive(Debug, PartialEq)]
pub enum ConnectError {
    BadUri,
    BadHost,
    BadAddrs,
    StreamError,
    NoDelayError,
    NonblockingError,
    HandshakeError,
    MaxRetries,
    WrongProtocol,
}

pub(crate) struct MessageChannel((Sender<RealtimeMessage>, Receiver<RealtimeMessage>));

impl Default for MessageChannel {
    fn default() -> Self {
        Self(crossbeam::channel::unbounded())
    }
}

struct MonitorChannel((Sender<MonitorSignal>, Receiver<MonitorSignal>));

impl Default for MonitorChannel {
    fn default() -> Self {
        Self(crossbeam::channel::unbounded())
    }
}

#[derive(Debug, PartialEq)]
enum MonitorSignal {
    Reconnect,
}

#[derive(Clone)]
pub struct ClientManager {
    tx: Sender<ClientManagerMessage>,
}

pub enum ClientManagerMessage {
    Channel {
        callback: SystemId<ChannelBuilder>,
    },
    AddChannel {
        channel: RealtimeChannel,
    },
    SetAccessToken {
        token: String,
    },
    ConnectionState {
        sender: CrossbeamEventSender<ConnectionState>,
    },
}

impl ClientManager {
    pub fn new(client: &Client) -> Self {
        Self {
            tx: client.manager_tx.clone(),
        }
    }

    pub fn channel(
        &self,
        callback: SystemId<ChannelBuilder>,
    ) -> Result<(), SendError<ClientManagerMessage>> {
        self.tx.send(ClientManagerMessage::Channel { callback })
    }

    pub fn add_channel(
        &self,
        channel: RealtimeChannel,
    ) -> Result<(), SendError<ClientManagerMessage>> {
        self.tx.send(ClientManagerMessage::AddChannel { channel })
    }

    pub fn set_access_token(&self, token: String) -> Result<(), SendError<ClientManagerMessage>> {
        self.tx.send(ClientManagerMessage::SetAccessToken { token })
    }

    pub fn connection_state(
        &self,
        sender: CrossbeamEventSender<ConnectionState>,
    ) -> Result<(), SendError<ClientManagerMessage>> {
        self.tx
            .send(ClientManagerMessage::ConnectionState { sender })
    }
}

/// Synchronous websocket client that interfaces with Supabase Realtime
pub struct Client {
    pub(crate) access_token: String,
    connection_state: ConnectionState,
    socket: Option<WebSocket>,
    channels: HashMap<Uuid, RealtimeChannel>,
    messages_this_second: Vec<SystemTime>,
    next_ref: Uuid,
    // mpsc
    pub(crate) outbound_channel: MessageChannel,
    inbound_channel: MessageChannel,
    monitor_channel: MonitorChannel,
    middleware: HashMap<Uuid, Box<dyn Fn(RealtimeMessage) -> RealtimeMessage + Send + Sync>>,
    // timers
    reconnect_now: Option<SystemTime>,
    reconnect_delay: Duration,
    reconnect_attempts: usize,
    heartbeat_now: Option<SystemTime>,
    // builder options
    headers: HeaderMap,
    params: Option<HashMap<String, String>>,
    heartbeat_interval: Duration,
    encode: Option<Box<dyn Fn(RealtimeMessage) -> RealtimeMessage + Send + Sync>>,
    decode: Option<Box<dyn Fn(RealtimeMessage) -> RealtimeMessage + Send + Sync>>,
    reconnect_interval: ReconnectFn,
    reconnect_max_attempts: usize,
    connection_timeout: Duration,
    auth_url: Option<String>,
    endpoint: String,
    max_events_per_second: usize,
    // sync bridge
    manager_rx: Receiver<ClientManagerMessage>,
    manager_tx: Sender<ClientManagerMessage>,
    channel_callback_event_sender: CrossbeamEventSender<ChannelCallbackEvent>,
}

#[derive(Event, Clone)]
pub struct ChannelCallbackEvent(pub (SystemId<ChannelBuilder>, ChannelBuilder));

impl Client {
    fn manager_recv(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(message) = self.manager_rx.try_recv() {
            match message {
                ClientManagerMessage::Channel { callback } => {
                    let c = self.channel();

                    self.channel_callback_event_sender
                        .send(ChannelCallbackEvent((callback, c)));
                }
                ClientManagerMessage::AddChannel { channel } => self.add_channel(channel),
                ClientManagerMessage::SetAccessToken { token } => {
                    self.access_token = token;
                }
                ClientManagerMessage::ConnectionState { sender } => {
                    sender.send(self.connection_state);
                }
            }
        }

        Ok(())
    }
    /// Returns a new [RealtimeClientBuilder] with provided `endpoint` and `access_token`
    pub fn builder(endpoint: impl Into<String>, access_token: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(endpoint, access_token)
    }

    /// Returns this client's [ConnectionState]
    pub fn get_status(&self) -> ConnectionState {
        self.connection_state
    }

    /// Returns a new [RealtimeChannelBuilder] instantiated with the provided `topic`
    pub fn channel(&mut self) -> ChannelBuilder {
        ChannelBuilder::new(self)
    }

    /// Attempt to create a websocket connection with the server
    pub fn connect(&mut self) -> Result<&mut Client, ConnectError> {
        info!("connecting...");

        let uri: Uri = match format!(
            "{}/websocket?apikey={}&vsn=1.0.0",
            self.endpoint, self.access_token
        )
        .parse()
        {
            Ok(uri) => uri,
            Err(_e) => return Err(ConnectError::BadUri),
        };

        // TODO REFAC tidy
        let ws_scheme = match uri.scheme_str() {
            Some(scheme) => {
                if scheme == "http" {
                    "ws"
                } else {
                    "wss"
                }
            }
            None => "ws",
        };

        let mut add_params = String::new();
        if let Some(params) = &self.params {
            for (field, value) in params {
                add_params = format!("{add_params}&{field}={value}");
            }
        }

        let mut p_q = uri.path_and_query().unwrap().to_string();

        if !add_params.is_empty() {
            p_q = format!("{p_q}{add_params}");
        }

        let uri = Uri::builder()
            .scheme(ws_scheme)
            .authority(uri.authority().unwrap().clone())
            .path_and_query(p_q)
            .build()
            .unwrap();

        let Ok(mut request) = uri.clone().into_client_request() else {
            return Err(ConnectError::BadUri);
        };

        let headers = request.headers_mut();

        let auth: HeaderValue = format!("Bearer {}", self.access_token)
            .parse()
            .expect("malformed access token?");
        headers.insert("Authorization", auth);

        // unwrap: shouldn't fail
        let xci: HeaderValue = "realtime-rs/0.1.0".to_string().parse().unwrap();
        headers.insert("X-Client-Info", xci);

        debug!("Connecting... Req: {:?}\n", request);

        let uri = request.uri();

        let Ok(mode) = uri_mode(uri) else {
            return Err(ConnectError::BadUri);
        };

        let Some(host) = uri.host() else {
            return Err(ConnectError::BadHost);
        };

        let port = uri.port_u16().unwrap_or(match mode {
            Mode::Plain => 80,
            Mode::Tls => 443,
        });

        let Ok(mut addrs) = (host, port).to_socket_addrs() else {
            return Err(ConnectError::BadAddrs);
        };

        let mut stream = match TcpStream::connect_timeout(
            &addrs.next().expect("uhoh no addr"),
            self.connection_timeout,
        ) {
            Ok(stream) => {
                self.reconnect_attempts = 0;
                stream
            }
            Err(_e) => {
                if self.reconnect_attempts < self.reconnect_max_attempts {
                    self.reconnect_attempts += 1;
                    let backoff = self.reconnect_interval.0.as_ref();
                    sleep(backoff(self.reconnect_attempts));
                    return self.connect();
                }
                // TODO get reason from stream error
                return Err(ConnectError::StreamError);
            }
        };

        let Ok(()) = NoDelay::set_nodelay(&mut stream, true) else {
            return Err(ConnectError::NoDelayError);
        };

        let maybe_tls = match mode {
            Mode::Tls => {
                let connector = TlsConnector::new().expect("No TLS tings");

                let connected_stream = connector
                    .connect(host, stream.try_clone().expect("noclone"))
                    .unwrap();

                stream
                    .set_nonblocking(true)
                    .expect("blocking mode oh nooooo");

                MaybeTlsStream::NativeTls(connected_stream)
            }
            Mode::Plain => {
                stream
                    .set_nonblocking(true)
                    .expect("blocking mode oh nooooo");

                MaybeTlsStream::Plain(stream)
            }
        };

        let conn: Result<(WebSocket, Response), TungsteniteError> = match client(request, maybe_tls)
        {
            Ok(stream) => {
                self.reconnect_attempts = 0;
                Ok(stream)
            }
            Err(err) => match err {
                HandshakeError::Failure(_err) => {
                    // TODO DRY break reconnect attempts code into own fn
                    if self.reconnect_attempts < self.reconnect_max_attempts {
                        self.reconnect_attempts += 1;
                        let backoff = &self.reconnect_interval.0;
                        sleep(backoff(self.reconnect_attempts));
                        return self.connect();
                    }

                    return Err(ConnectError::HandshakeError);
                }
                HandshakeError::Interrupted(mid_hs) => match self.retry_handshake(mid_hs) {
                    Ok(stream) => Ok(stream),
                    Err(_err) => {
                        if self.reconnect_attempts < self.reconnect_max_attempts {
                            self.reconnect_attempts += 1;
                            let backoff = &self.reconnect_interval.0;
                            sleep(backoff(self.reconnect_attempts));
                            return self.connect();
                        }

                        // TODO err data
                        return Err(ConnectError::MaxRetries);
                    }
                },
            },
        };

        let (socket, res) = conn.expect("Handshake fail");

        if res.status() != StatusCode::SWITCHING_PROTOCOLS {
            return Err(ConnectError::WrongProtocol);
        }

        self.socket = Some(socket);

        self.connection_state = ConnectionState::Open;
        info!("connected");

        Ok(self)
    }

    /// Disconnect the client
    pub fn disconnect(&mut self) {
        if self.connection_state == ConnectionState::Closed {
            return;
        }

        self.remove_all_channels();

        self.connection_state = ConnectionState::Closed;

        let Some(ref mut socket) = self.socket else {
            debug!("Already disconnected. {:?}", self.connection_state);
            return;
        };

        let _ = socket.close(None);
        debug!("Client disconnected. {:?}", self.connection_state);
    }

    /// Queues a [RealtimeMessage] for sending to the server
    pub fn send(&mut self, msg: RealtimeMessage) -> Result<(), SendError<RealtimeMessage>> {
        self.outbound_channel.0 .0.send(msg)
    }

    /// Returns an optional mutable reference to the [RealtimeChannel] with the provided [Uuid].
    /// If `channel_id` is not found returns [None]
    pub fn get_channel_mut(&mut self, channel_id: Uuid) -> Option<&mut RealtimeChannel> {
        self.channels.get_mut(&channel_id)
    }

    /// Returns an optional reference to the [RealtimeChannel] with the provided [Uuid].
    /// If `channel_id` is not found returns [None]
    pub fn get_channel(&self, channel_id: Uuid) -> Option<&RealtimeChannel> {
        self.channels.get(&channel_id)
    }

    /// Returns a reference to this client's HashMap of channels
    pub fn get_channels(&self) -> &HashMap<Uuid, RealtimeChannel> {
        &self.channels
    }

    /// Returns [Some(RealtimeChannel)] if channel was successfully removed, [None] if the channel
    /// was not found.
    pub fn remove_channel(&mut self, channel_id: Uuid) -> Option<RealtimeChannel> {
        if let Some(mut channel) = self.channels.remove(&channel_id) {
            let _ = channel.unsubscribe();

            if self.channels.is_empty() {
                self.disconnect();
            }

            return Some(channel);
        }

        None
    }

    /// Blocks the current thread until the channel with the provided `channel_id` has subscribed.
    pub fn block_until_subscribed(&mut self, channel_id: Uuid) -> Result<Uuid, ChannelState> {
        let channel = self.channels.get_mut(&channel_id);

        let channel = channel.unwrap();

        if channel.connection_state == ChannelState::Joined {
            return Ok(channel.id);
        }

        if channel.connection_state != ChannelState::Joining {
            self.channels
                .get_mut(&channel_id)
                .unwrap()
                .subscribe()
                .unwrap();
        }

        loop {
            match self.next_message() {
                Ok(channel_ids) => {
                    debug!(
                        "[Blocking Subscribe] Message forwarded to {:?}",
                        channel_ids
                    )
                }
                Err(NextMessageError::WouldBlock) => {}
                Err(_e) => {
                    //println!("NextMessageError: {:?}", e)
                }
            }

            let channel = self.channels.get_mut(&channel_id).unwrap();

            match channel.connection_state {
                ChannelState::Joined => {
                    break;
                }
                ChannelState::Closed => return Err(ChannelState::Closed),
                _ => {}
            }
        }

        Ok(channel_id)
    }

    /// Use provided JWT to authorize future requests from this client and all channels
    pub fn set_auth(&mut self, access_token: String) {
        self.access_token = access_token.clone();

        for channel in self.channels.values_mut() {
            // TODO single source of data for access token
            let _ = channel.set_auth(access_token.clone()); // TODO error handling
        }
    }

    /// Add a callback to run mutably on recieved [RealtimeMessage]s before any other registered
    /// callbacks.
    pub fn add_middleware(
        &mut self,
        middleware: Box<dyn Fn(RealtimeMessage) -> RealtimeMessage + Send + Sync>,
    ) -> Uuid {
        // TODO user defined middleware ordering
        let uuid = Uuid::new_v4();
        self.middleware.insert(uuid, middleware);
        uuid
    }

    /// Remove middleware by it's [Uuid]
    pub fn remove_middleware(&mut self, uuid: Uuid) -> &mut Client {
        self.middleware.remove(&uuid);
        self
    }

    /// The main step function for driving the [RealtimeClient]
    pub fn next_message(&mut self) -> Result<Vec<Uuid>, NextMessageError> {
        match self.manager_recv() {
            Ok(()) => {}
            Err(e) => debug!("client manager_recv error: {}", e),
        }

        for (_id, channel) in &mut self.channels {
            match channel.manager_recv() {
                Ok(()) => {}
                Err(e) => debug!("channel manager_recv error: {}", e),
            }
        }

        match self.connection_state {
            ConnectionState::Closed => {
                return Err(NextMessageError::ClientClosed);
            }
            ConnectionState::Reconnect => {
                let _ = self.monitor_channel.0 .0.send(MonitorSignal::Reconnect);
                return Err(NextMessageError::SocketError(SocketError::Disconnected));
            }
            ConnectionState::Reconnecting => {
                return Err(NextMessageError::WouldBlock);
            }
            ConnectionState::Connecting => {}
            ConnectionState::Closing => {}
            ConnectionState::Open => {}
        }

        match self.run_monitor() {
            Ok(_) => {}
            Err(MonitorError::WouldBlock) => {}
            Err(MonitorError::MaxReconnects) => {
                self.disconnect();
                return Err(NextMessageError::MonitorError(MonitorError::MaxReconnects));
            }
            Err(e) => {
                return Err(NextMessageError::MonitorError(e));
            }
        }

        self.run_heartbeat();

        match self.write_socket() {
            Ok(()) => {}
            Err(SocketError::WouldBlock) => {}
            Err(e) => {
                self.reconnect();
                return Err(NextMessageError::SocketError(e));
            }
        }

        match self.read_socket() {
            Ok(()) => {}
            Err(e) => {
                self.reconnect();
                return Err(NextMessageError::SocketError(e));
            }
        }

        match self.inbound_channel.0 .1.try_recv() {
            Ok(mut message) => {
                let mut ids = vec![];
                // TODO filter & route system messages and the like

                // Run middleware
                message = self.run_middleware(message);

                // Send message to channel
                for (id, channel) in &mut self.channels {
                    if channel.topic == message.topic {
                        channel.recieve(message.clone());
                        ids.push(*id);
                    }
                }

                Ok(ids)
            }
            Err(TryRecvError::Empty) => Err(NextMessageError::WouldBlock),
            Err(e) => Err(NextMessageError::TryRecvError(e)),
        }
    }

    pub(crate) fn add_channel(&mut self, channel: RealtimeChannel) {
        self.channels.insert(channel.id, channel);
    }

    pub(crate) fn get_channel_tx(&self) -> Sender<RealtimeMessage> {
        self.outbound_channel.0 .0.clone()
    }

    fn run_middleware(&self, mut message: RealtimeMessage) -> RealtimeMessage {
        for middleware in self.middleware.values() {
            message = middleware(message)
        }
        message
    }

    fn remove_all_channels(&mut self) {
        if self.connection_state == ConnectionState::Closing
            || self.connection_state == ConnectionState::Closed
        {
            return;
        }

        self.connection_state = ConnectionState::Closing;

        // wait until inbound_rx is drained
        loop {
            let recv = self.next_message();

            if Err(NextMessageError::WouldBlock) == recv {
                break;
            }
        }

        loop {
            let _ = self.next_message();

            let mut all_channels_closed = true;

            for channel in self.channels.values_mut() {
                let channel_state = channel.unsubscribe();

                match channel_state {
                    Ok(state) => {
                        if state != ChannelState::Closed {
                            all_channels_closed = false;
                        }
                    }
                    Err(e) => {
                        // TODO error handling
                        debug!("Unsubscribe error: {:?}", e);
                    }
                }
            }

            if all_channels_closed {
                debug!("All channels closed!");
                break;
            }
        }

        self.channels.clear();
    }

    fn retry_handshake(
        &mut self,
        mid_hs: MidHandshake<ClientHandshake<MaybeTlsStream<TcpStream>>>,
    ) -> Result<(WebSocket, Response), SocketError> {
        match mid_hs.handshake() {
            Ok(stream) => Ok(stream),
            Err(e) => match e {
                HandshakeError::Interrupted(mid_hs) => {
                    if self.reconnect_attempts < self.reconnect_max_attempts {
                        self.reconnect_attempts += 1;
                        let backoff = &self.reconnect_interval.0;
                        sleep(backoff(self.reconnect_attempts));
                        return self.retry_handshake(mid_hs);
                    }

                    Err(SocketError::TooManyRetries)
                }
                HandshakeError::Failure(_err) => {
                    // TODO pass error data
                    Err(SocketError::HandshakeError)
                }
            },
        }
    }

    fn run_monitor(&mut self) -> Result<(), MonitorError> {
        if self.reconnect_now.is_none() {
            self.reconnect_now = Some(SystemTime::now());

            self.reconnect_delay = self.reconnect_interval.0(self.reconnect_attempts);
        }

        match self.monitor_channel.0 .1.try_recv() {
            Ok(signal) => match signal {
                MonitorSignal::Reconnect => {
                    if self.connection_state == ConnectionState::Open
                        || self.connection_state == ConnectionState::Reconnecting
                        || SystemTime::now() < self.reconnect_now.unwrap() + self.reconnect_delay
                    {
                        return Err(MonitorError::WouldBlock);
                    }

                    if self.reconnect_attempts >= self.reconnect_max_attempts {
                        return Err(MonitorError::MaxReconnects);
                    }

                    self.connection_state = ConnectionState::Reconnecting;
                    self.reconnect_attempts += 1;
                    self.reconnect_now.take();

                    match self.connect() {
                        Ok(_) => {
                            for channel in self.channels.values_mut() {
                                channel.subscribe().unwrap();
                            }

                            Ok(())
                        }
                        Err(e) => {
                            debug!("reconnect error: {:?}", e);
                            self.connection_state = ConnectionState::Reconnect;
                            Err(MonitorError::ReconnectError)
                        }
                    }
                }
            },
            Err(TryRecvError::Empty) => Err(MonitorError::WouldBlock),
            Err(TryRecvError::Disconnected) => Err(MonitorError::Disconnected),
        }
    }

    fn run_heartbeat(&mut self) {
        if self.heartbeat_now.is_none() {
            self.heartbeat_now = Some(SystemTime::now());
        }

        if self.heartbeat_now.unwrap() + self.heartbeat_interval > SystemTime::now() {
            return;
        }

        self.heartbeat_now.take();

        let _ = self.send(RealtimeMessage::heartbeat());
    }

    fn read_socket(&mut self) -> Result<(), SocketError> {
        let Some(ref mut socket) = self.socket else {
            return Err(SocketError::NoSocket);
        };

        if !socket.can_read() {
            return Err(SocketError::NoRead);
        }

        match socket.read() {
            Ok(raw_message) => match raw_message {
                Message::Text(string_message) => {
                    // TODO recoverable error
                    let mut message: RealtimeMessage =
                        serde_json::from_str(&string_message).expect("Deserialization error: ");

                    debug!("[RECV] {:?}", message);

                    if let Some(decode) = &self.decode {
                        message = decode(message);
                    }

                    if let Payload::Empty {} = message.payload {
                        debug!("Possibly malformed payload: {:?}", string_message)
                    }

                    let _ = self.inbound_channel.0 .0.send(message);
                    Ok(())
                }
                Message::Close(_close_message) => {
                    self.disconnect();
                    Err(SocketError::Disconnected)
                }
                _ => {
                    // do nothing on ping, pong, binary messages
                    Err(SocketError::WouldBlock)
                }
            },
            Err(TungsteniteError::Io(err)) if err.kind() == io::ErrorKind::WouldBlock => {
                // do nothing here :)
                Ok(())
            }
            Err(err) => {
                debug!("Socket read error: {:?}", err);
                self.connection_state = ConnectionState::Reconnect;
                let _ = self.monitor_channel.0 .0.send(MonitorSignal::Reconnect);
                Err(SocketError::WouldBlock)
            }
        }
    }

    fn write_socket(&mut self) -> Result<(), SocketError> {
        let Some(ref mut socket) = self.socket else {
            return Err(SocketError::NoSocket);
        };

        if !socket.can_write() {
            return Err(SocketError::NoWrite);
        }

        // Throttling
        let now = SystemTime::now();

        self.messages_this_second = self
            .messages_this_second
            .clone()
            .into_iter()
            .filter(|st| now.duration_since(*st).unwrap_or_default() < Duration::from_secs(1))
            .collect();

        if self.messages_this_second.len() >= self.max_events_per_second {
            return Err(SocketError::WouldBlock);
        }

        // Send to server
        // TODO should drain outbound_channel
        // TODO drain should respect throttling
        let message = self.outbound_channel.0 .1.try_recv();

        // So should be a while try recv loop

        match message {
            Ok(mut message) => {
                if message.message_ref.is_none() {
                    message.message_ref = Some(self.next_ref.into());
                    self.next_ref = Uuid::new_v4();
                }

                if let Some(encode) = &self.encode {
                    message = encode(message);
                }

                let raw = serde_json::to_string(&message);
                debug!("[SEND] {:?}", raw);

                let _ = socket.send(message.into());
                self.messages_this_second.push(now);
                Ok(())
            }
            Err(TryRecvError::Empty) => {
                // do nothing
                Ok(())
            }
            Err(e) => {
                debug!("outbound error: {:?}", e);
                self.connection_state = ConnectionState::Reconnect;
                let _ = self.monitor_channel.0 .0.send(MonitorSignal::Reconnect);
                Err(SocketError::WouldBlock)
            }
        }
    }

    fn reconnect(&mut self) {
        self.connection_state = ConnectionState::Reconnect;
        let _ = self.monitor_channel.0 .0.send(MonitorSignal::Reconnect);
    }
}

/// Takes a `Box<dyn Fn(usize) -> Duration>`
/// The provided function should take a count of reconnect attempts and return a [Duration] to wait
/// until the next attempt is made.
pub struct ReconnectFn(pub Box<dyn Fn(usize) -> Duration + Send + Sync>);

impl ReconnectFn {
    pub fn new(f: impl Fn(usize) -> Duration + 'static + Sync + Send) -> Self {
        Self(Box::new(f))
    }
}

impl Default for ReconnectFn {
    fn default() -> Self {
        Self(Box::new(backoff))
    }
}

/// Builder struct for [RealtimeClient]
pub struct ClientBuilder {
    headers: HeaderMap,
    params: Option<HashMap<String, String>>,
    heartbeat_interval: Duration,
    encode: Option<Box<dyn Fn(RealtimeMessage) -> RealtimeMessage + Send + Sync>>,
    decode: Option<Box<dyn Fn(RealtimeMessage) -> RealtimeMessage + Send + Sync>>,
    reconnect_interval: ReconnectFn,
    reconnect_max_attempts: usize,
    connection_timeout: Duration,
    auth_url: Option<String>,
    endpoint: String,
    access_token: String,
    max_events_per_second: usize,
}

impl ClientBuilder {
    pub fn new(endpoint: impl Into<String>, access_token: impl Into<String>) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("X-Client-Info", "realtime-rs/0.1.0".parse().unwrap());

        Self {
            headers,
            params: Default::default(),
            heartbeat_interval: Duration::from_secs(29),
            encode: Default::default(),
            decode: Default::default(),
            reconnect_interval: ReconnectFn(Box::new(backoff)),
            reconnect_max_attempts: usize::MAX,
            connection_timeout: Duration::from_secs(10),
            auth_url: Default::default(),
            endpoint: endpoint.into(),
            access_token: access_token.into(),
            max_events_per_second: 10,
        }
    }

    /// Sets the client headers. Headers always contain "X-Client-Info: realtime-rs/{version}".
    pub fn set_headers(&mut self, set_headers: HeaderMap) -> &mut Self {
        let mut headers = HeaderMap::new();
        headers.insert("X-Client-Info", "realtime-rs/0.1.0".parse().unwrap());
        headers.extend(set_headers);

        self.headers = headers;

        self
    }

    /// Merges provided [HeaderMap] with currently held headers
    pub fn add_headers(&mut self, headers: HeaderMap) -> &mut Self {
        self.headers.extend(headers);
        self
    }

    /// Set endpoint URL params
    pub fn params(&mut self, params: HashMap<String, String>) -> &mut Self {
        self.params = Some(params);
        self
    }

    /// Set [Duration] between heartbeat packets. Default 29 seconds.
    pub fn heartbeat_interval(&mut self, heartbeat_interval: Duration) -> &mut Self {
        self.heartbeat_interval = heartbeat_interval;
        self
    }

    /// Set the function to provide time between reconnection attempts
    /// The provided function should take a count of reconnect attempts and return a [Duration] to wait
    /// until the next attempt is made.
    ///
    /// Don't implement an untested timing function here in prod or you might make a few too many
    /// requests.
    ///
    /// Defaults to stepped backoff
    pub fn reconnect_interval(&mut self, reconnect_interval: ReconnectFn) -> &mut Self {
        // TODO minimum interval to prevent 10000000 requests in seconds
        // then again it takes a bit of work to make that mistake?
        self.reconnect_interval = reconnect_interval;
        self
    }

    /// Configure the number of recconect attempts to be made before erroring
    pub fn reconnect_max_attempts(&mut self, max_attempts: usize) -> &mut Self {
        self.reconnect_max_attempts = max_attempts;
        self
    }

    /// Configure the duration to wait for a connection to succeed.
    /// Default: 10 seconds
    /// Minimum: 1 second
    pub fn connection_timeout(&mut self, timeout: Duration) -> &mut Self {
        // 1 sec min timeout
        let timeout = if timeout < Duration::from_secs(1) {
            Duration::from_secs(1)
        } else {
            timeout
        };

        self.connection_timeout = timeout;
        self
    }

    /// Set the base URL for the auth server
    /// In live supabase deployments this is the same as the endpoint URL, and defaults as such.
    /// In local deployments this may need to be set manually
    pub fn auth_url(&mut self, auth_url: impl Into<String>) -> &mut Self {
        self.auth_url = Some(auth_url.into());
        self
    }

    /// Sets the max messages we can send in a second.
    /// Default: 10
    pub fn max_events_per_second(&mut self, count: usize) -> &mut Self {
        self.max_events_per_second = count;
        self
    }

    pub fn encode(
        &mut self,
        encode: impl Fn(RealtimeMessage) -> RealtimeMessage + 'static + Send + Sync,
    ) -> &mut Self {
        self.encode = Some(Box::new(encode));
        self
    }

    pub fn decode(
        &mut self,
        decode: impl Fn(RealtimeMessage) -> RealtimeMessage + 'static + Send + Sync,
    ) -> &mut Self {
        self.decode = Some(Box::new(decode));
        self
    }

    /// Consume the [Self] and return a configured [RealtimeClient]
    pub fn build(
        self,
        channel_callback_event_sender: CrossbeamEventSender<ChannelCallbackEvent>,
    ) -> Client {
        let (manager_tx, manager_rx) = unbounded();
        let c = Client {
            headers: self.headers,
            params: self.params,
            heartbeat_interval: self.heartbeat_interval,
            encode: self.encode,
            decode: self.decode,
            reconnect_interval: self.reconnect_interval,
            reconnect_max_attempts: self.reconnect_max_attempts,
            connection_timeout: self.connection_timeout,
            auth_url: self.auth_url,
            endpoint: self.endpoint,
            access_token: self.access_token,
            max_events_per_second: self.max_events_per_second,
            next_ref: Uuid::new_v4(),
            connection_state: Default::default(),
            socket: Default::default(),
            channels: Default::default(),
            messages_this_second: Default::default(),
            outbound_channel: Default::default(),
            inbound_channel: Default::default(),
            monitor_channel: Default::default(),
            middleware: Default::default(),
            reconnect_now: Default::default(),
            reconnect_delay: Default::default(),
            reconnect_attempts: Default::default(),
            heartbeat_now: Default::default(),
            manager_rx,
            manager_tx,
            channel_callback_event_sender,
        };

        c
    }
}

fn backoff(attempts: usize) -> Duration {
    let times: Vec<u64> = vec![0, 1, 2, 5, 10];

    Duration::from_secs(times[attempts.min(times.len() - 1)])
}
