use std::collections::HashSet;
use std::sync::Arc;
use bytes::Bytes;
use dashmap::DashMap;

/// Topic = raw Vec<u8>
pub type Topic = Vec<u8>;

/// Mapping: Topic → Liste von Clients
pub type Subscriptions = DashMap<Topic, HashSet<Bytes>>;
