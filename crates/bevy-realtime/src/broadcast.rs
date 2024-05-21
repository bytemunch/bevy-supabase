pub mod bevy {
    use std::{collections::HashMap, marker::PhantomData};

    use bevy::prelude::*;
    use bevy_crossbeam_event::{CrossbeamEventApp, CrossbeamEventSender};
    use serde_json::Value;

    use crate::BevyChannelBuilder;

    pub trait BroadcastEventApp {
        fn add_broadcast_event<E: Event + BroadcastPayloadEvent + Clone, F: Component>(
            &mut self,
        ) -> &mut Self;
    }

    impl BroadcastEventApp for App {
        fn add_broadcast_event<E: Event + BroadcastPayloadEvent + Clone, F: Component>(
            &mut self,
        ) -> &mut Self {
            self.add_crossbeam_event::<E>()
                .add_systems(Update, (broadcast_forward::<E, F>,))
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

    pub fn broadcast_forward<E: Event + BroadcastPayloadEvent + Clone, T: Component>(
        mut commands: Commands,
        mut q: Query<
            (Entity, &mut BevyChannelBuilder, &BroadcastForwarder<E>),
            (Added<BroadcastForwarder<E>>, With<T>),
        >,
        sender: Res<CrossbeamEventSender<E>>,
    ) {
        for (e, mut cb, event) in q.iter_mut() {
            let s = sender.clone();

            cb.0.on_broadcast(event.event.clone(), move |payload| {
                let ev = E::new(payload.clone());
                s.send(ev);
            });

            commands.entity(e).remove::<BroadcastForwarder<E>>();
        }
    }
}
