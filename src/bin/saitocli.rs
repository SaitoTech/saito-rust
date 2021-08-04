/*!
# Saito Command Line Interface

A binary for interacting with your node's wallet and the Saito blockchain.

## Usage

```bash
saitocli help [subcommand]
```

## Available subcommands

**print**

prints keys from wallet

**tx**

create a sign a transcation

## Example

```bash
saitocli tx --amount 1 --to gYsu1fVHjP6Z8CHCzti9K9xb5JPqpEL7zi7arvLiVANm --filename tx.out
```
or
```bash
saitocli tx -a 1 -t gYsu1fVHjP6Z8CHCzti9K9xb5JPqpEL7zi7arvLiVANm
```

## Dev

To run from source:

```bash
cargo run --bin saitocli -- tx --amount 1 --to gYsu1fVHjP6Z8CHCzti9K9xb5JPqpEL7zi7arvLiVANm --filename tx.out
```
or
```bash
cargo run --bin saitocli -- tx -a 1 -t gYsu1fVHjP6Z8CHCzti9K9xb5JPqpEL7zi7arvLiVANm -f tx.out
```
*/
use base58::FromBase58;
use clap::{App, Arg};
use saito_rust::{
    crypto::{hash, SaitoHash},
    slip::Slip,
    transaction::{Transaction, TransactionType},
    wallet::Wallet,
};
use secp256k1::PublicKey;
use std::{fs::File, io::Write};

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let wallet = Wallet::new();
    let matches = App::new("Saito Command Line Interface")
        .about("Interact with your wallet and the Saito blockchain through the command line")
        .subcommand(
            App::new("tx")
                .about("controls testing features")
                .arg(
                    Arg::with_name("amount")
                        .short("a")
                        .long("amount")
                        .takes_value(true)
                        .required(true)
                        .help("amount to send"),
                )
                .arg(
                    Arg::with_name("to")
                        .short("t")
                        .long("to")
                        .takes_value(true)
                        .required(true)
                        .help("amount to send"),
                )
                .arg(
                    Arg::with_name("filename")
                        .short("f")
                        .long("filename")
                        .takes_value(true)
                        .help("output file"),
                ),
        )
        .subcommand(App::new("print").about("prints the keypair"))
        .get_matches();

    if let Some(ref _matches) = matches.subcommand_matches("print") {
        // TODO add an arg to enable/disable printing the private key
        // TODO add and arg to chande modes between hex and base58
        println!(" public key : {}", hex::encode(wallet.get_publickey()));
        println!("private key : {}", hex::encode(wallet.get_privatekey()));
    }
    if let Some(ref matches) = matches.subcommand_matches("tx") {
        let filename: String = match matches.value_of("filename") {
            Some(filename) => String::from(filename),
            None => String::from("transaction.out"),
        };
        let amount: f32 = matches
            .value_of("amount")
            .unwrap()
            .parse()
            .unwrap_or_else(|_error| {
                println!("amount must be a float");
                println!("got {}", matches.value_of("amount").unwrap());
                std::process::exit(1);
            });
        let to_pubkey =
            PublicKey::from_slice(&matches.value_of("to").unwrap().from_base58().unwrap())
                .unwrap_or_else(|_error| {
                    println!("Invalid pubkey in to field. Should be based58 encoded.");
                    std::process::exit(1);
                });

        let mut transaction = Transaction::new();
        transaction.set_transaction_type(TransactionType::Normal);

        // TODO: get inputs from the wallet and use the amount specified
        let mut input1 = Slip::new();
        input1.set_publickey(wallet.get_publickey());
        input1.set_amount(0);
        input1.set_uuid([0; 32]);

        let mut output1 = Slip::new();
        output1.set_publickey(to_pubkey.serialize());
        output1.set_amount(0);
        output1.set_uuid([0; 32]);

        transaction.add_input(input1);
        transaction.add_output(output1);

        let hash_for_signature: SaitoHash = hash(&transaction.serialize_for_signature());
        transaction.set_hash_for_signature(hash_for_signature);

        transaction.sign(wallet.get_privatekey());

        println!("Writing transaction");
        println!("Amount: {}", amount);
        println!("To: {}", to_pubkey);
        println!("====> {}", filename);

        let output = transaction.serialize_for_net();
        let mut buffer = File::create(filename).unwrap();
        buffer.write_all(&output[..]).unwrap();
        buffer.flush()?;
    }
    Ok(())
}
