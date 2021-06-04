pub const HEARTBEAT: u64 = 30_000;
/// The Burnfee object which contains our starting value and current value when the fee was paid
#[derive(PartialEq, Clone, Debug)]
pub struct BurnFee {
    pub start: f64,
}

impl BurnFee {
    /// Returns the BurnFee used to calculate the work needed to produce a block
    ///
    /// * `start` - y-value at x = 0
    pub fn new(start: f64) -> Self {
        Self { start }
    }

    /// Returns the amount of work needed to produce a block given the timestamp of
    /// the previous block, the current timestamp, and the y-axis of the burn fee
    /// curve. This is used both in the creation of blocks (mempool) as well as
    /// during block validation.
    ///
    /// * `start` - burn fee value (y-axis) for curve determination ("start")
    /// * `current_block_timestamp`- candidate timestamp
    /// * `previous_block_timestamp` - timestamp of previous block
    pub fn return_work_needed(
        &self,
        current_block_timestamp: u64,
        previous_block_timestamp: u64,
    ) -> u64 {
        let mut elapsed_time = current_block_timestamp - previous_block_timestamp;
        if elapsed_time == 0 {
            elapsed_time = 1;
        }
        if elapsed_time >= (2 * HEARTBEAT) {
            return 0;
        }

        let elapsed_time_float = elapsed_time as f64;
        let start_float = self.start as f64;
        let work_needed_float: f64 = start_float / elapsed_time_float;
        let work_needed = work_needed_float * 100_000_000.0;

        return work_needed.round() as u64;
    }

    /// Returns an adjusted burnfee based on the start value provided
    /// and the difference between the current block timestamp and the
    /// previous block timestamp
    pub fn burn_fee_adjustment(
        &self,
        current_block_timestamp: u64,
        previous_block_timestamp: u64,
    ) -> f64 {
        let timestamp_difference = match current_block_timestamp - previous_block_timestamp {
            0 => 1,
            diff => diff,
        };
        return self.start * ((HEARTBEAT) as f64 / (timestamp_difference) as f64).sqrt();
    }

    /// Adjusts the start value inside the `BurnFee` object based on
    /// the provided timestamps
    ///
    /// * `current_block_timestamp` - Current `Block` timestamp
    /// * `previous_block_timestamp` - Previous `Block` timestamp
    pub fn adjust_work_needed(
        &mut self,
        current_block_timestamp: u64,
        previous_block_timestamp: u64,
    ) {
        self.start = self.burn_fee_adjustment(current_block_timestamp, previous_block_timestamp);
    }

    pub fn burn_fee_calculation(
        start: f64,
        current_block_timestamp: u64,
        previous_block_timestamp: u64,
    ) -> u64 {
        let timestamp_difference = match current_block_timestamp - previous_block_timestamp {
            0 => 1,
            diff => diff,
        };
        return (start * ((HEARTBEAT) as f64 / (timestamp_difference) as f64).sqrt()) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burnfee_test() {
        let bf = BurnFee::new(10.0);
        assert_eq!(bf.start, 10.0);
    }

    #[test]
    fn burnfee_return_work_needed_test() {
        let bf = BurnFee::new(10.0);

        // if our elapsed time is twice our heartbeat, return 0
        assert_eq!(bf.return_work_needed(2 * HEARTBEAT, 0), 0);

        // if their is no difference, the value should be the start value * 10^8
        assert_eq!(bf.return_work_needed(0, 0), 10_0000_0000);

        // should return 1 * 10^8 * timestamp_diff
        assert_eq!(bf.return_work_needed(HEARTBEAT / 3000, 0), 10000_0000);
    }

    #[test]
    fn burnfee_burn_fee_adjustment_test() {
        // if the difference in timestamps is equal to HEARTBEAT, our start value should not change
        let bf = BurnFee::new(10.0);
        assert_eq!(bf.burn_fee_adjustment(HEARTBEAT, 0), 10.0);

        // the difference should be the square root of HEARBEAT over the difference in timestamps
        assert_eq!(
            bf.burn_fee_adjustment(HEARTBEAT / 10, 0),
            10.0 * (10.0 as f64).sqrt()
        );
    }
}
