use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::flows::OptionParams;

/// 시스템 이벤트 타입
#[derive(Clone, Debug)]
pub enum Event {
    PriceUpdate {
        price: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    OptionCreated {
        option_id: String,
        params: OptionParams,
    },
    OptionPurchased {
        option_id: String,
        buyer: String,
        premium: f64,
    },
    OptionExpired {
        option_id: String,
    },
    SettlementCompleted {
        option_id: String,
        payout: f64,
    },
    Error {
        module: String,
        message: String,
    },
}

type EventHandler = Arc<dyn Fn(Event) + Send + Sync>;

/// 이벤트 버스 - 모듈 간 통신
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<String, Vec<EventHandler>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 이벤트 핸들러 등록
    pub fn subscribe<F>(&self, event_type: Event, handler: F) 
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let event_name = match &event_type {
            Event::PriceUpdate { .. } => "PriceUpdate",
            Event::OptionCreated { .. } => "OptionCreated",
            Event::OptionPurchased { .. } => "OptionPurchased",
            Event::OptionExpired { .. } => "OptionExpired",
            Event::SettlementCompleted { .. } => "SettlementCompleted",
            Event::Error { .. } => "Error",
        };

        tokio::spawn({
            let handlers = self.handlers.clone();
            async move {
                let mut handlers = handlers.write().await;
                handlers
                    .entry(event_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(Arc::new(handler));
            }
        });
    }

    /// 이벤트 발행
    pub async fn emit(&self, event: Event) {
        let event_name = match &event {
            Event::PriceUpdate { .. } => "PriceUpdate",
            Event::OptionCreated { .. } => "OptionCreated",
            Event::OptionPurchased { .. } => "OptionPurchased",
            Event::OptionExpired { .. } => "OptionExpired",
            Event::SettlementCompleted { .. } => "SettlementCompleted",
            Event::Error { .. } => "Error",
        };

        let handlers = self.handlers.read().await;
        if let Some(event_handlers) = handlers.get(event_name) {
            for handler in event_handlers {
                let event_clone = event.clone();
                let handler_clone = handler.clone();
                tokio::spawn(async move {
                    handler_clone(event_clone);
                });
            }
        }
    }
}