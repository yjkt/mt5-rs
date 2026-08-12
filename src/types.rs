#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub login: i64,
    pub trade_mode: i64,
    pub leverage: i64,
    pub limit_orders: i64,
    pub margin_so_mode: i64,
    pub trade_allowed: bool,
    pub trade_expert: bool,
    pub margin_mode: i64,
    pub currency_digits: i64,
    pub fifo_close: bool,
    pub balance: f64,
    pub credit: f64,
    pub profit: f64,
    pub equity: f64,
    pub margin: f64,
    pub free_margin: f64,
    pub margin_level: f64,
    pub margin_so_call: f64,
    pub margin_so_so: f64,
    pub margin_initial: f64,
    pub margin_maintenance: f64,
    pub assets: f64,
    pub liabilities: f64,
    pub commission_blocked: f64,
    pub name: String,
    pub server: String,
    pub currency: String,
    pub company: String,
}

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub community_account: bool,
    pub community_connection: bool,
    pub connected: bool,
    pub dlls_allowed: bool,
    pub trade_allowed: bool,
    pub trade_api_disabled: bool,
    pub email_enabled: bool,
    pub ftp_enabled: bool,
    pub notifications_enabled: bool,
    pub mqid: bool,
    pub build: i64,
    pub max_bars: i64,
    pub code_page: i64,
    pub ping_last: i64,
    pub community_balance: f64,
    pub retransmission: f64,
    pub company: String,
    pub name: String,
    pub language: String,
    pub path: String,
    pub data_path: String,
    pub common_data_path: String,
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: i32,
    pub build: i32,
    pub build_date: String,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub custom: bool,
    pub chart_mode: i64,
    pub select: bool,
    pub visible: bool,
    pub session_deals: i64,
    pub session_buy_orders: i64,
    pub session_sell_orders: i64,
    pub volume: i64,
    pub volume_high: i64,
    pub volume_low: i64,
    pub time: i64,
    pub digits: i64,
    pub spread: i64,
    pub spread_float: bool,
    pub ticks_book_depth: i64,
    pub trade_calc_mode: i64,
    pub trade_mode: i64,
    pub start_time: i64,
    pub expiration_time: i64,
    pub trade_stops_level: i64,
    pub trade_freeze_level: i64,
    pub trade_exe_mode: i64,
    pub swap_mode: i64,
    pub swap_rollover3days: i64,
    pub margin_hedged_use_leg: bool,
    pub expiration_mode: i64,
    pub filling_mode: i64,
    pub order_mode: i64,
    pub order_gtc_mode: i64,
    pub option_mode: i64,
    pub option_right: i64,
    pub bid: f64,
    pub bidhigh: f64,
    pub bidlow: f64,
    pub ask: f64,
    pub askhigh: f64,
    pub asklow: f64,
    pub last: f64,
    pub lasthigh: f64,
    pub lastlow: f64,
    pub volume_real: f64,
    pub volumehigh_real: f64,
    pub volumelow_real: f64,
    pub option_strike: f64,
    pub point: f64,
    pub trade_tick_value: f64,
    pub trade_tick_value_profit: f64,
    pub trade_tick_value_loss: f64,
    pub trade_tick_size: f64,
    pub trade_contract_size: f64,
    pub trade_accrued_interest: f64,
    pub trade_face_value: f64,
    pub trade_liquidity_rate: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub volume_limit: f64,
    pub swap_long: f64,
    pub swap_short: f64,
    pub margin_initial: f64,
    pub margin_maintenance: f64,
    pub session_volume: f64,
    pub session_turnover: f64,
    pub session_interest: f64,
    pub session_buy_orders_volume: f64,
    pub session_sell_orders_volume: f64,
    pub session_open: f64,
    pub session_close: f64,
    pub session_aw: f64,
    pub session_price_settlement: f64,
    pub session_price_limit_min: f64,
    pub session_price_limit_max: f64,
    pub margin_hedged: f64,
    pub price_change: f64,
    pub price_volatility: f64,
    pub price_theoretical: f64,
    pub price_greeks_delta: f64,
    pub price_greeks_theta: f64,
    pub price_greeks_gamma: f64,
    pub price_greeks_vega: f64,
    pub price_greeks_rho: f64,
    pub price_greeks_omega: f64,
    pub price_sensitivity: f64,
    pub basis: String,
    pub category: String,
    pub currency_base: String,
    pub currency_profit: String,
    pub currency_margin: String,
    pub bank: String,
    pub description: String,
    pub exchange: String,
    pub formula: String,
    pub isin: String,
    pub name: String,
    pub page: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Tick {
    pub time: i64,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: u64,
    pub time_msc: i64,
    pub flags: u32,
    pub volume_real: f64,
}

