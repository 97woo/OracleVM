use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use anyhow::Result;

mod flows;
mod connectors_real;
mod events;

// 실제 구현 사용
use connectors_real as connectors;

use flows::{UpdateFlow, TradingFlow, SettlementFlow};
use connectors::{OracleConnector, CalculationConnector, ContractConnector, BitVMXConnector};
use events::{EventBus, Event};

/// 시스템 전체 상태 관리
#[derive(Clone)]
struct SystemState {
    pub current_price: f64,
    pub last_update: chrono::DateTime<chrono::Utc>,
    pub active_options: Vec<String>,
    pub pending_settlements: Vec<String>,
}

/// 메인 오케스트레이터
struct Orchestrator {
    state: Arc<RwLock<SystemState>>,
    event_bus: Arc<EventBus>,
    oracle_connector: Arc<OracleConnector>,
    calc_connector: Arc<CalculationConnector>,
    contract_connector: Arc<ContractConnector>,
    bitvmx_connector: Arc<BitVMXConnector>,
}

impl Orchestrator {
    pub async fn new() -> Result<Self> {
        let state = Arc::new(RwLock::new(SystemState {
            current_price: 0.0,
            last_update: chrono::Utc::now(),
            active_options: Vec::new(),
            pending_settlements: Vec::new(),
        }));

        let event_bus = Arc::new(EventBus::new());
        
        Ok(Self {
            state: state.clone(),
            event_bus: event_bus.clone(),
            oracle_connector: Arc::new(OracleConnector::new("http://localhost:50051").await?),
            calc_connector: Arc::new(CalculationConnector::new("http://localhost:3000")?),
            contract_connector: Arc::new(ContractConnector::new()?),
            bitvmx_connector: Arc::new(BitVMXConnector::new()?),
        })
    }

    /// 시스템 시작
    pub async fn start(&self) -> Result<()> {
        info!("Starting BTCFi Orchestrator...");
        
        // 이벤트 핸들러 등록
        self.setup_event_handlers().await;
        
        // 시스템 플로우 시작
        tokio::try_join!(
            self.start_update_flow(),
            self.start_trading_flow(),
            self.start_settlement_flow(),
            self.start_monitoring()
        )?;
        
        Ok(())
    }

    /// 이벤트 핸들러 설정
    async fn setup_event_handlers(&self) {
        let event_bus = self.event_bus.clone();
        let state = self.state.clone();
        
        // 가격 업데이트 이벤트 핸들러
        let calc_connector = self.calc_connector.clone();
        event_bus.subscribe(Event::PriceUpdate, move |event| {
            let calc = calc_connector.clone();
            tokio::spawn(async move {
                if let Event::PriceUpdate { price, .. } = event {
                    // Calculation 모듈에 새로운 가격 전달
                    if let Err(e) = calc.update_price(price).await {
                        error!("Failed to update calculation price: {}", e);
                    }
                }
            });
        });

        // 옵션 생성 이벤트 핸들러
        let bitvmx = self.bitvmx_connector.clone();
        event_bus.subscribe(Event::OptionCreated, move |event| {
            let bitvmx = bitvmx.clone();
            tokio::spawn(async move {
                if let Event::OptionCreated { option_id, params } = event {
                    // BitVMX pre-sign 생성
                    if let Err(e) = bitvmx.create_presign(&option_id, &params).await {
                        error!("Failed to create BitVMX presign: {}", e);
                    }
                }
            });
        });

        // 만기 도달 이벤트 핸들러
        let settlement_flow = SettlementFlow::new(
            self.oracle_connector.clone(),
            self.bitvmx_connector.clone(),
            self.contract_connector.clone(),
        );
        
        event_bus.subscribe(Event::OptionExpired, move |event| {
            let flow = settlement_flow.clone();
            tokio::spawn(async move {
                if let Event::OptionExpired { option_id } = event {
                    if let Err(e) = flow.execute_settlement(&option_id).await {
                        error!("Settlement failed for {}: {}", option_id, e);
                    }
                }
            });
        });
    }

    /// Update 사이클: Oracle → Calculation → Frontend
    async fn start_update_flow(&self) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(30));
        let update_flow = UpdateFlow::new(
            self.oracle_connector.clone(),
            self.calc_connector.clone(),
            self.event_bus.clone(),
        );
        
        loop {
            ticker.tick().await;
            
            match update_flow.execute().await {
                Ok(price) => {
                    let mut state = self.state.write().await;
                    state.current_price = price;
                    state.last_update = chrono::Utc::now();
                    info!("Price updated: ${:.2}", price);
                }
                Err(e) => {
                    warn!("Update flow error: {}", e);
                }
            }
        }
    }

    /// Trading 플로우: 옵션 생성 및 거래
    async fn start_trading_flow(&self) -> Result<()> {
        let trading_flow = TradingFlow::new(
            self.calc_connector.clone(),
            self.contract_connector.clone(),
            self.event_bus.clone(),
        );
        
        // 거래 요청 리스너 (실제로는 API 엔드포인트가 될 것)
        let mut ticker = interval(Duration::from_secs(60));
        
        loop {
            ticker.tick().await;
            
            // 시뮬레이션: 1분마다 새로운 옵션 생성 체크
            let state = self.state.read().await;
            if state.current_price > 0.0 {
                // 실제로는 사용자 요청에 의해 트리거됨
                trading_flow.check_new_options().await;
            }
        }
    }

    /// Settlement 플로우: 만기 시 자동 정산
    async fn start_settlement_flow(&self) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(600)); // 10분마다 체크
        
        loop {
            ticker.tick().await;
            
            let state = self.state.read().await;
            for option_id in &state.active_options {
                // 만기 체크 및 정산 프로세스
                if self.contract_connector.is_expired(option_id).await? {
                    self.event_bus.emit(Event::OptionExpired { 
                        option_id: option_id.clone() 
                    }).await;
                }
            }
        }
    }

    /// 시스템 모니터링
    async fn start_monitoring(&self) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(60));
        
        loop {
            ticker.tick().await;
            
            let state = self.state.read().await;
            info!("System Status:");
            info!("  Current Price: ${:.2}", state.current_price);
            info!("  Last Update: {}", state.last_update);
            info!("  Active Options: {}", state.active_options.len());
            info!("  Pending Settlements: {}", state.pending_settlements.len());
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("BTCFi Oracle VM Orchestrator v1.0");
    info!("=====================================");
    
    let orchestrator = Orchestrator::new().await?;
    
    // 시스템 시작
    orchestrator.start().await?;
    
    Ok(())
}