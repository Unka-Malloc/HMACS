use crate::order::{Order, OrderSide, OrderStatus};
use chrono::Utc;
use carbide_core::{AssetSymbol, OrderId};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Price-time priority order book for a single asset.
#[derive(Debug)]
pub struct OrderBook {
    pub asset: AssetSymbol,
    /// Buy orders sorted by price descending (highest first), then by time ascending
    bids: RwLock<BTreeMap<OrderBookKey, Order>>,
    /// Sell orders sorted by price ascending (lowest first), then by time ascending
    asks: RwLock<BTreeMap<OrderBookKey, Order>>,
}

/// Composite key for price-time priority.
/// For bids: negate price so BTreeMap natural ordering gives highest price first.
/// For asks: natural price ordering gives lowest price first.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OrderBookKey {
    /// Encoded price for sorting. Bids use negated price; asks use raw price.
    price_sort: i128,
    timestamp_nanos: i64,
    id: uuid::Uuid,
}

impl OrderBookKey {
    fn for_bid(price: Decimal, order: &Order) -> Self {
        let raw = price_to_i128(price);
        Self {
            price_sort: -raw,
            timestamp_nanos: order.created_at.timestamp_nanos_opt().unwrap_or(0),
            id: order.id.0,
        }
    }

    fn for_ask(price: Decimal, order: &Order) -> Self {
        Self {
            price_sort: price_to_i128(price),
            timestamp_nanos: order.created_at.timestamp_nanos_opt().unwrap_or(0),
            id: order.id.0,
        }
    }
}

fn price_to_i128(price: Decimal) -> i128 {
    // Scale to 18 decimal places for consistent sorting
    let scaled = price * Decimal::new(1_000_000_000_000_000_000, 0);
    scaled.mantissa()
}

impl OrderBook {
    pub fn new(asset: AssetSymbol) -> Self {
        Self {
            asset,
            bids: RwLock::new(BTreeMap::new()),
            asks: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn insert(&self, order: Order) {
        match order.side {
            OrderSide::Buy => {
                let key = OrderBookKey::for_bid(order.price, &order);
                self.bids.write().insert(key, order);
            }
            OrderSide::Sell => {
                let key = OrderBookKey::for_ask(order.price, &order);
                self.asks.write().insert(key, order);
            }
        }
    }

    pub fn cancel(&self, order_id: OrderId, side: OrderSide) -> Option<Order> {
        match side {
            OrderSide::Buy => {
                let mut bids = self.bids.write();
                let key = bids
                    .iter()
                    .find(|(_, o)| o.id == order_id)
                    .map(|(k, _)| k.clone());
                key.and_then(|k| {
                    let mut order = bids.remove(&k)?;
                    order.status = OrderStatus::Cancelled;
                    order.updated_at = Utc::now();
                    Some(order)
                })
            }
            OrderSide::Sell => {
                let mut asks = self.asks.write();
                let key = asks
                    .iter()
                    .find(|(_, o)| o.id == order_id)
                    .map(|(k, _)| k.clone());
                key.and_then(|k| {
                    let mut order = asks.remove(&k)?;
                    order.status = OrderStatus::Cancelled;
                    order.updated_at = Utc::now();
                    Some(order)
                })
            }
        }
    }

    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.read().values().next().map(|o| o.price)
    }

    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.read().values().next().map(|o| o.price)
    }

    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn bid_count(&self) -> usize {
        self.bids.read().len()
    }

    pub fn ask_count(&self) -> usize {
        self.asks.read().len()
    }

    /// Take the best bid (highest price buy order).
    pub fn take_best_bid(&self) -> Option<Order> {
        let mut bids = self.bids.write();
        let key = bids.keys().next()?.clone();
        bids.remove(&key)
    }

    /// Take the best ask (lowest price sell order).
    pub fn take_best_ask(&self) -> Option<Order> {
        let mut asks = self.asks.write();
        let key = asks.keys().next()?.clone();
        asks.remove(&key)
    }

    /// Re-insert an order that was partially filled.
    pub fn reinsert(&self, order: Order) {
        self.insert(order);
    }
}

/// Thread-safe collection of order books keyed by asset.
pub struct OrderBookManager {
    books: dashmap::DashMap<AssetSymbol, Arc<OrderBook>>,
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            books: dashmap::DashMap::new(),
        }
    }

    pub fn get_or_create(&self, asset: AssetSymbol) -> Arc<OrderBook> {
        self.books
            .entry(asset)
            .or_insert_with(|| Arc::new(OrderBook::new(asset)))
            .clone()
    }

    pub fn get(&self, asset: &AssetSymbol) -> Option<Arc<OrderBook>> {
        self.books.get(asset).map(|r| r.clone())
    }
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{OrderType};
    use carbide_core::ParticipantId;
    use rust_decimal_macros::dec;

    fn make_order(side: OrderSide, price: Decimal, qty: Decimal) -> Order {
        Order {
            id: OrderId::new(),
            participant_id: ParticipantId::new(),
            side,
            order_type: OrderType::Limit,
            asset: AssetSymbol::Usdc,
            price,
            quantity: qty,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            reference_id: None,
            reference_type: None,
        }
    }

    #[test]
    fn test_bid_ask_ordering() {
        let book = OrderBook::new(AssetSymbol::Usdc);
        book.insert(make_order(OrderSide::Buy, dec!(10), dec!(1)));
        book.insert(make_order(OrderSide::Buy, dec!(12), dec!(1)));
        book.insert(make_order(OrderSide::Buy, dec!(11), dec!(1)));

        assert_eq!(book.best_bid(), Some(dec!(12)));
        assert_eq!(book.bid_count(), 3);
    }

    #[test]
    fn test_ask_ordering() {
        let book = OrderBook::new(AssetSymbol::Usdc);
        book.insert(make_order(OrderSide::Sell, dec!(15), dec!(1)));
        book.insert(make_order(OrderSide::Sell, dec!(13), dec!(1)));
        book.insert(make_order(OrderSide::Sell, dec!(14), dec!(1)));

        assert_eq!(book.best_ask(), Some(dec!(13)));
        assert_eq!(book.ask_count(), 3);
    }

    #[test]
    fn test_spread() {
        let book = OrderBook::new(AssetSymbol::Usdc);
        book.insert(make_order(OrderSide::Buy, dec!(10), dec!(1)));
        book.insert(make_order(OrderSide::Sell, dec!(12), dec!(1)));
        assert_eq!(book.spread(), Some(dec!(2)));
    }
}
