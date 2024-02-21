use std::{collections::HashMap, marker::PhantomData};

use bevy::prelude::*;
pub use realtime_rs::message::presence::{PresenceEvent, PresenceState};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{forwarder_recv, Channel, ChannelBuilder, ChannelForwarder};

pub trait PresencePayloadEvent {
    fn new(key: String, old_state: PresenceState, new_state: PresenceState) -> Self;
}

pub trait AppExtend {
    fn add_presence_event<E: Event + PresencePayloadEvent, F: Component>(&mut self) -> &mut Self;
}

impl AppExtend for App {
    fn add_presence_event<E: Event + PresencePayloadEvent, F: Component>(&mut self) -> &mut Self {
        self.add_event::<E>().add_systems(
            Update,
            (presence_forward::<E, F>, forwarder_recv::<E>).chain(),
        )
    }
}

#[derive(Component)]
pub struct PresenceForwarder<E: Event + PresencePayloadEvent> {
    pub event: PresenceEvent,
    spooky: PhantomData<E>,
}

impl<E: Event + PresencePayloadEvent> PresenceForwarder<E> {
    pub fn new(event: PresenceEvent) -> PresenceForwarder<E> {
        Self {
            event,
            spooky: PhantomData::<E>,
        }
    }
}

// "consumes" PresenceForwarders, creates ChannelForwarders
pub fn presence_forward<E: Event + PresencePayloadEvent, T: Component>(
    mut commands: Commands,
    mut q: Query<
        (Entity, &mut ChannelBuilder, &PresenceForwarder<E>),
        (Added<PresenceForwarder<E>>, With<T>),
    >,
) {
    for (e, mut cb, event) in q.iter_mut() {
        let (tx, rx) = mpsc::channel(255);

        cb.0.on_presence(event.event.clone(), move |key, old, new| {
            let ev = E::new(key, old, new);

            tx.try_send(ev).unwrap();
        });

        commands
            .entity(e)
            .insert(ChannelForwarder::<E> { rx })
            .remove::<PresenceForwarder<E>>();
    }
}

// State tracking

#[derive(Component)]
pub struct PrescenceTrack {
    pub payload: HashMap<String, Value>,
}

pub fn update_presence_track(
    q: Query<(&PrescenceTrack, &Channel), Or<(Changed<PrescenceTrack>, Added<Channel>)>>,
) {
    for (p, c) in q.iter() {
        println!("RUNNUNG SYSTEM");
        c.inner.track(p.payload.clone()).unwrap();
    }
}

pub fn presence_untrack(q: Query<&Channel>, mut removed: RemovedComponents<PrescenceTrack>) {
    for r in removed.read() {
        if let Ok(c) = q.get(r) {
            c.inner.untrack().unwrap();
        }
    }
}
