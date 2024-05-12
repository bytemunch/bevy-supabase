pub mod broadcast;
pub mod postgres_changes;
pub mod presence;

use bevy::prelude::*;

pub use realtime_rs::{message::*, realtime_channel::*, realtime_client::*};

use tokio::sync::mpsc::{self, Receiver};

use crate::presence::{presence_untrack, update_presence_track};

#[derive(Resource, Deref)]
pub struct Client(pub ClientManagerSync);

impl Client {
    pub fn channel(&mut self, topic: impl Into<String>) -> ChannelBuilder {
        ChannelBuilder(self.0.channel(topic))
    }
}

#[derive(Component)]
pub struct ChannelBuilder(pub RealtimeChannelBuilder);

#[derive(Component)]
pub struct ChannelForwarder<E: Event> {
    rx: Receiver<E>,
}

pub fn forwarder_recv<E: Event>(
    mut commands: Commands,
    mut q_forwarders: Query<(Entity, &mut ChannelForwarder<E>)>,
    mut evw: EventWriter<E>,
) {
    for (e, mut c) in q_forwarders.iter_mut() {
        match c.rx.try_recv() {
            Ok(ev) => {
                evw.send(ev);
            }
            Err(err) => match err {
                mpsc::error::TryRecvError::Empty => continue,
                mpsc::error::TryRecvError::Disconnected => {
                    commands.entity(e).despawn();
                }
            },
        }
    }
}

#[derive(Component)]
pub struct BuildChannel;

fn build_channels(
    mut commands: Commands,
    mut q: Query<(Entity, &mut ChannelBuilder), With<BuildChannel>>,
    client: Res<Client>,
) {
    for (e, mut c) in q.iter_mut() {
        commands.entity(e).remove::<ChannelBuilder>();
        let Ok(channel) = c.0.build_sync(&client.0) else {
            continue;
        };

        channel.subscribe();

        commands.entity(e).insert(Channel { inner: channel });
    }
}

#[derive(Component)]
pub struct Channel {
    pub inner: ChannelManagerSync,
}

pub struct RealtimePlugin {
    pub client: ClientManagerSync,
}

impl RealtimePlugin {
    pub fn new(endpoint: String, apikey: String) -> Self {
        let client = RealtimeClientBuilder::new(endpoint, apikey)
            .connect()
            .to_sync();
        Self { client }
    }
}

impl Plugin for RealtimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Client(self.client.clone()))
            .add_systems(
                Update,
                ((update_presence_track, presence_untrack, build_channels).chain(),),
            );
    }
}
