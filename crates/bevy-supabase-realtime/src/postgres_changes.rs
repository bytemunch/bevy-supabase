use std::marker::PhantomData;

use bevy::prelude::*;
pub use realtime_rs::message::presence::{PresenceEvent, PresenceState};
use realtime_rs::message::{
    payload::{PostgresChangesEvent, PostgresChangesPayload},
    PostgresChangeFilter,
};
use tokio::sync::mpsc;

use crate::{forwarder_recv, ChannelBuilder, ChannelForwarder};

pub trait PostgresPayloadEvent {
    fn new(payload: PostgresChangesPayload) -> Self;
}

pub trait AppExtend {
    fn add_postgres_event<E: Event + PostgresPayloadEvent, F: Component>(&mut self) -> &mut Self;
}

impl AppExtend for App {
    fn add_postgres_event<E: Event + PostgresPayloadEvent, F: Component>(&mut self) -> &mut Self {
        self.add_event::<E>().add_systems(
            Update,
            (postgres_forward::<E, F>, forwarder_recv::<E>).chain(),
        )
    }
}

#[derive(Component)]
pub struct PostgresForwarder<E: Event + PostgresPayloadEvent> {
    pub event: PostgresChangesEvent,
    pub filter: PostgresChangeFilter,
    spooky: PhantomData<E>,
}

impl<E: Event + PostgresPayloadEvent> PostgresForwarder<E> {
    pub fn new(event: PostgresChangesEvent, filter: PostgresChangeFilter) -> PostgresForwarder<E> {
        Self {
            event,
            filter,
            spooky: PhantomData::<E>,
        }
    }
}

// "consumes" PostgresForwarders, creates ChannelForwarders
pub fn postgres_forward<E: Event + PostgresPayloadEvent, T: Component>(
    mut commands: Commands,
    mut q: Query<
        (Entity, &mut ChannelBuilder, &PostgresForwarder<E>),
        (Added<PostgresForwarder<E>>, With<T>),
    >,
) {
    for (e, mut cb, event) in q.iter_mut() {
        let (tx, rx) = mpsc::channel(255);

        cb.0.on_postgres_change(event.event.clone(), event.filter.clone(), move |payload| {
            let ev = E::new(payload.clone());

            tx.try_send(ev).unwrap();
        });

        commands
            .entity(e)
            .insert(ChannelForwarder::<E> { rx })
            .remove::<PostgresForwarder<E>>();
    }
}
