//! This is a module that contains the settlement logic for the Eigen network.
//! including the following settlement api:
//! 1. get_state: get the latest state of the settlement layer, including the state root and block number.
//! 2. update_state: update the state of the settlement layer with the given proof and public input.

pub(crate) mod ethereum;
