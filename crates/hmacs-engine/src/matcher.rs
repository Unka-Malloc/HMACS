use crate::order::{CreateOrder, Order, OrderSide, OrderStatus, OrderType, TradeExecution};
use crate::orderbook::OrderBookManager;
use chrono::Utc;
use hmacs_core::{AssetSymbol, HmacsError, HmacsResult, OrderId};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::info;

/// The matching engine processes incoming orders against the order book.
pub struct MatchingEngine {
    books: Arc<OrderBookManager>,
}

impl MatchingEngine {
    pub fn new(books: Arc<OrderBookManager>) -> Self {
        Self { books }
    }

    /// Submit a new order. Returns the placed order and any resulting trades.
    pub fn submit_order(&self, create: CreateOrder) -> HmacsResult<(Order, Vec<TradeExecution>)> {
        if create.quantity <= Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Quantity must be positive".into()));
        }
        if create.price < Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Price must be non-negative".into()));
        }

        let mut order = Order {
            id: OrderId::new(),
            participant_id: create.participant_id,
            side: create.side,
            order_type: create.order_type,
            asset: create.asset,
            price: create.price,
            quantity: create.quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: create.expires_at,
            reference_id: create.reference_id,
            reference_type: create.reference_type,
        };

        let book = self.books.get_or_create(order.asset);
        let mut trades = Vec::new();

        match order.order_type {
            OrderType::Limit => {
                self.match_limit_order(&mut order, &mut trades);
            }
            OrderType::Market => {
                self.match_market_order(&mut order, &mut trades);
            }
        }

        if !order.is_fully_filled() && order.order_type == OrderType::Limit {
            order.status = if order.filled_quantity > Decimal::ZERO {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::Open
            };
            book.insert(order.clone());
        } else if order.is_fully_filled() {
            order.status = OrderStatus::Filled;
        }

        info!(
            order_id = %order.id,
            trades = trades.len(),
            filled = %order.filled_quantity,
            "Order processed"
        );

        Ok((order, trades))
    }

    pub fn cancel_order(
        &self,
        order_id: OrderId,
        asset: AssetSymbol,
        side: OrderSide,
    ) -> HmacsResult<Order> {
        let book = self
            .books
            .get(&asset)
            .ok_or_else(|| HmacsError::not_found("OrderBook", asset))?;

        book.cancel(order_id, side)
            .ok_or_else(|| HmacsError::not_found("Order", order_id))
    }

    fn match_limit_order(&self, incoming: &mut Order, trades: &mut Vec<TradeExecution>) {
        let book = self.books.get_or_create(incoming.asset);

        loop {
            if incoming.is_fully_filled() {
                break;
            }

            let counter = match incoming.side {
                OrderSide::Buy => book.take_best_ask(),
                OrderSide::Sell => book.take_best_bid(),
            };

            let mut counter = match counter {
                Some(c) => c,
                None => break,
            };

            let price_matches = match incoming.side {
                OrderSide::Buy => incoming.price >= counter.price,
                OrderSide::Sell => incoming.price <= counter.price,
            };

            if !price_matches {
                book.reinsert(counter);
                break;
            }

            let exec_price = counter.price;
            let exec_qty = incoming
                .remaining_quantity()
                .min(counter.remaining_quantity());

            incoming.filled_quantity += exec_qty;
            counter.filled_quantity += exec_qty;

            let (buy_id, sell_id) = match incoming.side {
                OrderSide::Buy => (incoming.id, counter.id),
                OrderSide::Sell => (counter.id, incoming.id),
            };

            trades.push(TradeExecution {
                buy_order_id: buy_id,
                sell_order_id: sell_id,
                price: exec_price,
                quantity: exec_qty,
                asset: incoming.asset,
                executed_at: Utc::now(),
            });

            if !counter.is_fully_filled() {
                counter.status = OrderStatus::PartiallyFilled;
                counter.updated_at = Utc::now();
                book.reinsert(counter);
            }
        }
    }

    fn match_market_order(&self, incoming: &mut Order, trades: &mut Vec<TradeExecution>) {
        let book = self.books.get_or_create(incoming.asset);

        loop {
            if incoming.is_fully_filled() {
                break;
            }

            let counter = match incoming.side {
                OrderSide::Buy => book.take_best_ask(),
                OrderSide::Sell => book.take_best_bid(),
            };

            let mut counter = match counter {
                Some(c) => c,
                None => break,
            };

            let exec_price = counter.price;
            let exec_qty = incoming
                .remaining_quantity()
                .min(counter.remaining_quantity());

            incoming.filled_quantity += exec_qty;
            counter.filled_quantity += exec_qty;

            let (buy_id, sell_id) = match incoming.side {
                OrderSide::Buy => (incoming.id, counter.id),
                OrderSide::Sell => (counter.id, incoming.id),
            };

            trades.push(TradeExecution {
                buy_order_id: buy_id,
                sell_order_id: sell_id,
                price: exec_price,
                quantity: exec_qty,
                asset: incoming.asset,
                executed_at: Utc::now(),
            });

            if !counter.is_fully_filled() {
                counter.status = OrderStatus::PartiallyFilled;
                counter.updated_at = Utc::now();
                book.reinsert(counter);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmacs_core::ParticipantId;
    use rust_decimal_macros::dec;

    fn make_engine() -> MatchingEngine {
        MatchingEngine::new(Arc::new(OrderBookManager::new()))
    }

    fn limit_order(
        side: OrderSide,
        price: Decimal,
        qty: Decimal,
    ) -> CreateOrder {
        CreateOrder {
            participant_id: ParticipantId::new(),
            side,
            order_type: OrderType::Limit,
            asset: AssetSymbol::Usdc,
            price,
            quantity: qty,
            expires_at: None,
            reference_id: None,
            reference_type: None,
        }
    }

    #[test]
    fn test_no_match() {
        let engine = make_engine();
        let (order, trades) = engine
            .submit_order(limit_order(OrderSide::Buy, dec!(10), dec!(5)))
            .unwrap();
        assert!(trades.is_empty());
        assert_eq!(order.status, OrderStatus::Open);
    }

    #[test]
    fn test_exact_match() {
        let engine = make_engine();
        engine
            .submit_order(limit_order(OrderSide::Sell, dec!(10), dec!(5)))
            .unwrap();
        let (order, trades) = engine
            .submit_order(limit_order(OrderSide::Buy, dec!(10), dec!(5)))
            .unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, dec!(10));
        assert_eq!(trades[0].quantity, dec!(5));
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_partial_fill() {
        let engine = make_engine();
        engine
            .submit_order(limit_order(OrderSide::Sell, dec!(10), dec!(3)))
            .unwrap();
        let (order, trades) = engine
            .submit_order(limit_order(OrderSide::Buy, dec!(10), dec!(5)))
            .unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(3));
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.filled_quantity, dec!(3));
    }

    #[test]
    fn test_price_priority() {
        let engine = make_engine();
        engine
            .submit_order(limit_order(OrderSide::Sell, dec!(12), dec!(5)))
            .unwrap();
        engine
            .submit_order(limit_order(OrderSide::Sell, dec!(10), dec!(5)))
            .unwrap();
        let (_, trades) = engine
            .submit_order(limit_order(OrderSide::Buy, dec!(12), dec!(5)))
            .unwrap();
        assert_eq!(trades[0].price, dec!(10));
    }
}
