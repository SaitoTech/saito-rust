use serde::{Serialize, Deserialize};
use crate::block::BlockHeader;

pub const HEARTBEAT: u64 = 30_000;

// #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
/// The Burnfee object which contains our starting value and current value when the fee was paid
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BurnFee {
    pub start: f32,
    pub current: u64,
}

impl BurnFee {
    /// Returns the BurnFee used to calculate the work needed to produce a block
    ///
    /// * `start` - y-value at x = 0
    /// * `current` - y-value at x = 0 for next bloc
    pub fn new(start: f32, current: u64) -> Self {
        return BurnFee {
	      start,
	      current
	    };
    }

    /// Returns the amount of work needed to produce a block given the timestamp of
    /// the previous block, the current timestamp, and the y-axis of the burn fee
    /// curve. This is used both in the creation of blocks (mempool) as well as
    /// during block validation.
    ///
    /// * `prevts` - timestamp of previous block
    /// * `ts`     - candidate timestamp
    /// * `start`  - burn fee value (y-axis) for curve determination ("start")
    ///
    pub fn return_work_needed(prevts: u64, ts: u64, start: f32) -> u64 {

	    let mut elapsed_time = ts - prevts;
        if elapsed_time == 0 { elapsed_time = 1; }
        if elapsed_time > (2 * HEARTBEAT) { return 0; }

        let elapsed_time_float     = elapsed_time as f64;
        let start_float            = start as f64;
        let work_needed_float: f64 = start_float / elapsed_time_float;
	    let work_needed		   = work_needed_float * 100_000_000.0;

	    return work_needed.round() as u64;
    }

    pub fn adjust_work_needed(previous_block_header: BlockHeader, current_block_timestamp: u64) -> Self {
        let start: f32 = BurnFee::burn_fee_adjustment(previous_block_header.clone(), current_block_timestamp);
        let current: u64 = BurnFee::return_work_needed(previous_block_header.ts, current_block_timestamp, previous_block_header.bf.start);

        return BurnFee::new(start, current);
    }

    pub fn burn_fee_adjustment(previous_block_header: BlockHeader, current_block_timestamp: u64) -> f32 {
        return previous_block_header.bf.start * ((HEARTBEAT) as f32 / ((current_block_timestamp - previous_block_header.ts) + 1) as f32).sqrt();
    }
}

