use std::env;

#[derive(Debug, Clone)]
pub struct KeyPairStore {}

impl KeyPairStore {
    /// Create new `KeyPairStore`
    pub fn new() -> Self {
        println!("----> KEYPAIRSTORE");
        let args: Vec<String> = env::args().collect();

        println!("arguments: {}", args.len());

        if args.len() > 1 {
            println!("argument given");
            let command = args[1].clone();

            if command == "foo" {
                println!("argument is foo");
                let foo_arg = args[2].clone();
                println!("FOO is: {}", &foo_arg.to_string());
            }
        } else {
            println!("no arguments given");
        }

        KeyPairStore {}
    }
}
