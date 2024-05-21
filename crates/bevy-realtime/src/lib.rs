pub mod broadcast;
pub mod channel;
pub mod client;
pub mod message;
pub mod postgres_changes;
pub mod presence;

use std::{thread::sleep, time::Duration};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_crossbeam_event::{CrossbeamEventApp, CrossbeamEventSender};
use channel::{ChannelBuilder, ChannelManager};
use client::{ClientBuilder, ClientManager, ConnectionState, NextMessageError};

use crate::presence::bevy::{presence_untrack, update_presence_track};

#[derive(Resource, Deref)]
pub struct Client(pub ClientManager);

#[derive(Component, Deref, DerefMut)]
pub struct BevyChannelBuilder(pub ChannelBuilder);

#[derive(Component, Deref, DerefMut)]
pub struct Channel(pub ChannelManager);

#[derive(Component)]
pub struct BuildChannel;

fn build_channels(
    mut commands: Commands,
    mut q: Query<(Entity, &mut BevyChannelBuilder), With<BuildChannel>>,
    mut client: ResMut<Client>,
) {
    for (e, c) in q.iter_mut() {
        commands.entity(e).remove::<BevyChannelBuilder>();

        let channel = c.build(&mut client.0);

        channel.subscribe().unwrap();
        commands.entity(e).insert(Channel(channel));
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct ClientTask(Task<()>);

pub struct RealtimePlugin {
    endpoint: String,
    apikey: String,
}
#[derive(Resource)]
pub struct RealtimeConfig {
    endpoint: String,
    apikey: String,
}

impl RealtimePlugin {
    pub fn new(endpoint: String, apikey: String) -> Self {
        Self { endpoint, apikey }
    }
}

fn setup(mut commands: Commands, config: Res<RealtimeConfig>) {
    let pool = AsyncComputeTaskPool::get();

    let endpoint = config.endpoint.clone();
    let apikey = config.apikey.clone();
    let mut client = ClientBuilder::new(endpoint, apikey).build();

    commands.insert_resource(Client(ClientManager::new(&client)));

    let task = pool.spawn(async move {
        client.connect().unwrap();
        loop {
            match client.next_message() {
                Err(NextMessageError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => println!("{}", e),
            }

            // TODO find a sane sleep value
            sleep(Duration::from_secs_f32(f32::MIN_POSITIVE));
        }
    });

    commands.insert_resource(ClientTask(task));
}

impl Plugin for RealtimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RealtimeConfig {
            apikey: self.apikey.clone(),
            endpoint: self.endpoint.clone(),
        })
        .add_crossbeam_event::<ConnectionState>()
        .add_systems(PreStartup, (setup,))
        .add_systems(
            Update,
            ((
                //
                update_presence_track,
                presence_untrack,
                build_channels,
            )
                .chain()
                .run_if(client_ready),),
        );
    }
}

pub fn client_ready(
    mut evr: EventReader<ConnectionState>,
    mut last_state: Local<ConnectionState>,
    mut rate_limiter: Local<usize>,
    client: Res<Client>,
    sender: Res<CrossbeamEventSender<ConnectionState>>,
) -> bool {
    *rate_limiter += 1;
    if *rate_limiter % 30 == 0 {
        *rate_limiter = 0;
        client.connection_state(sender.clone()).unwrap_or(());
    }

    for ev in evr.read() {
        *last_state = ev.clone();
    }

    *last_state == ConnectionState::Open
}
