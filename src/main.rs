mod config;

use config::Config;

fn main() {
    let config = Config::from_env_and_args(
        "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv",
        "2025-01-01",
        "2025-03-31",
        "both",
        None,
    );

    match config {
        Ok(c) => {
            println!("Wallet:   {}", c.wallet.as_str());
            println!("From:     {}", c.date_range.from_date());
            println!("To:       {}", c.date_range.to_date());
            println!("Days:     {}", c.date_range.num_days());
            println!("RPC URL:  {}", c.rpc_url);
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}