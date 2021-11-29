/*!
# Saito Command Line Wallet Interface

A binary for interacting with your node's wallet and the Saito blockchain.

## Usage

```bash
saitocli help [subcommand]
```

## Available subcommands

**print**

prints keys from wallet

**tx**

create a sign a transaction

**create_tx**

create vip tx for modelling test network only

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
or
```
cargo run --bin saitocli -- print --keyfile test/testwallet --password asdf
```
or
```
cargo run --bin saitocli -- block --filename 0000017bdd95d7d7-22bf9b0495da48e917180871c4498139a0a320e8f31dd8a94181a82e69ca6ce4.sai
```
or
```
cargo run --bin saitocli -- blocks
```
or
```
cargo run --bin saitocli -- create_tx -a 1 -t gYsu1fVHjP6Z8CHCzti9K9xb5JPqpEL7zi7arvLiVANm  --keyfile test/testwallet --password asdf -o 0 -f data/test/out1.tx
```

*/

use base58::FromBase58;
use clap::{App, Arg};
use saito_rust::{
    block::Block,
    crypto::{hash, SaitoHash},
    slip::Slip,
    storage::{Storage, BLOCKS_DIR_PATH},
    transaction::{Transaction, TransactionType},
    wallet::Wallet,
};
use secp256k1::PublicKey;
use std::{
    convert::TryInto,
    fs::{self, File},
    io::{Read, Write},
};