#[derive(Debug, Clone)]
pub struct Rate {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub tick_volume: u64,
    pub spread: i32,
    pub real_volume: u64,
}

#[derive(Debug, Clone)]
pub struct TradePosition {
    pub ticket: i64,
    pub time: i64,
    pub time_msc: i64,
    pub time_update: i64,
    pub time_update_msc: i64,
    pub r#type: i32,
    pub magic: i64,
    pub identifier: i64,
    pub reason: i32,
    pub volume: f64,
    pub price_open: f64,
    pub price_current: f64,
    pub price_sl: f64,
    pub price_tp: f64,
    pub commission: f64,
    pub swap: f64,
    pub profit: f64,
    pub symbol: String,
    pub comment: String,
    pub external_id: String,
}

#[derive(Debug, Clone)]
pub struct TradeOrder {
    pub ticket: i64,
    pub time_setup: i64,
    pub time_setup_msc: i64,
    pub time_done: i64,
    pub time_done_msc: i64,
    pub time_expiration: i64,
    pub r#type: i32,
    pub type_time: i32,
    pub type_filling: i32,
    pub state: i32,
    pub magic: i64,
    pub position_id: i64,
    pub position_by_id: i64,
    pub reason: i32,
    pub volume_initial: f64,
    pub volume_current: f64,
    pub price_open: f64,
    pub price_current: f64,
    pub price_sl: f64,
    pub price_tp: f64,
    pub price_stoplimit: f64,
    pub symbol: String,
    pub comment: String,
    pub external_id: String,
}

#[derive(Debug, Clone)]
pub struct TradeDeal {
    pub ticket: i64,
    pub order: i64,
    pub time: i64,
    pub time_msc: i64,
    pub r#type: i32,
    pub entry: i32,
    pub magic: i64,
    pub position_id: i64,
    pub reason: i32,
    pub volume: f64,
    pub price: f64,
    pub commission: f64,
    pub swap: f64,
    pub profit: f64,
    pub fee: f64,
    pub symbol: String,
    pub comment: String,
    pub external_id: String,
}

#[derive(Debug, Clone)]
pub struct BookInfo {
    pub r#type: i64,
    pub price: f64,
    pub volume: i64,
    pub volume_real: f64,
}

/// Trade operation type. Values match Python `mt5.TRADE_ACTION_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TradeAction {
    #[default]
    Deal = 1,
    Pending = 5,
    SLTP = 6,
    Modify = 7,
    Remove = 8,
    CloseBy = 10,
}

/// Order type. Values match Python `mt5.ORDER_TYPE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderType {
    #[default]
    Buy = 0,
    Sell = 1,
    BuyLimit = 2,
    SellLimit = 3,
    BuyStop = 4,
    SellStop = 5,
    BuyStopLimit = 6,
    SellStopLimit = 7,
    CloseBy = 8,
}

/// Order filling mode. Values match Python `mt5.ORDER_FILLING_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderFilling {
    #[default]
    FOK = 0,
    IOC = 1,
    Return = 2,
    BOC = 3,
}

/// Order expiration mode. Values match Python `mt5.ORDER_TIME_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderTime {
    #[default]
    GTC = 0,
    Day = 1,
    Specified = 2,
    SpecDay = 3,
}

