pub mod bevy {
    use std::{collections::HashMap, marker::PhantomData};

    use bevy::prelude::*;
    use crossbeam::channel::unbounded;
    use serde_json::Value;

    use crate::{forwarder_recv, BevyChannelBuilder, ChannelForwarder};

    pub trait AppExtend {
        fn add_broadcast_event<E: Event + BroadcastPayloadEvent, F: Component>(
            &mut self,
        ) -> &mut Self;
    }

    impl AppExtend for App {
        fn add_broadcast_event<E: Event + BroadcastPayloadEvent, F: Component>(
            &mut self,
        ) -> &mut Self {
            self.add_event::<E>().add_systems(
                Update,
                (broadcast_forward::<E, F>, forwarder_recv::<E>).chain(),
            )
        }
    }

    pub trait BroadcastPayloadEvent {
        fn new(payload: HashMap<String, Value>) -> Self;
    }

    #[derive(Component, Default)]
    pub struct BroadcastForwarder<E: Event + BroadcastPayloadEvent> {
        pub event: String,
        spooky: PhantomData<E>,
    }

    impl<E: Event + BroadcastPayloadEvent> BroadcastForwarder<E> {
        pub fn new(topic: String) -> BroadcastForwarder<E> {
            Self {
                event: topic,
                spooky: PhantomData::<E>,
            }
        }
    }

    // "consumes" BroadcastForwarders, creates ChannelForwarders
    pub fn broadcast_forward<E: Event + BroadcastPayloadEvent, T: Component>(
        mut commands: Commands,
        mut q: Query<
            (Entity, &mut BevyChannelBuilder, &BroadcastForwarder<E>),
            (Added<BroadcastForwarder<E>>, With<T>),
        >,
    ) {
        for (e, mut cb, event) in q.iter_mut() {
            let (tx, rx) = unbounded();

            cb.0.on_broadcast(event.event.clone(), move |payload| {
                let ev = E::new(payload.clone());

                tx.send(ev).unwrap();
            });

            commands.spawn(ChannelForwarder::<E> { rx });
            commands.entity(e).remove::<BroadcastForwarder<E>>();
        }
    }
}
