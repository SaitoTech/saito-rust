use std::{env, fs::File, io::Write, path::Path};

// Please do not change this value, it will cause tests to run very long
// If we are doing a production build, this value can be set via environmental variable
// as described below
const GENESIS_PERIOD: i64 = 1000;

fn main() {
    write_epoch_length_to_constants();
}

// The GENESIS_PERIOD must be a const because it is needed at compile time to set the length
// of the epoch_ring in longest_chain_queue. However, if we set this to a reasonable number
// this makes testing the longest_chain_queue very difficult. To work around this we use
// the build script to include!() constants.rs.
// The value can also be overridden by set by environmental variables.
// e.g.
// GENESIS_PERIOD=10000 cargo test
// GENESIS_PERIOD=10000 cargo build

fn write_epoch_length_to_constants() {
    let out_dir = env::var("OUT_DIR").expect("No out dir");
    let dest_path = Path::new(&out_dir).join("constants.rs");
    let epoch_length = option_env!("GENESIS_PERIOD");
    let epoch_length = epoch_length
        .map_or(Ok(GENESIS_PERIOD), str::parse)
        .expect("Could not parse GENESIS_PERIOD");
    let mut f = File::create(&dest_path).expect("Could not create file");
    write!(&mut f, "const GENESIS_PERIOD: u64 = {};", epoch_length).expect("Could not write file");
    println!("cargo:rerun-if-env-changed=GENESIS_PERIOD");
}
