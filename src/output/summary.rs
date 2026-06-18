use crate::config::Config;

fn truncate_address(addr: &str) -> String {
    if addr.len() <= 12 {
        return addr.to_string();
    }
    format!("{}....{}", &addr[..8], &addr[addr.len() - 4..])
}

pub fn print_config_summary(config: &Config) {
    println!("╔══════════════════════════════════════╗");
    println!("║         lvr-meter dry run            ║");
    println!("╚══════════════════════════════════════╝");
    println!("  Wallet:   {}", truncate_address(config.wallet.as_str()));
    println!("  From:     {}", config.date_range.from_date());
    println!("  To:       {}", config.date_range.to_date());
    println!("  Days:     {}", config.date_range.num_days());
    println!("  Protocol: {:?}", config.filter.protocol);
    println!(
        "  Pool:     {}",
        config
            .filter
            .specific_pool
            .as_deref()
            .map(truncate_address)
            .unwrap_or_else(|| "all pools".to_string())
    );
    println!("  RPC:      {}", truncate_rpc(&config.rpc_url));
}

fn truncate_rpc(url: &str) -> String {
    // Hide the API key in the URL
    if let Some(pos) = url.find("api-key=") {
        format!("{}api-key=****", &url[..pos])
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn truncate_long_address() {
    let addr = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv";
    assert_eq!(truncate_address(addr), "7xKXtg2C....gHkv");  
  }

    #[test]
    fn truncate_short_address_unchanged() {
        let addr = "short";
        assert_eq!(truncate_address(addr), "short");
    }

    #[test]
    fn truncate_rpc_hides_key() {
        let url = "https://mainnet.helius-rpc.com/?api-key=secret123";
        assert!(truncate_rpc(url).contains("****"));
        assert!(!truncate_rpc(url).contains("secret123"));
    }
}