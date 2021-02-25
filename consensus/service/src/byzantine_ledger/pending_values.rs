// Copyright (c) 2018-2021 The MobileCoin Foundation

use crate::tx_manager::TxManager;
use mc_transaction_core::tx::TxHash;
use std::{
    collections::{hash_map::Entry::Vacant, HashMap},
    sync::Arc,
    time::Instant,
};

/// Transactions that this node will attempt to submit to consensus.
/// Invariant: each pending transaction is well-formed.
/// Invariant: each pending transaction is valid w.r.t he current ledger.
///
/// We need to store them as a vec so we can process values
/// on a first-come first-served basis. However, we want to be able to:
/// 1) Efficiently see if we already have a given transaction and ignore duplicates
/// 2) Track how long each transaction took to externalize.
///
/// To accomplish these goals we store, in addition to the queue of pending values, a
/// map that maps a value to when we first encountered it. Note that we only store a
/// timestamp for values that were handed to us directly from a client. We skip tracking
/// processing times for relayed values since we want to track the time from when the network
/// first saw a value, and not when a specific node saw it.
pub struct PendingValues<TXM: TxManager> {
    tx_manager: Arc<TXM>,
    pending_values: Vec<TxHash>,
    pending_values_map: HashMap<TxHash, Option<Instant>>,
}

impl<TXM: TxManager> PendingValues<TXM> {
    pub fn new(tx_manager: Arc<TXM>) -> Self {
        Self {
            tx_manager,
            pending_values: Vec::new(),
            pending_values_map: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        // Invariant
        assert_eq!(self.pending_values.len(), self.pending_values_map.len());

        self.pending_values.is_empty()
    }

    pub fn len(&self) -> usize {
        // Invariant
        assert_eq!(self.pending_values.len(), self.pending_values_map.len());

        self.pending_values.len()
    }

    pub fn push(&mut self, tx_hash: TxHash, timestamp: Option<Instant>) -> bool {
        if let Vacant(entry) = self.pending_values_map.entry(tx_hash) {
            // A new transaction.
            if self.tx_manager.validate(&tx_hash).is_ok() {
                // The transaction is well-formed and valid.
                entry.insert(timestamp);
                self.pending_values.push(tx_hash);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &TxHash> {
        self.pending_values.iter()
    }

    pub fn get_timestamp_for_value(&self, tx_hash: &TxHash) -> Option<Instant> {
        self.pending_values_map.get(tx_hash).cloned().flatten()
    }

    pub fn retain<F>(&mut self, predicate: F)
    where
        F: Fn(&TxHash) -> bool,
    {
        self.pending_values_map
            .retain(|tx_hash, _| predicate(tx_hash));

        // (Help the borrow checker)
        let self_pending_values_map = &self.pending_values_map;
        self.pending_values
            .retain(|tx_hash| self_pending_values_map.contains_key(tx_hash));

        // Invariant
        assert_eq!(self.pending_values_map.len(), self.pending_values.len());
    }

    /// Clear any pending values that are no longer valid.
    pub fn clear_invalid_values(&mut self) {
        let tx_manager = self.tx_manager.clone();
        self.retain(|tx_hash| tx_manager.validate(tx_hash).is_ok());
    }
}