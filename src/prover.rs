//! The Client of Eigen Proof Network
//! A standalone process is needed to finish:
//! 1) Get the Latest block that not proved, check if current node need to prove it. If so, submit
//!    the proof generation request to proof network;
//! 2) Keep polling if the task is finished.
//! 3) If the task is finished, update the status into proof database, hence the extended RPC module will fetch this and return it to SDK.
