//! Đọc lượng mưa theo giờ của các trạm Vrain (xem `contexte/vrain-api.md`). Không phụ thuộc giao diện.

mod client;
mod error;
mod parse;
mod poller;

pub use client::{Client, Credentials};
pub use error::VrainError;
pub use poller::{spawn, PollResult, Target};
pub use parse::{parse_details, vn_date, Obs, ParseOutcome};
