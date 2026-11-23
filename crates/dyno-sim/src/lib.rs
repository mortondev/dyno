//! Game rules for survival: inventory, crafting and mining.
//!
//! This crate has no sockets and no wall clock. Randomness comes from a seeded
//! generator, so every rule can be unit-tested and replayed.

pub mod character;
pub mod crafting;
pub mod inventory;
pub mod mining;
pub mod movement;
pub mod progression;
pub mod rng;

pub use inventory::{Inventory, SLOTS, Stack};
