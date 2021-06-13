use clap::Clap;
// use std::env;
// use structopt::StructOpt;

/// Search for a pattern in a file and display the lines that contain it.
// #[derive(StructOpt)]
// struct Cli {
//     /// The pattern to look for
//     pattern: String,
//     /// The path to the file to read
//     #[structopt(parse(from_os_str))]
//     path: std::path::PathBuf,
// }

#[derive(Clap, Debug)]
#[clap(name = "saito")]
struct PwFile {
    /// Path to pw-file
    #[clap(short, long, default_value = "/spme/default/pw/path")]
    path: String,
}

#[derive(Debug, Clone)]
pub struct KeyPairStore {}

impl KeyPairStore {
    /// Create new `KeyPairStore`
    pub fn new() -> Self {
        println!("----> KEYPAIRSTORE");
        // let args = Cli::from_args();
        // let args: Vec<String> = env::args().collect();

        let pw_file = PwFile::parse();

        println!("PwFile {}!", pw_file.path);

        // println!("arguments: {}", args.len());

        // "./saito_rust --pw-file=/myencryptedusb/saitopw"
        // then it should read from that, otherwise it should read from
        // a standard location(maybe ./data/passwd?).
        // If --pw-file isn't passed and the standard location is empty, then it should say something like
        // - "no password file found, creating one at ./data/passwd, please enter a password:".
        // - "Loading password from {location which user has provided}, please enter password:"
        // - "Loading password from {standardize location on the system}, please enter password:"
        // - "Password file not found, creating one at {standardize location on the system}, please enter password:"

        // if args.len() > 1 {
        //     println!("argument given");
        //     let command = args[1].clone();

        //     if command == "pw-file" {
        //         println!("argument is `pw-file`");
        //         let arg_pw_file = args[2].clone();
        //         println!("pw-file is: {}", &arg_pw_file.to_string());
        //     }
        // } else {
        //     println!("no arguments given");
        // }

        KeyPairStore {}
    }
}
