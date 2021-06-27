extern crate rpassword;

use clap::Clap;
use std::fs::File;
use std::io;
use std::io::Read;

// cargo run -- --key-path boom

#[derive(Clap, Debug)]
#[clap(name = "saito_rust")]
struct KeyFile {
    /// Path to key-file
    #[clap(short, long, default_value = "./keyFile")]
    key_path: String,
}

#[derive(Debug, Clone)]
pub struct KeyPairStore {}

impl KeyPairStore {
    /// Create new `KeyPairStore`
    pub fn new() -> Self {
        println!("----> KEYPAIRSTORE");

        let key_file = KeyFile::parse();

        println!("KeyFile path: {}", key_file.key_path);

        let data = || -> Result<String, io::Error> {
            let mut f = File::open(key_file.key_path)?;
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            Ok(s)
        };

        let _content = match data() {
            Err(_e) => {
                // panic!("{}", e)
                KeyPairStore::create_key_file()
            }
            Ok(s) => {
                println!("PW File content: {}", s);
                s
            }
        };

        // "./saito_rust --pw-file=/myencryptedusb/saitopw"
        // then it should read from that, otherwise it should read from
        // a standard location(maybe ./data/passwd?).
        // If --pw-file isn't passed and the standard location is empty, then it should say something like
        // - "no password file found, creating one at ./data/passwd, please enter a password:".
        // - "Loading password from {location which user has provided}, please enter password:"
        // - "Loading password from {standardize location on the system}, please enter password:"
        // - "Password file not found, creating one at {standardize location on the system}, please enter password:"

        KeyPairStore {}
    }

    pub fn create_key_file() -> String {
        // - ask for pw
        // - write pw to file to given path

        println!("Let's create a file");
        // "my_secure_password".to_owned()
        let pass = rpassword::prompt_password_stdout("Password: ").unwrap();
        println!("Your password is {}", pass);
        pass.to_string()
    }
}