// TODO Combine this into the main binary?
#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let command_matches = App::new("Saito Command Line Interface")
        .about("Interact with your wallet and the Saito blockchain through the command line")
        .subcommand(
            App::new("print")
                .about("prints the keypair")
                .arg(
                    Arg::with_name("keyfile")
                        .short("k")
                        .long("keyfile")
                        .required(true)
                        .takes_value(true)
                        .help("path to keyfile"),
                )
                .arg(
                    Arg::with_name("password")
                        .short("p")
                        .long("password")
                        .required(true)
                        .takes_value(true)
                        .help("password of keyfile"),
                ),
        )
        .subcommand(
            App::new("tx")
                .about("creates transactions using the wallet")
                .arg(
                    Arg::with_name("keyfile")
                        .short("k")
                        .long("keyfile")
                        .required(true)
                        .takes_value(true)
                        .help("path to keyfile"),
                )
                .arg(
                    Arg::with_name("password")
                        .short("p")
                        .long("password")
                        .required(true)
                        .takes_value(true)
                        .help("password of keyfile"),
                )
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
                        .help("the recipient"),
                )
                .arg(
                    Arg::with_name("filename")
                        .short("f")
                        .long("filename")
                        .takes_value(true)
                        .help("output file"),
                ),
        )
        .subcommand(
            App::new("block")
                .about("print info about a block file")
                .arg(
                    Arg::with_name("filename")
                        .short("f")
                        .long("filename")
                        .takes_value(true)
                        .required(true)
                        .help("amount to send"),
                ),
        )
        .subcommand(
            App::new("blocks")
                .about("print info about all blocks in the data directory")
                .arg(
                    Arg::with_name("path")
                        .short("p")
                        .long("path")
                        .takes_value(true)
                        .help("path to blocks directory"),
                ),
        )
        .subcommand(
            App::new("create_tx")
                .about("create VIP transaction")
                .arg(
                    Arg::with_name("keyfile")
                        .short("k")
                        .long("keyfile")
                        .required(true)
                        .takes_value(true)
                        .help("path to keyfile"),
                )
                .arg(
                    Arg::with_name("password")
                        .short("p")
                        .long("password")
                        .required(true)
                        .takes_value(true)
                        .help("password of keyfile"),
                )
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
                        .help("the recipient"),
                )
                .arg(
                    Arg::with_name("filename")
                        .short("f")
                        .long("filename")
                        .takes_value(true)
                        .help("output file"),
                )
                .arg(
                    Arg::with_name("input-tx")
                        .short("i")
                        .takes_value(true)
                        .default_value("data/test/out.tx")
                        .help("input transaction hash"),
                )
                .arg(
                    Arg::with_name("input-ordinal")
                        .short("o")
                        .takes_value(true)
                        .help("order of an input"),
                ),
        )
        .get_matches();

    if let Some(matches) = command_matches.subcommand_matches("print") {
        let key_file = matches.value_of("keyfile").unwrap();
        let password = matches.value_of("password");

        let mut wallet = Wallet::new();
        wallet.save();
        wallet.load_wallet(key_file, password);

        println!("public key : {}", hex::encode(wallet.get_publickey()));
        println!("private key : {}", hex::encode(wallet.get_privatekey()));
    }
    if let Some(matches) = command_matches.subcommand_matches("block") {
        let mut filename = BLOCKS_DIR_PATH.clone();
        let block_filename = matches.value_of("filename").unwrap();
        filename.push_str(block_filename);
        let block = Storage::load_block_from_disk(filename).await;
        println!("hash: {:?}", &hex::encode(&block.get_hash()));
        println!(
            "prev hash: {:?}",
            &hex::encode(&block.get_previous_block_hash())
        );
    }
    if let Some(matches) = command_matches.subcommand_matches("blocks") {
        let blocks_dir = match matches.value_of("path") {
            Some(path) => String::from(path),
            None => BLOCKS_DIR_PATH.clone(),
        };
        println!("blocks_dir {} {:?}", blocks_dir, matches.value_of("path"));
        let mut paths: Vec<_> = fs::read_dir(blocks_dir.clone())
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        paths.sort_by(|a, b| {
            let a_metadata = fs::metadata(a.path()).unwrap();
            let b_metadata = fs::metadata(b.path()).unwrap();
            a_metadata
                .modified()
                .unwrap()
                .partial_cmp(&b_metadata.modified().unwrap())
                .unwrap()
        });
        for (_pos, path) in paths.iter().enumerate() {
            println!("path: {:?}", path.path());
            if !path.path().to_str().unwrap().ends_with(".gitignore") {
                let mut f = File::open(path.path()).unwrap();
                let mut encoded = Vec::<u8>::new();
                f.read_to_end(&mut encoded).unwrap();
                let mut block = Block::deserialize_for_net(&encoded);
                println!("--------------------------------------------------------------");
                println!("filename: {:?}", path);
                println!("hash on disk  : {:?}", &hex::encode(&block.get_hash()));
                println!(
                    "prev hash     : {:?}",
                    &hex::encode(&block.get_previous_block_hash())
                );
                block.generate_hashes();
                println!("computed hash : {:?}", &hex::encode(&block.get_hash()));
            }
        }
    }
    if let Some(matches) = command_matches.subcommand_matches("tx") {
        let key_file = matches.value_of("keyfile").unwrap();
        let password = matches.value_of("password");

        let mut wallet = Wallet::new();
        wallet.load_wallet(key_file, password);

        let filename: String = match matches.value_of("filename") {
            Some(filename) => String::from(filename),
            None => String::from("transaction.out"),
        };
        let amount: u64 = matches
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

        // get inputs from the wallet and use the amount specified
        let mut input1 = Slip::new();
        input1.set_publickey(wallet.get_publickey());
        input1.set_amount(amount);
        input1.set_uuid([0; 32]);

        let mut output1 = Slip::new();
        output1.set_publickey(to_pubkey.serialize());
        output1.set_amount(amount);
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
    if let Some(matches) = command_matches.subcommand_matches("create_tx") {
        let key_file = matches.value_of("keyfile").unwrap();
        let password = matches.value_of("password");

        // let log_file = matches.value_of("log-output-path").to_str().unwrap();
        // let log_level = matches.value_of("log-level");

        let mut wallet = Wallet::new();
        wallet.load_wallet(key_file, password);

        let out_file: String = match matches.value_of("filename") {
            Some(out_file) => String::from(out_file),
            None => String::from("out.tx"),
        };
        let amount: u64 = matches
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

        let input_tx_fp = matches.value_of("input-tx").unwrap();
        let mut f = File::open(input_tx_fp).unwrap();
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).unwrap();
        let input_tx = encoded.try_into().unwrap();

        let input_ordinal: u8 = matches
            .value_of("input-ordinal")
            .unwrap()
            .parse()
            .unwrap_or_else(|_error| {
                println!("input_ordinal must be an int");
                println!("got {}", matches.value_of("input-ordinal").unwrap());
                std::process::exit(1);
            });

        let mut transaction = Transaction::new();
        transaction.set_transaction_type(TransactionType::Vip);

        let mut slip_inp = Slip::new();
        slip_inp.set_slip_ordinal(input_ordinal);
        slip_inp.set_uuid(input_tx);

        let mut slip_outp = Slip::new();
        slip_outp.set_publickey(to_pubkey.serialize());
        slip_outp.set_amount(amount);
        slip_outp.set_uuid([0; 32]);

        transaction.add_input(slip_inp);
        transaction.add_output(slip_outp);

        let hash_for_signature: SaitoHash = hash(&transaction.serialize_for_signature());
        transaction.set_hash_for_signature(hash_for_signature);

        transaction.sign(wallet.get_privatekey());

        println!("Propagate transaction");
        println!("Amount: {}", amount);
        println!("To: {}", to_pubkey);
        println!("====> {}", out_file);

        let tx_out = transaction.get_hash_for_signature();
        let mut buffer = File::create(out_file).unwrap();
        buffer.write_all(&tx_out.unwrap()[..]).unwrap();
        buffer.flush()?;
    }
    Ok(())
}