/// Trade retcodes (subset of Python `mt5.TRADE_RETCODE_*`).
pub mod retcodes {
    pub const REQUOTE: u32 = 10004;
    pub const REJECT: u32 = 10006;
    pub const CANCEL: u32 = 10007;
    pub const PLACED: u32 = 10008;
    pub const DONE: u32 = 10009;
    pub const DONE_PARTIAL: u32 = 10010;
    pub const ERROR: u32 = 10011;
    pub const TIMEOUT: u32 = 10012;
    pub const INVALID: u32 = 10013;
    pub const INVALID_VOLUME: u32 = 10014;
    pub const INVALID_PRICE: u32 = 10015;
    pub const INVALID_STOPS: u32 = 10016;
    pub const TRADE_DISABLED: u32 = 10017;
    pub const MARKET_CLOSED: u32 = 10018;
    pub const NO_MONEY: u32 = 10019;
    pub const PRICE_CHANGED: u32 = 10020;
    pub const PRICE_OFF: u32 = 10021;
    pub const INVALID_EXPIRATION: u32 = 10022;
    pub const ORDER_CHANGED: u32 = 10023;
    pub const TOO_MANY_REQUESTS: u32 = 10024;
    pub const NO_CHANGES: u32 = 10025;
    pub const SERVER_DISABLES_AT: u32 = 10026;
    pub const CLIENT_DISABLES_AT: u32 = 10027;
    pub const LOCKED: u32 = 10028;
    pub const FROZEN: u32 = 10029;
    pub const INVALID_FILL: u32 = 10030;
    pub const CONNECTION: u32 = 10031;
    pub const ONLY_REAL: u32 = 10032;
    pub const LIMIT_ORDERS: u32 = 10033;
    pub const INVALID_VOLUME2: u32 = 10034;
    pub const INVALID_PRICE2: u32 = 10035;
    pub const INVALID_STOPS2: u32 = 10036;
    pub const INVALID_TRADE_VOLUME: u32 = 10037;
    pub const MARKET_CLOSED2: u32 = 10038;
    pub const TRADE_DISABLED2: u32 = 10039;
    pub const NOT_ENOUGH_MARGIN: u32 = 10040;
    pub const NOT_ENOUGH_MONEY: u32 = 10041;
    pub const VIDEO_CARD_ERR: u32 = 10042;
    pub const ORDER_LOCKED: u32 = 10043;
    pub const LONG_ONLY: u32 = 10044;
    pub const SHORT_ONLY: u32 = 10045;
    pub const CLOSE_ONLY: u32 = 10046;
    pub const INVALID_STATE: u32 = 10047;
    pub const LIMIT_ORDERS2: u32 = 10048;
    pub const INVALID_FILL2: u32 = 10049;
    pub const INVALID_EXPIRATION2: u32 = 10050;
    pub const NO_EXPERT: u32 = 10051;
    pub const ONLY_REAL2: u32 = 10052;
    pub const TRADE_HEDGE_PROHIBITED: u32 = 10053;
    pub const TRADE_EXPERT_DISABLED: u32 = 10054;
}

/// Trading request, mirroring Python `mt5.TradeRequest`.
#[derive(Debug, Clone, Default)]
pub struct TradeRequest {
    pub action: TradeAction,
    pub magic: i64,
    pub order: i64,
    pub symbol: String,
    pub volume: f64,
    pub price: f64,
    pub stoplimit: f64,
    pub sl: f64,
    pub tp: f64,
    pub deviation: u64,
    pub r#type: OrderType,
    pub type_filling: OrderFilling,
    pub type_time: OrderTime,
    pub expiration: i64,
    pub comment: String,
    pub position: i64,
    pub position_by: i64,
}

impl TradeRequest {
    pub fn new(action: TradeAction, symbol: &str, volume: f64, r#type: OrderType) -> Self {
        Self {
            action,
            magic: 0,
            order: 0,
            symbol: symbol.to_string(),
            volume,
            price: 0.0,
            stoplimit: 0.0,
            sl: 0.0,
            tp: 0.0,
            deviation: 0,
            r#type,
            type_filling: OrderFilling::Return,
            type_time: OrderTime::GTC,
            expiration: 0,
            comment: String::new(),
            position: 0,
            position_by: 0,
        }
    }
}

/// Result of `order_check`, mirroring Python `mt5.TradeCheckResult`.
#[derive(Debug, Clone, Default)]
pub struct TradeCheckResult {
    pub retcode: u32,
    pub balance: f64,
    pub equity: f64,
    pub profit: f64,
    pub margin: f64,
    pub margin_free: f64,
    pub margin_level: f64,
    pub comment: String,
}

impl TradeCheckResult {
    pub fn is_ok(&self) -> bool {
        self.retcode == 0 || self.retcode == retcodes::DONE
    }
}

/// Result of `order_send`, mirroring Python `mt5.TradeResult`.
#[derive(Debug, Clone, Default)]
pub struct TradeResult {
    pub retcode: u32,
    pub deal: i64,
    pub order: i64,
    pub volume: f64,
    pub price: f64,
    pub bid: f64,
    pub ask: f64,
    pub comment: String,
    pub request_id: u32,
    pub retcode_ext: i32,
}

impl TradeResult {
    pub fn is_ok(&self) -> bool {
        matches!(
            self.retcode,
            retcodes::PLACED | retcodes::DONE | retcodes::DONE_PARTIAL
        )
    }
}
