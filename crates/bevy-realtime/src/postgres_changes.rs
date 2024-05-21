pub mod bevy {
    use std::marker::PhantomData;

    use bevy::prelude::*;
    use crossbeam::channel::unbounded;

    use crate::{
        client_ready, forwarder_recv,
        message::{
            payload::{PostgresChangesEvent, PostgresChangesPayload},
            postgres_change_filter::PostgresChangeFilter,
        },
        BevyChannelBuilder, ChannelForwarder,
    };

    pub trait PostgresPayloadEvent {
        fn new(payload: PostgresChangesPayload) -> Self;
    }

    pub trait AppExtend {
        fn add_postgres_event<E: Event + PostgresPayloadEvent, F: Component>(
            &mut self,
        ) -> &mut Self;
    }

    impl AppExtend for App {
        fn add_postgres_event<E: Event + PostgresPayloadEvent, F: Component>(
            &mut self,
        ) -> &mut Self {
            self.add_event::<E>().add_systems(
                Update,
                (postgres_forward::<E, F>, forwarder_recv::<E>)
                    .chain()
                    .run_if(client_ready),
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
        pub fn new(
            event: PostgresChangesEvent,
            filter: PostgresChangeFilter,
        ) -> PostgresForwarder<E> {
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
            (Entity, &mut BevyChannelBuilder, &PostgresForwarder<E>),
            (Added<PostgresForwarder<E>>, With<T>),
        >,
    ) {
        for (e, mut cb, event) in q.iter_mut() {
            let (tx, rx) = unbounded();

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
}
