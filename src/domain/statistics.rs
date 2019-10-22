use super::{
    amount::*,
    market::Market,
    offer::{OfferDirection, OfferId},
};
use crate::bisq::PersistentMessageHash;
use std::time::SystemTime;

#[derive(Clone)]
pub struct Trade {
    pub market: &'static Market,
    pub direction: OfferDirection,
    pub offer_id: OfferId,
    pub price: NumberWithPrecision,
    pub amount: NumberWithPrecision,
    pub volume: NumberWithPrecision,
    pub payment_method_id: String,
    pub timestamp: SystemTime,
    pub hash: PersistentMessageHash,
}
impl Trade {
    pub fn new(
        market: &'static Market,
        direction: OfferDirection,
        offer_id: OfferId,
        price: NumberWithPrecision,
        amount: NumberWithPrecision,
        payment_method_id: String,
        timestamp: SystemTime,
        hash: PersistentMessageHash,
    ) -> Self {
        Self {
            market,
            direction,
            offer_id,
            price,
            amount,
            volume: price * amount,
            payment_method_id,
            timestamp,
            hash,
        }
    }
}

#[cfg(feature = "statistics")]
pub use inner::*;
#[cfg(feature = "statistics")]
mod inner {
    use super::*;
    use crate::{
        bisq::PersistentMessageHash,
        domain::{CommandResult, FutureCommandResult},
        prelude::*,
    };
    use std::{collections::HashSet, sync::Arc};

    pub struct TradeHistory {
        inner: Vec<Trade>,
    }
    impl TradeHistory {
        fn new() -> Self {
            Self { inner: Vec::new() }
        }
        fn insert(&mut self, trade: Trade) {
            for n in (0..=self.inner.len()).rev() {
                if n == 0 || trade.timestamp > self.inner[n - 1].timestamp {
                    self.inner.insert(n, trade);
                    break;
                }
            }
        }
    }
    pub struct StatsCacheInner {
        trades: TradeHistory,
        hashes: HashSet<PersistentMessageHash>,
    }
    impl StatsCacheInner {
        fn insert(&mut self, trade: Trade) -> CommandResult {
            if self.hashes.insert(trade.hash) {
                self.trades.insert(trade);
                CommandResult::Accepted
            } else {
                CommandResult::Ignored
            }
        }
        pub fn trades(&self) -> impl DoubleEndedIterator<Item = &Trade> {
            self.trades.inner.iter()
        }
    }

    #[derive(Clone)]
    pub struct StatsCache {
        inner: Arc<locks::RwLock<StatsCacheInner>>,
    }
    impl StatsCache {
        pub fn new() -> Option<Self> {
            Some(Self {
                inner: Arc::new(locks::RwLock::new(StatsCacheInner {
                    trades: TradeHistory::new(),
                    hashes: HashSet::new(),
                })),
            })
        }

        pub fn add(&self, trade: Trade) -> impl FutureCommandResult {
            self.inner
                .write()
                .map(move |mut inner| inner.insert(trade))
                .map_err(|_| MailboxError::Closed)
        }

        pub fn inner(
            &self,
        ) -> impl Future<Item = locks::RwLockReadGuard<StatsCacheInner>, Error = ()> {
            self.inner.read()
        }
    }
}

#[cfg(not(feature = "statistics"))]
pub use empty::*;
#[cfg(not(feature = "statistics"))]
mod empty {
    #[derive(Clone)]
    pub struct StatsCache;
    impl StatsCache {
        pub fn new() -> Option<Self> {
            None
        }
    }
}
