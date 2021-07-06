extern crate rpassword;

use clap::Clap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::Read;

// cargo run -- --help
// cargo run -- --key-path boom.txt

#[derive(Clap, Debug)]
#[clap(name = "saito_rust")]
struct KeyFile {
    /// Path to key-file
    #[clap(short, long, default_value = "./keyFile")]
    key_path: String,
}

#[derive(Debug, Clone)]
pub struct KeyPairStore {}

impl Default for KeyPairStore {
    fn default() -> Self {
        KeyPairStore::new()
    }
}

impl KeyPairStore {
    /// Create new `KeyPairStore`
    pub fn new() -> Self {
        println!("----> KEYPAIRSTORE");

        let key_file = KeyFile::parse();

        println!("KeyFile path: {}", key_file.key_path);

        let data = || -> Result<String, io::Error> {
            let mut f = File::open(&key_file.key_path)?;
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            Ok(s)
        };

        match data() {
            Err(_e) => {
                // panic!("{}", e)
                // @QUESTION: why is this failing if i leave out the expect?
                // because the type doesn't math then but why? when to know to do what?
                KeyPairStore::create_key_file(&key_file.key_path).expect("Failed to create file!")
            }
            Ok(s) => {
                println!("PW File content: {}", s);
                s
            }
        };

        KeyPairStore {}
    }

    pub fn create_key_file(key_file_path: &str) -> Result<String, io::Error> {
        println!("File does not exist. Let's create it!");
        let password = rpassword::prompt_password_stdout("Password: ").unwrap();
        println!("Your password is {}", password);

        // btw, for directly reading/writing a whoel fiel to/from a string, there is std::fs::read_to_string and std::Fs::write

        let mut file = File::create(key_file_path)?;
        file.write_all(password.as_bytes())?;
        Ok(password)
    }
}
