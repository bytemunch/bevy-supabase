use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Enum of presence event types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PresenceEvent {
    Track,
    Untrack,
    Join,
    Leave,
    Sync,
}

pub type RawPresenceState = HashMap<String, RawPresenceMetas>;

#[derive(Clone)]
pub(crate) struct PresenceCallback(
    pub Arc<dyn Fn(String, PresenceState, PresenceState) + Send + Sync>,
);
//{
//  abc123: {1: {foo: bar}, 2: {foo: baz} },
//  def456: {3: {foo: baz}, 4: {foo: bar} },
//}
//
// triple nested hashmap, fantastic. gonna need to write some helper functions for this one
pub type PresenceStateInner = HashMap<String, PhxMap>;

pub type PhxMap = HashMap<String, StateData>;

pub type StateData = HashMap<String, Value>;

/// PresenceState triple nested hashmap.
///
/// Layout:
/// HashMap<id, HashMap<phx_ref, HashMap<key, value>>>
/// { \[id\]: { \[ref\]: { \[key\]: value } } }
#[derive(Default, Clone, Debug)]
pub struct PresenceState(pub PresenceStateInner);

impl PresenceState {
    /// Returns a once flattened map of presence data:
    /// HashMap<phx_ref, <key, value>>
    pub fn get_phx_map(&self) -> PhxMap {
        let mut new_map = HashMap::new();
        for (_id, map) in self.0.clone() {
            for (phx_id, state_data) in map {
                new_map.insert(phx_id, state_data);
            }
        }
        new_map
    }
}

type PresenceIteratorItem = (String, HashMap<String, HashMap<String, Value>>);

impl FromIterator<PresenceIteratorItem> for PresenceState {
    fn from_iter<T: IntoIterator<Item = PresenceIteratorItem>>(iter: T) -> Self {
        let mut new_id_map = HashMap::new();

        for (id, id_map) in iter {
            new_id_map.insert(id, id_map);
        }

        PresenceState(new_id_map)
    }
}

/// Raw presence meta data
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RawPresenceMeta {
    pub phx_ref: String,
    #[serde(flatten)]
    pub state_data: HashMap<String, Value>,
}

/// Collection of raw presence metas
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RawPresenceMetas {
    pub metas: Vec<RawPresenceMeta>,
}

impl From<RawPresenceState> for PresenceState {
    fn from(val: RawPresenceState) -> Self {
        let mut transformed_state = PresenceState(HashMap::new());

        for (id, metas) in val {
            let mut transformed_inner = HashMap::new();

            for meta in metas.metas {
                transformed_inner.insert(meta.phx_ref, meta.state_data);
            }

            transformed_state.0.insert(id, transformed_inner);
        }

        transformed_state
    }
}

/// Internal, visibility skill issues mean still visible to crate consumer TODO
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawPresenceDiff {
    joins: RawPresenceState,
    leaves: RawPresenceState,
}

impl From<RawPresenceDiff> for PresenceDiff {
    fn from(val: RawPresenceDiff) -> Self {
        PresenceDiff {
            joins: val.joins.into(),
            leaves: val.leaves.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PresenceDiff {
    joins: PresenceState,
    leaves: PresenceState,
}

#[derive(Default)]
pub(crate) struct Presence {
    pub state: PresenceState,
    callbacks: HashMap<PresenceEvent, Vec<PresenceCallback>>,
}

impl Presence {
    pub(crate) fn from_channel_builder(
        callbacks: HashMap<PresenceEvent, Vec<PresenceCallback>>,
    ) -> Self {
        Self {
            state: PresenceState::default(),
            callbacks,
        }
    }

    pub(crate) fn sync(&mut self, new_state: PresenceState) {
        // TODO state? functional? Nah both mixed together. lol and also lmao even
        let joins: PresenceState = new_state
            .0
            .clone()
            .into_iter()
            .map(|(new_id, mut new_phx_map)| {
                new_phx_map.retain(|new_phx_ref, _new_state_data| {
                    let mut retain = true;
                    let _ = self.state.0.clone().into_values().map(|self_phx_map| {
                        if self_phx_map.contains_key(new_phx_ref) {
                            retain = false;
                        }
                    });
                    retain
                });

                (new_id, new_phx_map)
            })
            .collect();

        let leaves: PresenceState = self
            .state
            .0
            .clone()
            .into_iter()
            .map(|(current_id, mut current_phx_map)| {
                current_phx_map.retain(|current_phx_ref, _current_state_data| {
                    let mut retain = false;
                    let _ = new_state.0.clone().into_values().map(|new_phx_map| {
                        if !new_phx_map.contains_key(current_phx_ref) {
                            retain = true;
                        }
                    });
                    retain
                });

                (current_id, current_phx_map)
            })
            .collect();

        let prev_state = self.state.clone();

        self.sync_diff(PresenceDiff { joins, leaves });

        for (id, _data) in self.state.0.clone() {
            for cb in self
                .callbacks
                .get_mut(&PresenceEvent::Sync)
                .unwrap_or(&mut vec![])
            {
                cb.0(id.clone(), prev_state.clone(), self.state.clone());
            }
        }
    }

    pub(crate) fn sync_diff(&mut self, diff: PresenceDiff) -> &PresenceState {
        // mutate own state with diff
        // return new state
        // trigger diff callbacks

        for (id, _data) in diff.joins.0.clone() {
            for cb in self
                .callbacks
                .get_mut(&PresenceEvent::Join)
                .unwrap_or(&mut vec![])
            {
                cb.0(id.clone(), self.state.clone(), diff.clone().joins);
            }
        }

        for (id, _data) in diff.leaves.0.clone() {
            for cb in self
                .callbacks
                .get_mut(&PresenceEvent::Leave)
                .unwrap_or(&mut vec![])
            {
                cb.0(id.clone(), self.state.clone(), diff.clone().leaves);
            }

            self.state.0.remove(&id);
        }

        self.state.0.extend(diff.joins.0);

        &self.state
    }
}

pub mod bevy {
    use std::{collections::HashMap, marker::PhantomData};

    use bevy::prelude::*;
    use crossbeam::channel::unbounded;
    use serde_json::Value;

    use crate::{
        client_ready, forwarder_recv,
        presence::{PresenceEvent, PresenceState},
        BevyChannelBuilder, Channel, ChannelForwarder,
    };

    pub trait PresencePayloadEvent {
        fn new(key: String, old_state: PresenceState, new_state: PresenceState) -> Self;
    }

    pub trait AppExtend {
        fn add_presence_event<E: Event + PresencePayloadEvent, F: Component>(
            &mut self,
        ) -> &mut Self;
    }

    impl AppExtend for App {
        fn add_presence_event<E: Event + PresencePayloadEvent, F: Component>(
            &mut self,
        ) -> &mut Self {
            self.add_event::<E>().add_systems(
                Update,
                (presence_forward::<E, F>, forwarder_recv::<E>)
                    .chain()
                    .run_if(client_ready),
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
            (Entity, &mut BevyChannelBuilder, &PresenceForwarder<E>),
            (Added<PresenceForwarder<E>>, With<T>),
        >,
    ) {
        for (e, mut cb, event) in q.iter_mut() {
            let (tx, rx) = unbounded();

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
            c.track(p.payload.clone()).unwrap();
        }
    }

    pub fn presence_untrack(q: Query<&Channel>, mut removed: RemovedComponents<PrescenceTrack>) {
        for r in removed.read() {
            if let Ok(c) = q.get(r) {
                c.untrack().unwrap();
            }
        }
    }
}
