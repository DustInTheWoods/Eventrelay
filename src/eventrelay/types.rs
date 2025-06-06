use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Client Identifier Typ
pub type ClientId = u64;

/// Topic = raw Bytes
pub type Topic = Arc<Vec<u8>>;

/// Mapping: Topic → Liste von Clients
pub type Subscriptions = HashMap<Topic, HashSet<ClientId>>;
