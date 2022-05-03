// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    ConnectedPeer, MembershipState, NotConnectedPeer, Peer, SessionId, SetConfig, SetId, SetInfo,
    UnknownPeer,
};
use fnv::FnvHashMap;
use itertools::Itertools;
use libp2p::PeerId;
use std::{borrow::Cow, collections::HashMap};

/// State storage behind the peerset.
///
/// # Usage
///
/// This struct is nothing more but a data structure containing a list of nodes, where each node
/// is either connected to us or not.
#[derive(Debug, Clone)]
pub struct PeersState {
    /// List of nodes that we know about.
    pub(crate) nodes: HashMap<PeerId, Node>,

    /// Configuration of the set.
    pub(crate) sets: Vec<SetInfo>,

    /// Sets utilization mapped by session ids.
    pub(crate) sessions: HashMap<SessionId, usize>,
}

impl PeersState {
    /// Builds a new empty [`PeersState`].
    pub fn new(sets: impl IntoIterator<Item = SetConfig>) -> Self {
        Self {
            nodes: HashMap::default(),
            sets: sets
                .into_iter()
                .map(|config| SetInfo {
                    num_peers: 0,
                    target_size: config.target_size,
                    initial_nodes: config.boot_nodes.into_iter().collect(),
                })
                .collect(),
            sessions: HashMap::default(),
        }
    }

    /// Returns the number of sets.
    pub fn num_sets(&self) -> usize {
        self.sets.len()
    }

    /// Returns an object that grants access to the state of a peer in the context of the set.
    pub fn peer<'a>(&'a mut self, set: usize, peer_id: &'a PeerId) -> Option<Peer<'a>> {
        if self.sets.len() >= set {
            return None;
        }

        Some(match self.nodes.get(peer_id).map(|n| *n.sets[set]) {
            None | Some(MembershipState::NotMember) => Peer::Unknown(UnknownPeer {
                set,
                parent: self,
                peer_id: Cow::Borrowed(peer_id),
            }),
            Some(MembershipState::Connected) => Peer::Connected(ConnectedPeer {
                set,
                state: self,
                peer_id: Cow::Borrowed(peer_id),
            }),
            Some(MembershipState::NotConnected { .. }) => Peer::NotConnected(NotConnectedPeer {
                set,
                state: self,
                peer_id: Cow::Borrowed(peer_id),
            }),
        })
    }

    /// Returns the list of all the peers we know of.
    pub fn peer_ids(&self) -> impl ExactSizeIterator<Item = PeerId> {
        self.nodes.keys().map(|p| p.clone())
    }

    /// Returns the index of a specified peer in a given set.
    pub fn index_of(&self, peer: PeerId) -> Option<usize> {
        self.nodes
            .iter()
            .map(|(p, _)| p)
            .sorted_by_key(|p| p.to_bytes())
            .position(|elem| *elem == peer)
    }

    /// Returns the index of a specified peer in a given set.
    pub fn at_index(&self, index: usize) -> Option<PeerId> {
        let peers: Vec<&PeerId> = self
            .nodes
            .iter()
            .map(|(p, _)| p)
            .sorted_by_key(|p| p.to_bytes())
            .collect();

        if peers.len() > index {
            Some(peers[index].clone())
        } else {
            None
        }
    }

    /// Returns the list of peers we are connected to in the context of the set.
    pub fn connected_peers(&self, set: usize) -> Option<impl Iterator<Item = &PeerId>> {
        if self.sets.len() >= set {
            return None;
        }

        Some(
            self.nodes
                .iter()
                .filter(move |(p, n)| n.sets[set].is_connected())
                .map(|(p, _)| p),
        )
    }

    /// Returns peer's membership state in the set.
    pub fn peer_membership(&self, peer_id: &PeerId, set: usize) -> Option<MembershipState> {
        if !self.sets.contains_key(&session_id) {
            return None;
        }

        Some(
            self.nodes
                .iter()
                .find(move |(p, _)| p.to_bytes() == peer_id.to_bytes())
                .map(|(_, s)| *s.sets[set])
                .unwrap_or(MembershipState::NotMember),
        )
    }
}

/// State of a single node that we know about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    /// Map of sets the node to session_ids.
    pub(crate) sets: Vec<MembershipState>,
}

impl Node {
    pub(crate) fn new(num_sets: usize) -> Self {
        Self {
            sets: (0..num_sets).map(|_| MembershipState::NotMember).collect(),
        }
    }
}
