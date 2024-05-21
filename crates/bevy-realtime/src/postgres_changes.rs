pub mod bevy {
    use std::marker::PhantomData;

    use bevy::prelude::*;
    use bevy_crossbeam_event::{CrossbeamEventApp, CrossbeamEventSender};

    use crate::{
        message::{
            payload::{PostgresChangesEvent, PostgresChangesPayload},
            postgres_change_filter::PostgresChangeFilter,
        },
        BevyChannelBuilder,
    };

    pub trait PostgresPayloadEvent {
        fn new(payload: PostgresChangesPayload) -> Self;
    }

    pub trait PostresEventApp {
        fn add_postgres_event<E: Event + PostgresPayloadEvent + Clone, F: Component>(
            &mut self,
        ) -> &mut Self;
    }

    impl PostresEventApp for App {
        fn add_postgres_event<E: Event + PostgresPayloadEvent + Clone, F: Component>(
            &mut self,
        ) -> &mut Self {
            self.add_crossbeam_event::<E>()
                .add_systems(Update, (postgres_forward::<E, F>,).chain())
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

    pub fn postgres_forward<E: Event + PostgresPayloadEvent + Clone, T: Component>(
        mut commands: Commands,
        mut q: Query<
            (Entity, &mut BevyChannelBuilder, &PostgresForwarder<E>),
            (Added<PostgresForwarder<E>>, With<T>),
        >,
        sender: Res<CrossbeamEventSender<E>>,
    ) {
        for (e, mut cb, event) in q.iter_mut() {
            let s = sender.clone();
            cb.0.on_postgres_change(event.event.clone(), event.filter.clone(), move |payload| {
                let ev = E::new(payload.clone());
                s.send(ev);
            });

            commands.entity(e).remove::<PostgresForwarder<E>>();
        }
    }
}
