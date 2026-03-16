use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Chain {
    Solana,
    Ethereum,
    Polygon,
    Arbitrum,
    Base,
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Chain::Solana => write!(f, "SOLANA"),
            Chain::Ethereum => write!(f, "ETHEREUM"),
            Chain::Polygon => write!(f, "POLYGON"),
            Chain::Arbitrum => write!(f, "ARBITRUM"),
            Chain::Base => write!(f, "BASE"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetSymbol {
    Usdc,
    Eth,
    Sol,
    Weth,
    Wsol,
}

impl fmt::Display for AssetSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetSymbol::Usdc => write!(f, "USDC"),
            AssetSymbol::Eth => write!(f, "ETH"),
            AssetSymbol::Sol => write!(f, "SOL"),
            AssetSymbol::Weth => write!(f, "WETH"),
            AssetSymbol::Wsol => write!(f, "WSOL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Asset {
    pub symbol: AssetSymbol,
    pub chain: Chain,
    /// On-chain token mint/contract address
    pub contract_address: Option<String>,
    pub decimals: u8,
}

impl Asset {
    pub fn usdc_solana() -> Self {
        Self {
            symbol: AssetSymbol::Usdc,
            chain: Chain::Solana,
            contract_address: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
            decimals: 6,
        }
    }

    pub fn sol() -> Self {
        Self {
            symbol: AssetSymbol::Sol,
            chain: Chain::Solana,
            contract_address: None,
            decimals: 9,
        }
    }

    pub fn usdc_ethereum() -> Self {
        Self {
            symbol: AssetSymbol::Usdc,
            chain: Chain::Ethereum,
            contract_address: Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()),
            decimals: 6,
        }
    }

    pub fn eth() -> Self {
        Self {
            symbol: AssetSymbol::Eth,
            chain: Chain::Ethereum,
            contract_address: None,
            decimals: 18,
        }
    }
}

/// Monetary amount with asset information, using Decimal for precision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub value: Decimal,
    pub asset: AssetSymbol,
}

impl Amount {
    pub fn new(value: Decimal, asset: AssetSymbol) -> Self {
        Self { value, asset }
    }

    pub fn zero(asset: AssetSymbol) -> Self {
        Self {
            value: Decimal::ZERO,
            asset,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    pub fn is_positive(&self) -> bool {
        self.value.is_sign_positive() && !self.value.is_zero()
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_display() {
        let amt = Amount::new(Decimal::new(1050, 2), AssetSymbol::Usdc);
        assert_eq!(format!("{}", amt), "10.50 USDC");
    }

    #[test]
    fn test_asset_presets() {
        let usdc = Asset::usdc_solana();
        assert_eq!(usdc.decimals, 6);
        assert_eq!(usdc.chain, Chain::Solana);

        let eth = Asset::eth();
        assert_eq!(eth.decimals, 18);
    }
}
