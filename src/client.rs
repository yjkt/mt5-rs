use crate::error::{Mt5Error, Result};
use crate::protocol::{NamedPipeClient, Transport};
use crate::types::*;

pub struct Mt5Client {
    transport: Option<Box<dyn Transport>>,
    build: i32,
    last_error: std::sync::Mutex<(i32, String)>,
}

impl Default for Mt5Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Mt5Client {
    pub fn new() -> Self {
        Self {
            transport: None,
            build: 0,
            last_error: std::sync::Mutex::new((0, String::new())),
        }
    }

    /// Construct a client over a custom transport (used by tests with an
    /// in-memory mock pipe; mirror of go-mt5's `NewClientFromConn`).
    pub fn from_transport(transport: Box<dyn Transport>) -> Self {
        Self {
            transport: Some(transport),
            build: 0,
            last_error: std::sync::Mutex::new((0, String::new())),
        }
    }

    pub fn initialize(&mut self, pipe_name: Option<&str>) -> Result<()> {
        self.transport = Some(Box::new(NamedPipeClient::new(pipe_name)?));

        let transport = self.transport()?;
        let mut data = Vec::new();
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&encode_string("Go"));

        let resp = transport.send(4, &data)?;
        if resp.len() >= 4 {
            let build = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
            self.build = build as i32;
        }

        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.transport = None;
    }

    fn transport(&self) -> Result<&dyn Transport> {
        self.transport.as_deref().ok_or(Mt5Error::NotInitialized)
    }

    /// Send a command, recording the last error for `last_error()`.
    fn send(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>> {
        match self.transport()?.send(cmd, data) {
            Ok(resp) => Ok(resp),
            Err(e) => {
                *self.last_error.lock().unwrap() = (-1, e.to_string());
                Err(e)
            }
        }
    }

    pub fn login(&self, login: i64, password: &str, server: &str) -> Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&login.to_le_bytes());
        data.extend_from_slice(&encode_string(password));
        data.extend_from_slice(&encode_string(server));

        // CmdLogin = 100 (bukan 4 yang dipakai initialize).
        let resp = self.send(100, &data)?;
        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let status = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        if status != 0 {
            return Err(Mt5Error::CommandFailed {
                cmd: 100,
                error: format!("Login failed with status: {}", status),
            });
        }

        Ok(())
    }

    pub fn account_info(&self) -> Result<AccountInfo> {
        let resp = self.send(190, &[])?;
        decode_account_info(&resp)
    }
}

/// Decode the account_info response payload (cmd 190).
///
/// Byte layout verified against the real capture in testdata/account_info.bin
/// (login i64, then 139-byte numeric block, then 4 fixed-width UTF-16LE
/// strings: name 256, server 128, currency 64, company 256).
fn decode_account_info(data: &[u8]) -> Result<AccountInfo> {
    if data.len() < 8 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);

    // 按照 Python 输出和二进制数据验证的精确位置解析
    // Pos 0-7: login (i64)
    let login = reader.read_i64();

    // Pos 8-11: trade_mode (i32)
    let trade_mode = reader.read_i32() as i64;

    // Pos 12-15: leverage (i32)
    let leverage = reader.read_i32() as i64;

    // Pos 16-19: limit_orders (i32)
    let limit_orders = reader.read_i32() as i64;

    // Pos 20-23: margin_so_mode (i32)
    let margin_so_mode = reader.read_i32() as i64;

    // Pos 24: trade_allowed (bool, 1字节)
    let trade_allowed = reader.read_bool1();

    // Pos 25: trade_expert (bool, 1字节)
    let trade_expert = reader.read_bool1();

    // Pos 26-29: margin_mode (i32)
    let margin_mode = reader.read_i32() as i64;

    // Pos 30-33: currency_digits (i32)
    let currency_digits = reader.read_i32() as i64;

    // Pos 34: fifo_close (bool, 1字节)
    let fifo_close = reader.read_bool1();

    // Pos 35-42: balance (f64)
    let balance = reader.read_f64();

    // Pos 43-50: credit (f64)
    let credit = reader.read_f64();

    // Pos 51-58: profit (f64)
    let profit = reader.read_f64();

    // Pos 59-66: equity (f64)
    let equity = reader.read_f64();

    // Pos 67-74: margin (f64)
    let margin = reader.read_f64();

    // Pos 75-82: margin_free (f64)
    let free_margin = reader.read_f64();

    // Pos 83-90: margin_level (f64)
    let margin_level = reader.read_f64();

    // Pos 91-98: margin_so_call (f64)
    let margin_so_call = reader.read_f64();

    // Pos 99-106: margin_so_so (f64)
    let margin_so_so = reader.read_f64();

    // Pos 107-114: margin_initial (f64)
    let margin_initial = reader.read_f64();

    // Pos 115-122: margin_maintenance (f64)
    let margin_maintenance = reader.read_f64();

    // Pos 123-130: assets (f64)
    let assets = reader.read_f64();

    // Pos 131-138: liabilities (f64)
    let liabilities = reader.read_f64();

    // Pos 139-146: commission_blocked (f64)
    let commission_blocked = reader.read_f64();

    // 读取字符串字段 (从 pos 147 开始)
    let strings_offset = 147;
    if strings_offset >= data.len() {
        return Err(Mt5Error::InvalidResponse(format!(
            "Response too short for strings: {} < {}",
            data.len(),
            strings_offset
        )));
    }

    let mut sr = Reader::new(&data[strings_offset..]);
    let name = sr.read_fixed_string(256);
    let server = sr.read_fixed_string(128);
    let currency = sr.read_fixed_string(64);
    let company = sr.read_fixed_string(256);

    if sr.has_error() {
        return Err(Mt5Error::InvalidResponse("Failed to read strings".into()));
    }

    Ok(AccountInfo {
        login,
        trade_mode,
        leverage,
        limit_orders,
        margin_so_mode,
        trade_allowed,
        trade_expert,
        margin_mode,
        currency_digits,
        fifo_close,
        balance,
        credit,
        profit,
        equity,
        margin,
        free_margin,
        margin_level,
        margin_so_call,
        margin_so_so,
        margin_initial,
        margin_maintenance,
        assets,
        liabilities,
        commission_blocked,
        name,
        server,
        currency,
        company,
    })
}

fn decode_symbol_info(reader: &mut Reader) -> Result<SymbolInfo> {
    // 严格按照 go-mt5 decodeSymbolInfo 的字段顺序和类型解析
    // 参考：https://github.com/Mukbeast4/go-mt5/blob/main/symbols.go
    let custom = reader.read_bool1();
    let chart_mode = reader.read_u32() as i64;
    let select = reader.read_bool1();
    let visible = reader.read_bool1();
    let session_deals = reader.read_i64();
    let session_buy_orders = reader.read_i64();
    let session_sell_orders = reader.read_i64();
    let volume = reader.read_i64();
    let volume_high = reader.read_i64();
    let volume_low = reader.read_i64();
    let time = reader.read_i64();
    let digits = reader.read_u32() as i64;
    let spread = reader.read_u32() as i64;
    let spread_float = reader.read_bool1();
    let ticks_book_depth = reader.read_u32() as i64;
    let trade_calc_mode = reader.read_u32() as i64;
    let trade_mode = reader.read_u32() as i64;
    let start_time = reader.read_i64();
    let expiration_time = reader.read_i64();
    let trade_stops_level = reader.read_u32() as i64;
    let trade_freeze_level = reader.read_u32() as i64;
    let trade_exe_mode = reader.read_u32() as i64;
    let swap_mode = reader.read_u32() as i64;
    let swap_rollover3days = reader.read_u32() as i64;
    let margin_hedged_use_leg = reader.read_bool1();
    let expiration_mode = reader.read_u32() as i64;
    let filling_mode = reader.read_u32() as i64;
    let order_mode = reader.read_u32() as i64;
    let order_gtc_mode = reader.read_u32() as i64;
    let option_mode = reader.read_u32() as i64;
    let option_right = reader.read_u32() as i64;
    let bid = reader.read_f64();
    let bid_high = reader.read_f64();
    let bid_low = reader.read_f64();
    let ask = reader.read_f64();
    let ask_high = reader.read_f64();
    let ask_low = reader.read_f64();
    let last = reader.read_f64();
    let last_high = reader.read_f64();
    let last_low = reader.read_f64();
    let volume_real = reader.read_f64();
    let volume_high_real = reader.read_f64();
    let volume_low_real = reader.read_f64();
    let option_strike = reader.read_f64();
    let point = reader.read_f64();
    let trade_tick_value = reader.read_f64();
    let trade_tick_value_profit = reader.read_f64();
    let trade_tick_value_loss = reader.read_f64();
    let trade_tick_size = reader.read_f64();
    let trade_contract_size = reader.read_f64();
    let trade_accrued_interest = reader.read_f64();
    let trade_face_value = reader.read_f64();
    let trade_liquidity_rate = reader.read_f64();
    let volume_min = reader.read_f64();
    let volume_max = reader.read_f64();
    let volume_step = reader.read_f64();
    let volume_limit = reader.read_f64();
    let swap_long = reader.read_f64();
    let swap_short = reader.read_f64();
    let margin_initial = reader.read_f64();
    let margin_maintenance = reader.read_f64();
    let session_volume = reader.read_f64();
    let session_turnover = reader.read_f64();
    let session_interest = reader.read_f64();
    let session_buy_orders_volume = reader.read_f64();
    let session_sell_orders_volume = reader.read_f64();
    let session_open = reader.read_f64();
    let session_close = reader.read_f64();
    let session_aw = reader.read_f64();
    let session_price_settlement = reader.read_f64();
    let session_price_limit_min = reader.read_f64();
    let session_price_limit_max = reader.read_f64();
    let margin_hedged = reader.read_f64();
    let price_change = reader.read_f64();
    let price_volatility = reader.read_f64();
    let price_theoretical = reader.read_f64();
    let price_greeks_delta = reader.read_f64();
    let price_greeks_theta = reader.read_f64();
    let price_greeks_gamma = reader.read_f64();
    let price_greeks_vega = reader.read_f64();
    let price_greeks_rho = reader.read_f64();
    let price_greeks_omega = reader.read_f64();
    let price_sensitivity = reader.read_f64();

    // 字符串字段：固定宽度 UTF-16LE 槽（go-mt5 PR#3 验证）
    // 总字符串区域 = 2432 字节
    let basis = reader.read_fixed_string(64);
    let category = reader.read_fixed_string(128);
    let currency_base = reader.read_fixed_string(32);
    let currency_profit = reader.read_fixed_string(32);
    let currency_margin = reader.read_fixed_string(32);
    let bank = reader.read_fixed_string(512);
    let description = reader.read_fixed_string(64);
    let exchange = reader.read_fixed_string(64);
    let formula = reader.read_fixed_string(1024);
    let isin = reader.read_fixed_string(32);
    let page = reader.read_fixed_string(128);
    let path = reader.read_fixed_string(256);
    let symbol_name = reader.read_fixed_string(64);

    if reader.has_error() {
        return Err(Mt5Error::InvalidResponse(
            "Failed to read symbol info".into(),
        ));
    }

    Ok(SymbolInfo {
        custom,
        chart_mode,
        select,
        visible,
        session_deals,
        session_buy_orders,
        session_sell_orders,
        volume,
        volume_high,
        volume_low,
        time,
        digits,
        spread,
        spread_float,
        ticks_book_depth,
        trade_calc_mode,
        trade_mode,
        start_time,
        expiration_time,
        trade_stops_level,
        trade_freeze_level,
        trade_exe_mode,
        swap_mode,
        swap_rollover3days,
        margin_hedged_use_leg,
        expiration_mode,
        filling_mode,
        order_mode,
        order_gtc_mode,
        option_mode,
        option_right,
        bid,
        bidhigh: bid_high,
        bidlow: bid_low,
        ask,
        askhigh: ask_high,
        asklow: ask_low,
        last,
        lasthigh: last_high,
        lastlow: last_low,
        volume_real,
        volumehigh_real: volume_high_real,
        volumelow_real: volume_low_real,
        option_strike,
        point,
        trade_tick_value,
        trade_tick_value_profit,
        trade_tick_value_loss,
        trade_tick_size,
        trade_contract_size,
        trade_accrued_interest,
        trade_face_value,
        trade_liquidity_rate,
        volume_min,
        volume_max,
        volume_step,
        volume_limit,
        swap_long,
        swap_short,
        margin_initial,
        margin_maintenance,
        session_volume,
        session_turnover,
        session_interest,
        session_buy_orders_volume,
        session_sell_orders_volume,
        session_open,
        session_close,
        session_aw,
        session_price_settlement,
        session_price_limit_min,
        session_price_limit_max,
        margin_hedged,
        price_change,
        price_volatility,
        price_theoretical,
        price_greeks_delta,
        price_greeks_theta,
        price_greeks_gamma,
        price_greeks_vega,
        price_greeks_rho,
        price_greeks_omega,
        price_sensitivity,
        basis,
        category,
        currency_base,
        currency_profit,
        currency_margin,
        bank,
        description,
        exchange,
        formula,
        isin,
        name: symbol_name,
        page,
        path,
    })
}
impl Mt5Client {
    pub fn terminal_info(&self) -> Result<TerminalInfo> {
        let resp = self.send(180, &[])?;

        if resp.len() < 40 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let community_account = resp[2] != 0;
        let community_connection = resp[3] != 0;
        let connected = resp[6] != 0;
        let dlls_allowed = resp[7] != 0;
        let trade_allowed = resp[8] != 0;
        let trade_api_disabled = resp[9] != 0;
        let email_enabled = resp[10] != 0;
        let ftp_enabled = resp[11] != 0;
        let notifications_enabled = resp[4] != 0;
        let mqid = resp[5] != 0;

        let build = u16::from_le_bytes([resp[0], resp[1]]) as i64;
        let max_bars = u32::from_le_bytes([resp[12], resp[13], resp[14], resp[15]]) as i64;
        let code_page = u16::from_le_bytes([resp[17], resp[18]]) as i64;
        let ping_last = u16::from_le_bytes([resp[21], resp[22]]) as i64;
        let community_balance = f64::from_le_bytes([
            resp[24], resp[25], resp[26], resp[27], resp[28], resp[29], resp[30], resp[31],
        ]);
        let retransmission = f64::from_le_bytes([
            resp[32], resp[33], resp[34], resp[35], resp[36], resp[37], resp[38], resp[39],
        ]);

        let company = read_string_at_offset(&resp, 41);
        let name = read_string_at_offset(&resp, 561);
        let language = read_string_at_offset(&resp, 1081);
        let path = read_string_at_offset(&resp, 1601);
        let data_path = read_string_at_offset(&resp, 2121);
        let common_data_path = read_string_at_offset(&resp, 2641);

        Ok(TerminalInfo {
            community_account,
            community_connection,
            connected,
            dlls_allowed,
            trade_allowed,
            trade_api_disabled,
            email_enabled,
            ftp_enabled,
            notifications_enabled,
            mqid,
            build,
            max_bars,
            code_page,
            ping_last,
            community_balance,
            retransmission,
            company,
            name,
            language,
            path,
            data_path,
            common_data_path,
        })
    }

    /// Version info. `build` is read from the terminal (cmd 180). The Python
    /// library reports API version 500 and a build date from cmd 1, but MT5
    /// builds >= 5836 close the pipe for cmd 1 — so `version` always reports
    /// API 500 and leaves `build_date` empty rather than fabricating it.
    pub fn version(&self) -> Result<VersionInfo> {
        let info = self.terminal_info()?;
        Ok(VersionInfo {
            version: 500,
            build: info.build as i32,
            build_date: String::new(),
        })
    }

    pub fn symbols_total(&self) -> Result<i64> {
        let resp = self.send(173, &[])?;

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let total = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(total as i64)
    }

    pub fn symbols_get(&self) -> Result<Vec<SymbolInfo>> {
        let resp = self.send(174, &[])?;

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let mut reader = Reader::new(&resp);
        let count = reader.read_u32() as usize;

        let mut symbols = Vec::with_capacity(count);

        for _ in 0..count {
            let sym = decode_symbol_info(&mut reader)?;
            symbols.push(sym);
        }

        Ok(symbols)
    }

    pub fn symbol_info(&self, symbol: &str) -> Result<Option<SymbolInfo>> {
        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));

        let resp = self.send(170, &data)?;

        if resp.is_empty() {
            return Ok(None);
        }

        let mut reader = Reader::new(&resp);
        let info = decode_symbol_info(&mut reader)?;
        Ok(Some(info))
    }

    pub fn symbol_info_tick(&self, symbol: &str) -> Result<Option<Tick>> {
        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));

        let resp = self.send(172, &data)?;

        if resp.is_empty() {
            return Ok(None);
        }

        Ok(Some(decode_tick(&resp)?))
    }

    pub fn symbol_select(&self, symbol: &str, enable: bool) -> Result<bool> {
        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));
        data.push(if enable { 1u8 } else { 0u8 });

        let resp = self.send(171, &data)?;

        // 空响应表示成功（MT5只返回8字节的头部，没有额外数据）
        if resp.is_empty() {
            return Ok(true);
        }

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let status = i32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(status != 0)
    }

    pub fn positions_total(&self) -> Result<i64> {
        let resp = self.send(120, &[])?;

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let total = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(total as i64)
    }

    pub fn orders_total(&self) -> Result<i64> {
        let resp = self.send(130, &[])?;

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let total = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(total as i64)
    }

    pub fn positions_get(&self, symbol: Option<&str>) -> Result<Vec<TradePosition>> {
        let cmd = 121;

        let mut data = Vec::new();
        if let Some(sym) = symbol {
            data.extend_from_slice(&encode_string(sym));
        }

        let resp = self.send(cmd, &data)?;
        parse_positions_response(&resp)
    }

    pub fn orders_get(&self, symbol: Option<&str>) -> Result<Vec<TradeOrder>> {
        let cmd = 131;

        let mut data = Vec::new();
        if let Some(sym) = symbol {
            data.extend_from_slice(&encode_string(sym));
        }

        let resp = self.send(cmd, &data)?;
        parse_orders_response(&resp)
    }

    pub fn send_raw_command(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>> {
        self.send(cmd, data)
    }

    /// Return the last command error as `(code, message)`.
    ///
    /// The Python library reads this from its DLL; go-mt5 keeps it in client
    /// state. Sending a dedicated IPC command for this is not possible on
    /// modern MT5 builds (cmd 3 closes the pipe), so mt5-rs records the last
    /// error from any failed `send` in client state.
    pub fn last_error(&self) -> Result<(i32, String)> {
        Ok(self.last_error.lock().unwrap().clone())
    }

    pub fn copy_rates_from_pos(
        &self,
        symbol: &str,
        timeframe: i32,
        start_pos: i64,
        count: i32,
    ) -> Result<Vec<Rate>> {
        // 根据go-mt5源码，命令代码108，参数使用u32编码
        let cmd = 108;

        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));
        data.extend_from_slice(&(timeframe as u32).to_le_bytes());
        data.extend_from_slice(&(start_pos as u32).to_le_bytes());
        data.extend_from_slice(&(count as u32).to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_rates_response(&resp)
    }

    pub fn copy_rates_from(
        &self,
        symbol: &str,
        timeframe: i32,
        date_from: i64,
        count: i32,
    ) -> Result<Vec<Rate>> {
        let cmd = 106;

        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));
        data.extend_from_slice(&(timeframe as u32).to_le_bytes());
        data.extend_from_slice(&date_from.to_le_bytes());
        data.extend_from_slice(&(count as u32).to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_rates_response(&resp)
    }

    pub fn copy_rates_range(
        &self,
        symbol: &str,
        timeframe: i32,
        date_from: i64,
        date_to: i64,
    ) -> Result<Vec<Rate>> {
        let cmd = 107;

        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));
        data.extend_from_slice(&(timeframe as u32).to_le_bytes());
        data.extend_from_slice(&date_from.to_le_bytes());
        data.extend_from_slice(&date_to.to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_rates_response(&resp)
    }

    pub fn copy_ticks_from(
        &self,
        symbol: &str,
        from: i64,
        count: i32,
        flags: i32,
    ) -> Result<Vec<Tick>> {
        let cmd = 104;

        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));
        data.extend_from_slice(&from.to_le_bytes());
        data.extend_from_slice(&(count as u32).to_le_bytes());
        data.extend_from_slice(&(flags as u32).to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_ticks_response(&resp)
    }

    pub fn copy_ticks_range(
        &self,
        symbol: &str,
        from: i64,
        to: i64,
        flags: i32,
    ) -> Result<Vec<Tick>> {
        let cmd = 105;

        let mut data = Vec::new();
        data.extend_from_slice(&encode_string(symbol));
        data.extend_from_slice(&from.to_le_bytes());
        data.extend_from_slice(&to.to_le_bytes());
        data.extend_from_slice(&(flags as u32).to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_ticks_response(&resp)
    }

    pub fn history_deals_total(&self, from: i64, to: i64) -> Result<i64> {
        let cmd = 150;

        let mut data = Vec::new();
        data.extend_from_slice(&from.to_le_bytes());
        data.extend_from_slice(&to.to_le_bytes());

        let resp = self.send(cmd, &data)?;
        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let total = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(total as i64)
    }

    pub fn history_deals_get(&self, from: i64, to: i64) -> Result<Vec<TradeDeal>> {
        let cmd = 151;

        let mut data = Vec::new();
        data.extend_from_slice(&from.to_le_bytes());
        data.extend_from_slice(&to.to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_deals_response(&resp)
    }

    pub fn history_orders_total(&self, from: i64, to: i64) -> Result<i64> {
        let cmd = 140;

        let mut data = Vec::new();
        data.extend_from_slice(&from.to_le_bytes());
        data.extend_from_slice(&to.to_le_bytes());

        let resp = self.send(cmd, &data)?;
        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let total = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(total as i64)
    }

    pub fn history_orders_get(&self, from: i64, to: i64) -> Result<Vec<TradeOrder>> {
        let cmd = 141;

        let mut data = Vec::new();
        data.extend_from_slice(&from.to_le_bytes());
        data.extend_from_slice(&to.to_le_bytes());

        let resp = self.send(cmd, &data)?;
        parse_orders_response(&resp)
    }

    pub fn market_book_add(&self, symbol: &str) -> Result<bool> {
        let cmd = 191;

        let data = encode_string(symbol);
        let resp = self.send(cmd, &data)?;

        if resp.is_empty() {
            return Ok(true);
        }

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let status = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(status == 0)
    }

    pub fn market_book_get(&self, symbol: &str) -> Result<Vec<BookInfo>> {
        let cmd = 193;

        let data = encode_string(symbol);
        let resp = self.send(cmd, &data)?;
        parse_book_response(&resp)
    }

    pub fn market_book_release(&self, symbol: &str) -> Result<bool> {
        let cmd = 192;

        let data = encode_string(symbol);
        let resp = self.send(cmd, &data)?;

        if resp.is_empty() {
            return Ok(true);
        }

        if resp.len() < 4 {
            return Err(Mt5Error::InvalidResponse("Response too short".into()));
        }

        let status = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        Ok(status == 0)
    }

    /// Calculate required margin for a trade (local calculation, no IPC).
    ///
    /// Uses `margin_initial` from symbol info when the broker provides it
    /// (margin = volume × price × margin_initial / 4). Some brokers return
    /// `margin_initial = 0` (Elev8 demo returns 0); in that case fall back to
    /// `volume × price × trade_contract_size / leverage` from account info,
    /// which matches the terminal's own margin within broker rounding.
    ///
    /// Note: this is an approximation. For the exact margin the terminal would
    /// reserve, use `order_check`, whose response includes the real margin.
    pub fn order_calc_margin(
        &self,
        _action: i32,
        symbol: &str,
        volume: f64,
        price: f64,
    ) -> Result<f64> {
        let symbol_info = self
            .symbol_info(symbol)?
            .ok_or_else(|| Mt5Error::CommandFailed {
                cmd: 0,
                error: format!("symbol not found: {symbol}"),
            })?;

        if symbol_info.margin_initial > 0.0 {
            return Ok(volume * price * symbol_info.margin_initial / 4.0);
        }

        // Broker tidak menyediakan margin_initial: fallback berbasis leverage.
        let account = self.account_info()?;
        let leverage = if account.leverage > 0 {
            account.leverage as f64
        } else {
            100.0
        };
        let margin = volume * price * symbol_info.trade_contract_size / leverage;
        Ok(margin)
    }

    /// Calculate expected profit for a trade (local calculation, no IPC).
    /// profit = volume × (price_close - price_open) × trade_contract_size
    pub fn order_calc_profit(
        &self,
        _action: i32,
        symbol: &str,
        volume: f64,
        price_open: f64,
        price_close: f64,
    ) -> Result<f64> {
        let symbol_info = self
            .symbol_info(symbol)?
            .ok_or_else(|| Mt5Error::CommandFailed {
                cmd: 0,
                error: format!("symbol not found: {symbol}"),
            })?;

        let profit = volume * (price_close - price_open) * symbol_info.trade_contract_size;
        Ok(profit)
    }

    /// Check whether a trade request is valid without placing it (Python `mt5.order_check`).
    /// Wire format verified against the real terminal (build 6090) and go-mt5 fixtures.
    pub fn order_check(&self, request: &TradeRequest) -> Result<TradeCheckResult> {
        let data = encode_trade_request(request);
        let resp = self.send(CMD_ORDER_CHECK, &data)?;
        decode_check_result(&resp)
    }

    /// Send a trade request to the terminal for execution (Python `mt5.order_send`).
    /// Same request encoding as `order_check`; decodes the 260-byte trade result.
    pub fn order_send(&self, request: &TradeRequest) -> Result<TradeResult> {
        let data = encode_trade_request(request);
        let resp = self.send(CMD_ORDER_SEND, &data)?;
        decode_trade_result(&resp)
    }
}

const CMD_ORDER_CHECK: u32 = 160;
const CMD_ORDER_SEND: u32 = 161;

const TRADE_REQUEST_SYMBOL_SLOT: usize = 64;
const TRADE_REQUEST_COMMENT_SLOT: usize = 64;
const TRADE_REQUEST_TOTAL: usize = 232;

const CHECK_RESULT_COMMENT_SLOT: usize = 200;
const CHECK_RESULT_TOTAL: usize = 252;

const TRADE_RESULT_COMMENT_SLOT: usize = 200;
const TRADE_RESULT_TOTAL: usize = 260;

fn encode_fixed_string(slot: &mut [u8], s: &str) {
    for (i, c) in s.encode_utf16().enumerate() {
        if i * 2 + 1 < slot.len() {
            slot[i * 2] = c.to_le_bytes()[0];
            slot[i * 2 + 1] = c.to_le_bytes()[1];
        }
    }
}

/// Encode a TradeRequest to the exact 232-byte wire layout used by the official
/// Python MetaTrader5 library (offsets verified by go-mt5 tests):
///   action(4) magic(8) order(8) symbol(64) volume(8) price(8) stoplimit(8)
///   sl(8) tp(8) deviation(8) type(4) filling(4) time(4) expiration(8)
///   comment(64) position(8) position_by(8)
fn encode_trade_request(request: &TradeRequest) -> Vec<u8> {
    let mut w = Vec::with_capacity(TRADE_REQUEST_TOTAL);
    w.extend_from_slice(&(request.action as u32).to_le_bytes());
    w.extend_from_slice(&request.magic.to_le_bytes());
    w.extend_from_slice(&request.order.to_le_bytes());
    let mut sym = vec![0u8; TRADE_REQUEST_SYMBOL_SLOT];
    encode_fixed_string(&mut sym, &request.symbol);
    w.extend_from_slice(&sym);
    w.extend_from_slice(&request.volume.to_le_bytes());
    w.extend_from_slice(&request.price.to_le_bytes());
    w.extend_from_slice(&request.stoplimit.to_le_bytes());
    w.extend_from_slice(&request.sl.to_le_bytes());
    w.extend_from_slice(&request.tp.to_le_bytes());
    w.extend_from_slice(&request.deviation.to_le_bytes());
    w.extend_from_slice(&(request.r#type as u32).to_le_bytes());
    w.extend_from_slice(&(request.type_filling as u32).to_le_bytes());
    w.extend_from_slice(&(request.type_time as u32).to_le_bytes());
    w.extend_from_slice(&request.expiration.to_le_bytes());
    let mut cmt = vec![0u8; TRADE_REQUEST_COMMENT_SLOT];
    encode_fixed_string(&mut cmt, &request.comment);
    w.extend_from_slice(&cmt);
    w.extend_from_slice(&request.position.to_le_bytes());
    w.extend_from_slice(&request.position_by.to_le_bytes());

    debug_assert_eq!(
        w.len(),
        TRADE_REQUEST_TOTAL,
        "trade request must be 232 bytes"
    );
    w
}

fn decode_fixed_string(data: &[u8], start: usize, slot_bytes: usize) -> Result<String> {
    if start + slot_bytes > data.len() {
        return Err(Mt5Error::InvalidResponse(format!(
            "string slot out of bounds: start={start} slot={slot_bytes} len={}",
            data.len()
        )));
    }
    let mut chars = Vec::with_capacity(slot_bytes / 2);
    let mut i = start;
    while i + 1 < start + slot_bytes {
        let c = u16::from_le_bytes([data[i], data[i + 1]]);
        if c == 0 {
            break;
        }
        chars.push(c);
        i += 2;
    }
    Ok(String::from_utf16_lossy(&chars))
}

/// Decode the 252-byte order_check response:
///   retcode(4) balance(8) equity(8) profit(8) margin(8) margin_free(8)
///   margin_level(8) comment(200)
fn decode_check_result(data: &[u8]) -> Result<TradeCheckResult> {
    if data.len() < CHECK_RESULT_TOTAL {
        return Err(Mt5Error::InvalidResponse(format!(
            "check result too short: {} bytes (want {})",
            data.len(),
            CHECK_RESULT_TOTAL
        )));
    }
    let read_f64 = |off: usize| f64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    let retcode = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let comment = decode_fixed_string(data, 52, CHECK_RESULT_COMMENT_SLOT)?;
    Ok(TradeCheckResult {
        retcode,
        balance: read_f64(4),
        equity: read_f64(12),
        profit: read_f64(20),
        margin: read_f64(28),
        margin_free: read_f64(36),
        margin_level: read_f64(44),
        comment,
    })
}

/// Decode the 260-byte order_send response:
///   retcode(4) deal(8) order(8) volume(8) price(8) bid(8) ask(8)
///   comment(200) request_id(4) retcode_ext(4)
fn decode_trade_result(data: &[u8]) -> Result<TradeResult> {
    if data.len() < TRADE_RESULT_TOTAL {
        return Err(Mt5Error::InvalidResponse(format!(
            "trade result too short: {} bytes (want {})",
            data.len(),
            TRADE_RESULT_TOTAL
        )));
    }
    let read_f64 = |off: usize| f64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    let retcode = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let deal = i64::from_le_bytes(data[4..12].try_into().unwrap());
    let order = i64::from_le_bytes(data[12..20].try_into().unwrap());
    let comment = decode_fixed_string(data, 52, TRADE_RESULT_COMMENT_SLOT)?;
    let request_id = u32::from_le_bytes(data[252..256].try_into().unwrap());
    let retcode_ext = i32::from_le_bytes(data[256..260].try_into().unwrap());
    Ok(TradeResult {
        retcode,
        deal,
        order,
        volume: read_f64(20),
        price: read_f64(28),
        bid: read_f64(36),
        ask: read_f64(44),
        comment,
        request_id,
        retcode_ext,
    })
}

fn parse_positions_response(data: &[u8]) -> Result<Vec<TradePosition>> {
    if data.len() < 4 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);
    let count = reader.read_u32() as usize;

    let mut positions = Vec::with_capacity(count);

    for _ in 0..count {
        // Wire layout (go-mt5 decodePositions, verified against a real capture):
        //   ticket time time_msc time_update time_update_msc type magic
        //   identifier reason volume price_open price_sl price_tp
        //   price_current commission swap profit symbol comment external_id
        let ticket = reader.read_i64();
        let time = reader.read_i64();
        let time_msc = reader.read_i64();
        let time_update = reader.read_i64();
        let time_update_msc = reader.read_i64();
        let r#type = reader.read_u32() as i32;
        let magic = reader.read_i64();
        let identifier = reader.read_i64();
        let reason = reader.read_u32() as i32;
        let volume = reader.read_f64();
        let price_open = reader.read_f64();
        let price_sl = reader.read_f64();
        let price_tp = reader.read_f64();
        let price_current = reader.read_f64();
        let commission = reader.read_f64();
        let swap = reader.read_f64();
        let profit = reader.read_f64();
        let symbol = reader.read_fixed_string(64);
        let comment = reader.read_fixed_string(64);
        let external_id = reader.read_fixed_string(64);

        if reader.has_error() {
            break;
        }

        positions.push(TradePosition {
            ticket,
            time,
            time_msc,
            time_update,
            time_update_msc,
            r#type,
            magic,
            identifier,
            reason,
            volume,
            price_open,
            price_current,
            price_sl,
            price_tp,
            commission,
            swap,
            profit,
            symbol,
            comment,
            external_id,
        });
    }

    Ok(positions)
}

fn parse_orders_response(data: &[u8]) -> Result<Vec<TradeOrder>> {
    if data.len() < 4 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);
    let count = reader.read_u32() as usize;

    let mut orders = Vec::with_capacity(count);

    for _ in 0..count {
        let ticket = reader.read_i64();
        let time_setup = reader.read_i64();
        let time_setup_msc = reader.read_i64();
        let time_done = reader.read_i64();
        let time_done_msc = reader.read_i64();
        let time_expiration = reader.read_i64();
        let r#type = reader.read_u32() as i32;
        let type_time = reader.read_u32() as i32;
        let type_filling = reader.read_u32() as i32;
        let state = reader.read_u32() as i32;
        let magic = reader.read_i64();
        let position_id = reader.read_i64();
        let position_by_id = reader.read_i64();
        let reason = reader.read_u32() as i32;
        let volume_initial = reader.read_f64();
        let volume_current = reader.read_f64();
        let price_open = reader.read_f64();
        let price_current = reader.read_f64();
        let price_sl = reader.read_f64();
        let price_tp = reader.read_f64();
        let price_stoplimit = reader.read_f64();
        let symbol = reader.read_fixed_string(64);
        let comment = reader.read_fixed_string(64);
        let external_id = reader.read_fixed_string(64);

        if reader.has_error() {
            break;
        }

        orders.push(TradeOrder {
            ticket,
            time_setup,
            time_setup_msc,
            time_done,
            time_done_msc,
            time_expiration,
            r#type,
            type_time,
            type_filling,
            state,
            magic,
            position_id,
            position_by_id,
            reason,
            volume_initial,
            volume_current,
            price_open,
            price_current,
            price_sl,
            price_tp,
            price_stoplimit,
            symbol,
            comment,
            external_id,
        });
    }

    Ok(orders)
}

fn parse_deals_response(data: &[u8]) -> Result<Vec<TradeDeal>> {
    if data.len() < 4 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);
    let count = reader.read_u32() as usize;

    let mut deals = Vec::with_capacity(count);

    for _ in 0..count {
        let ticket = reader.read_i64();
        let order = reader.read_i64();
        let time = reader.read_i64();
        let time_msc = reader.read_i64();
        let r#type = reader.read_u32() as i32;
        let entry = reader.read_u32() as i32;
        let magic = reader.read_i64();
        let position_id = reader.read_i64();
        let reason = reader.read_u32() as i32;
        let volume = reader.read_f64();
        let price = reader.read_f64();
        let commission = reader.read_f64();
        let swap = reader.read_f64();
        let profit = reader.read_f64();
        let fee = reader.read_f64();
        let symbol = reader.read_fixed_string(64);
        let comment = reader.read_fixed_string(64);
        let external_id = reader.read_fixed_string(64);

        if reader.has_error() {
            break;
        }

        deals.push(TradeDeal {
            ticket,
            order,
            time,
            time_msc,
            r#type,
            entry,
            magic,
            position_id,
            reason,
            volume,
            price,
            commission,
            swap,
            profit,
            fee,
            symbol,
            comment,
            external_id,
        });
    }

    Ok(deals)
}

fn parse_rates_response(data: &[u8]) -> Result<Vec<Rate>> {
    if data.len() < 4 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);
    let count = reader.read_u32() as usize;

    let mut rates = Vec::with_capacity(count);

    for _ in 0..count {
        let time = reader.read_i64();
        let open = reader.read_f64();
        let high = reader.read_f64();
        let low = reader.read_f64();
        let close = reader.read_f64();
        let tick_volume = reader.read_u64();
        let spread = reader.read_i32();
        let real_volume = reader.read_u64();

        if reader.has_error() {
            break;
        }

        rates.push(Rate {
            time,
            open,
            high,
            low,
            close,
            tick_volume,
            spread,
            real_volume,
        });
    }

    Ok(rates)
}

/// Decode a single tick record (cmd 172 response payload, 60 bytes).
fn decode_tick(data: &[u8]) -> Result<Tick> {
    let mut reader = Reader::new(data);

    // Field order mirrors go-mt5 decodeTick (verified against testdata/tick_eurusd.bin).
    let time = reader.read_i64();
    let bid = reader.read_f64();
    let ask = reader.read_f64();
    let last = reader.read_f64();
    let volume = reader.read_u64();
    let time_msc = reader.read_i64();
    let flags = reader.read_u32();
    let volume_real = reader.read_f64();

    if reader.has_error() {
        return Err(Mt5Error::InvalidResponse("Failed to read tick".into()));
    }

    Ok(Tick {
        time,
        bid,
        ask,
        last,
        volume,
        time_msc,
        flags,
        volume_real,
    })
}

fn parse_ticks_response(data: &[u8]) -> Result<Vec<Tick>> {
    if data.len() < 4 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);
    let count = reader.read_u32() as usize;

    let mut ticks = Vec::with_capacity(count);

    for _ in 0..count {
        let time = reader.read_i64();
        let bid = reader.read_f64();
        let ask = reader.read_f64();
        let last = reader.read_f64();
        let volume = reader.read_u64();
        let time_msc = reader.read_i64();
        let flags = reader.read_u32();
        let volume_real = reader.read_f64();

        if reader.has_error() {
            break;
        }

        ticks.push(Tick {
            time,
            bid,
            ask,
            last,
            volume,
            time_msc,
            flags,
            volume_real,
        });
    }

    Ok(ticks)
}

fn parse_book_response(data: &[u8]) -> Result<Vec<BookInfo>> {
    if data.len() < 4 {
        return Err(Mt5Error::InvalidResponse("Response too short".into()));
    }

    let mut reader = Reader::new(data);
    let count = reader.read_u32() as usize;

    let mut books = Vec::with_capacity(count);

    for _ in 0..count {
        let r#type = reader.read_i64();
        let price = reader.read_f64();
        let volume = reader.read_i64();
        let volume_real = reader.read_f64();

        if reader.has_error() {
            break;
        }

        books.push(BookInfo {
            r#type,
            price,
            volume,
            volume_real,
        });
    }

    Ok(books)
}

fn encode_string(s: &str) -> Vec<u8> {
    let chars: Vec<u16> = s.encode_utf16().collect();
    let mut data = Vec::with_capacity(4 + chars.len() * 2);
    data.extend_from_slice(&(chars.len() as u32).to_le_bytes());
    for c in chars {
        data.extend_from_slice(&c.to_le_bytes());
    }
    data
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    error: bool,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            error: false,
        }
    }

    fn has_error(&self) -> bool {
        self.error
    }

    fn read_i64(&mut self) -> i64 {
        if self.error || self.pos + 8 > self.data.len() {
            self.error = true;
            return 0;
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ];
        self.pos += 8;
        i64::from_le_bytes(bytes)
    }

    fn read_u64(&mut self) -> u64 {
        if self.error || self.pos + 8 > self.data.len() {
            self.error = true;
            return 0;
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ];
        self.pos += 8;
        u64::from_le_bytes(bytes)
    }

    fn read_i32(&mut self) -> i32 {
        if self.error || self.pos + 4 > self.data.len() {
            self.error = true;
            return 0;
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        i32::from_le_bytes(bytes)
    }

    fn read_u32(&mut self) -> u32 {
        if self.error || self.pos + 4 > self.data.len() {
            self.error = true;
            return 0;
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        u32::from_le_bytes(bytes)
    }

    fn read_f64(&mut self) -> f64 {
        if self.error || self.pos + 8 > self.data.len() {
            self.error = true;
            return 0.0;
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ];
        self.pos += 8;
        f64::from_le_bytes(bytes)
    }

    fn read_bool1(&mut self) -> bool {
        if self.error || self.pos + 1 > self.data.len() {
            self.error = true;
            return false;
        }
        let b = self.data[self.pos];
        self.pos += 1;
        b != 0
    }

    fn read_fixed_string(&mut self, slot_bytes: usize) -> String {
        if self.error || self.pos + slot_bytes > self.data.len() {
            self.error = true;
            return String::new();
        }
        let end = self.pos + slot_bytes;
        let buf = &self.data[self.pos..end];

        let mut chars = Vec::with_capacity(slot_bytes / 2);
        let mut i = 0;
        while i + 1 < buf.len() {
            let c = u16::from_le_bytes([buf[i], buf[i + 1]]);
            if c == 0 {
                break;
            }
            chars.push(c);
            i += 2;
        }
        self.pos = end;
        String::from_utf16_lossy(&chars)
    }
}

fn read_string_at_offset(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }

    let mut chars = Vec::new();
    let mut pos = offset;
    while pos + 1 < data.len() {
        let c = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        if c == 0 {
            break;
        }
        chars.push(c);
    }
    String::from_utf16_lossy(&chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_slot(slot: &[u8]) -> String {
        let mut chars = Vec::new();
        let mut i = 0;
        while i + 1 < slot.len() {
            let c = u16::from_le_bytes([slot[i], slot[i + 1]]);
            if c == 0 {
                break;
            }
            chars.push(c);
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    }

    #[test]
    fn encode_trade_request_byte_positions() {
        // Mirror of go-mt5 TestOrderSendEncodesPackedRequest offsets.
        let req = TradeRequest {
            action: TradeAction::Deal,
            magic: 0,
            order: 0,
            symbol: "EURUSD".into(),
            volume: 0.01,
            price: 0.0,
            stoplimit: 0.0,
            sl: 0.0,
            tp: 0.0,
            deviation: 20,
            r#type: OrderType::Buy,
            type_filling: OrderFilling::IOC,
            type_time: OrderTime::GTC,
            expiration: 0,
            comment: "t".into(),
            position: 0,
            position_by: 0,
        };
        let data = encode_trade_request(&req);
        assert_eq!(data.len(), 232, "request must be 232 bytes");
        assert_eq!(
            u32::from_le_bytes(data[0..4].try_into().unwrap()),
            1,
            "action @0"
        );
        assert_eq!(utf16_slot(&data[20..84]), "EURUSD", "symbol slot @20");
        assert_eq!(
            f64::from_le_bytes(data[84..92].try_into().unwrap()),
            0.01,
            "volume @84"
        );
        assert_eq!(
            u64::from_le_bytes(data[124..132].try_into().unwrap()),
            20,
            "deviation @124"
        );
        assert_eq!(
            u32::from_le_bytes(data[132..136].try_into().unwrap()),
            0,
            "type @132"
        );
        assert_eq!(
            u32::from_le_bytes(data[136..140].try_into().unwrap()),
            1,
            "filling @136"
        );
        assert_eq!(
            u32::from_le_bytes(data[140..144].try_into().unwrap()),
            0,
            "time @140"
        );
        assert_eq!(utf16_slot(&data[152..216]), "t", "comment slot @152");
        assert_eq!(
            i64::from_le_bytes(data[216..224].try_into().unwrap()),
            0,
            "position @216"
        );
        assert_eq!(
            i64::from_le_bytes(data[224..232].try_into().unwrap()),
            0,
            "position_by @224"
        );
    }

    #[test]
    fn decode_check_result_fixture() {
        // Captured from a real terminal; expected values from go-mt5 TestOrderCheckDecodesPackedResponse.
        let fixture = include_bytes!("../testdata/order_check_done.bin");
        assert_eq!(fixture.len(), 252);
        let res = decode_check_result(fixture).unwrap();
        assert_eq!(res.retcode, 0);
        assert_eq!(res.balance, 103000.0);
        assert_eq!(res.equity, 103000.0);
        assert_eq!(res.profit, 0.0);
        assert_eq!(res.margin, 2.0);
        assert_eq!(res.margin_free, 102998.0);
        assert_eq!(res.margin_level, 5150000.0);
        assert_eq!(res.comment, "Done");
        assert!(res.is_ok());
    }

    #[test]
    fn decode_trade_result_fixture() {
        // Expected values from go-mt5 TestOrderSendDecodesPackedResponse.
        let fixture = include_bytes!("../testdata/order_send_done.bin");
        assert_eq!(fixture.len(), 260);
        let res = decode_trade_result(fixture).unwrap();
        assert_eq!(res.retcode, 10009);
        assert_eq!(res.deal, 18785220);
        assert_eq!(res.order, 27822128);
        assert_eq!(res.volume, 0.01);
        assert!((res.price - 1.16218).abs() < 1e-9);
        assert!((res.bid - 1.16213).abs() < 1e-9);
        assert!((res.ask - 1.16218).abs() < 1e-9);
        assert_eq!(res.comment, "Request executed");
        assert_eq!(res.request_id, 2316072679);
        assert_eq!(res.retcode_ext, 0);
        assert!(res.is_ok());
    }

    #[test]
    fn decode_trade_result_too_short() {
        assert!(decode_trade_result(&[0u8; 100]).is_err());
        assert!(decode_check_result(&[0u8; 100]).is_err());
    }

    #[test]
    fn trade_result_is_ok_matches_retcodes() {
        let ok = TradeResult {
            retcode: retcodes::DONE,
            ..Default::default()
        };
        assert!(ok.is_ok());
        let placed = TradeResult {
            retcode: retcodes::PLACED,
            ..Default::default()
        };
        assert!(placed.is_ok());
        let reject = TradeResult {
            retcode: retcodes::REJECT,
            ..Default::default()
        };
        assert!(!reject.is_ok());
    }

    // --- Fixture tests: every read parser validated against real terminal captures ---

    #[test]
    fn decode_account_info_fixture() {
        // Golden values from go-mt5 TestAccountInfoDecodeRealCapture.
        let fixture = include_bytes!("../testdata/account_info.bin");
        assert_eq!(fixture.len(), 851);
        let info = decode_account_info(fixture).unwrap();
        assert_eq!(info.login, 7395945);
        assert_eq!(info.leverage, 500);
        assert_eq!(info.currency, "EUR");
        assert_eq!(info.server, "FPTradingLLC-Demo");
        assert_eq!(info.company, "FP Trading LLC");
        assert_eq!(info.balance, 103000.0);
        assert_eq!(info.equity, 103000.0);
        assert_eq!(info.credit, 0.0);
        assert_eq!(info.profit, 0.0);
        assert!(!info.name.is_empty());
        // No open positions in this capture, so margin fields are zero.
        assert!(info.margin >= 0.0 && info.margin_level >= 0.0);
    }

    #[test]
    fn decode_symbol_info_fixture() {
        // Fixture carries an 8-byte cmd echo + success header; the record starts at [8..].
        let fixture = include_bytes!("../testdata/symbol_info_eurusd.bin");
        assert_eq!(fixture.len(), 3001);
        let mut reader = Reader::new(&fixture[8..]);
        let sym = decode_symbol_info(&mut reader).unwrap();
        assert_eq!(sym.name, "EURUSD");
        assert_eq!(sym.path, "Forex\\EURUSD");
        assert_eq!(sym.description, "Euro vs US Dollar");
        assert_eq!(sym.currency_base, "EUR");
        assert_eq!(sym.currency_profit, "USD");
        assert_eq!(sym.currency_margin, "EUR");
        assert_eq!(sym.digits, 5);
        assert_eq!(sym.spread, 32);
        assert_eq!(sym.trade_mode, 4);
        assert_eq!(sym.filling_mode, 3);
        assert_eq!(sym.point, 1e-05);
        assert_eq!(sym.trade_contract_size, 100000.0);
        assert_eq!(sym.volume_min, 0.01);
        assert_eq!(sym.volume_max, 100.0);
        assert_eq!(sym.volume_step, 0.01);
        assert!((sym.bid - 1.14152).abs() < 1e-9);
        assert!((sym.ask - 1.14184).abs() < 1e-9);
        assert!(sym.select);
        assert!(sym.visible);
        assert!(sym.spread_float);
        // The record must be consumed exactly: no bytes left over, none missing.
        assert!(!reader.has_error());
    }

    #[test]
    fn decode_tick_fixture() {
        // Golden values from go-mt5 TestSymbolInfoTickDecodesVolumeReal.
        let fixture = include_bytes!("../testdata/tick_eurusd.bin");
        assert_eq!(fixture.len(), 60);
        let tick = decode_tick(fixture).unwrap();
        assert_eq!(tick.time_msc, 1779194693301);
        assert_eq!(tick.flags, 0x406);
        assert_eq!(tick.volume_real, 0.0);
        assert!((tick.bid - 1.16177).abs() < 1e-4);
        assert!((tick.ask - 1.16189).abs() < 1e-4);
        assert_eq!(tick.time_msc / 1000, tick.time);
    }

    #[test]
    fn parse_ticks_fixture() {
        let fixture = include_bytes!("../testdata/ticks_100_eurusd.bin");
        assert_eq!(fixture.len(), 6004);
        let ticks = parse_ticks_response(fixture).unwrap();
        assert_eq!(ticks.len(), 100);
        for t in &ticks {
            assert!(t.time_msc > 0 && t.bid > 0.0 && t.ask > t.bid);
        }
        for w in ticks.windows(2) {
            assert!(w[0].time_msc <= w[1].time_msc, "ticks must be time-ordered");
        }
    }

    #[test]
    fn parse_rates_fixture() {
        let fixture = include_bytes!("../testdata/rates_h1_50_eurusd.bin");
        assert_eq!(fixture.len(), 3004);
        let rates = parse_rates_response(fixture).unwrap();
        assert_eq!(rates.len(), 50);
        for r in &rates {
            assert!(r.time > 0 && r.high >= r.low && r.high >= r.open && r.high >= r.close);
            assert!(r.low <= r.open && r.low <= r.close);
            assert!(r.tick_volume > 0);
        }
        for w in rates.windows(2) {
            assert!(w[0].time < w[1].time, "rates must be strictly time-ordered");
        }
    }

    #[test]
    fn parse_positions_fixture() {
        // Golden values from go-mt5 TestPositionsDecodeRealCapture (issue #17 regression).
        let fixture = include_bytes!("../testdata/positions_4buys_eurusd.bin");
        assert_eq!(fixture.len(), 1284);
        let positions = parse_positions_response(fixture).unwrap();
        assert_eq!(positions.len(), 4);

        let p0 = &positions[0];
        assert_eq!(p0.ticket, 27822128);
        assert_eq!(p0.r#type, 0, "PositionType::Buy");
        assert_eq!(p0.magic, 0);
        assert_eq!(p0.identifier, 27822128);
        assert_eq!(p0.reason, 3);
        assert_eq!(p0.volume, 0.01);
        assert_eq!(p0.price_open, 1.16218);
        assert_eq!(p0.price_sl, 0.0);
        assert_eq!(p0.price_tp, 0.0);
        assert_eq!(p0.commission, 0.0);
        assert_eq!(p0.swap, 0.0);
        assert_eq!(p0.symbol, "EURUSD");
        assert_eq!(p0.comment, "test-1");
        assert_eq!(p0.external_id, "");
        assert!((p0.price_current - 1.1597).abs() < 1e-6);
        assert!((p0.profit - (-2.14)).abs() < 1e-6);

        let expected_tail = [
            (27822129i64, "test-2", -2.11f64),
            (27822794, "test-3", -2.01),
            (27822815, "test-4", -2.08),
        ];
        for (i, (ticket, comment, profit)) in expected_tail.iter().enumerate() {
            let p = &positions[i + 1];
            assert_eq!(p.ticket, *ticket);
            assert_eq!(p.comment, *comment);
            assert!((p.profit - *profit).abs() < 1e-2);
        }
    }

    #[test]
    fn parse_deals_fixture() {
        let fixture = include_bytes!("../testdata/history_deals_30d.bin");
        assert_eq!(fixture.len(), 904);
        let deals = parse_deals_response(fixture).unwrap();
        assert_eq!(deals.len(), 3);
        for d in &deals {
            assert!(d.ticket > 0 && d.time > 0);
            assert!(d.time_msc / 1000 == d.time);
            // Balance/credit operations legitimately carry zero volume and price.
            for v in [d.volume, d.price, d.commission, d.swap, d.profit, d.fee] {
                assert!(v.is_finite());
            }
        }
    }

    // --- Round-trip tests: full request/response path through a mock transport ---

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// In-memory `Transport` that replays canned terminal payloads and records
    /// every request so tests can verify both encoding and decoding end-to-end.
    /// Mirrors go-mt5's newMockPipe (Layer 3 of the CI hardening).
    struct MockPipe {
        responses: Mutex<HashMap<u32, Vec<u8>>>,
        requests: Mutex<Vec<(u32, Vec<u8>)>>,
    }

    impl MockPipe {
        fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn respond(&self, cmd: u32, payload: &[u8]) -> &Self {
            self.responses.lock().unwrap().insert(cmd, payload.to_vec());
            self
        }

        fn requests_for(&self, cmd: u32) -> Vec<Vec<u8>> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|(c, _)| *c == cmd)
                .map(|(_, d)| d.clone())
                .collect()
        }
    }

    impl Transport for MockPipe {
        fn send(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>> {
            self.requests.lock().unwrap().push((cmd, data.to_vec()));
            self.responses
                .lock()
                .unwrap()
                .get(&cmd)
                .cloned()
                .ok_or_else(|| {
                    Mt5Error::InvalidResponse(format!("no canned response for cmd {}", cmd))
                })
        }
    }

    impl Transport for Arc<MockPipe> {
        fn send(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>> {
            MockPipe::send(self, cmd, data)
        }
    }

    fn mock_client(pipe: Arc<MockPipe>) -> Mt5Client {
        Mt5Client::from_transport(Box::new(pipe))
    }

    #[test]
    fn roundtrip_account_info() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/account_info.bin");
        pipe.respond(190, fixture);
        let client = mock_client(pipe.clone());
        let info = client.account_info().unwrap();
        assert_eq!(info.login, 7395945);
        assert_eq!(info.balance, 103000.0);
        // Request payload for account_info must be empty.
        assert!(pipe.requests_for(190)[0].is_empty());
    }

    #[test]
    fn roundtrip_symbol_info_encodes_symbol() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/symbol_info_eurusd.bin");
        pipe.respond(170, &fixture[8..]);
        let client = mock_client(pipe.clone());
        let sym = client.symbol_info("EURUSD").unwrap().unwrap();
        assert_eq!(sym.name, "EURUSD");
        assert_eq!(sym.digits, 5);
        // The request must carry the symbol as a length-prefixed UTF-16 string.
        let req = &pipe.requests_for(170)[0];
        assert_eq!(encode_string("EURUSD"), req.clone());
    }

    #[test]
    fn roundtrip_symbol_info_tick() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/tick_eurusd.bin");
        pipe.respond(172, fixture);
        let client = mock_client(pipe.clone());
        let tick = client.symbol_info_tick("EURUSD").unwrap().unwrap();
        assert_eq!(tick.time_msc, 1779194693301);
        assert_eq!(pipe.requests_for(172)[0], encode_string("EURUSD"));
    }

    #[test]
    fn roundtrip_positions_get() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/positions_4buys_eurusd.bin");
        pipe.respond(121, fixture);
        let client = mock_client(pipe.clone());
        let positions = client.positions_get(None).unwrap();
        assert_eq!(positions.len(), 4);
        assert_eq!(positions[0].ticket, 27822128);
    }

    #[test]
    fn roundtrip_copy_rates_from_pos() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/rates_h1_50_eurusd.bin");
        pipe.respond(108, fixture);
        let client = mock_client(pipe.clone());
        let rates = client.copy_rates_from_pos("EURUSD", 1, 0, 50).unwrap();
        assert_eq!(rates.len(), 50);
        assert_eq!(rates[0].time, rates[0].time);
    }

    #[test]
    fn roundtrip_copy_ticks_from() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/ticks_100_eurusd.bin");
        pipe.respond(104, fixture);
        let client = mock_client(pipe.clone());
        let ticks = client.copy_ticks_from("EURUSD", 0, 100, 0).unwrap();
        assert_eq!(ticks.len(), 100);
    }

    #[test]
    fn roundtrip_history_deals_get() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/history_deals_30d.bin");
        pipe.respond(151, fixture);
        let client = mock_client(pipe.clone());
        let deals = client.history_deals_get(0, i64::MAX).unwrap();
        assert_eq!(deals.len(), 3);
    }

    #[test]
    fn roundtrip_order_check() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/order_check_done.bin");
        pipe.respond(160, fixture);
        let client = mock_client(pipe.clone());
        let req = TradeRequest {
            action: TradeAction::Deal,
            symbol: "EURUSD".into(),
            volume: 0.01,
            deviation: 20,
            r#type: OrderType::Buy,
            type_filling: OrderFilling::IOC,
            ..Default::default()
        };
        let res = client.order_check(&req).unwrap();
        assert_eq!(res.retcode, 0);
        assert_eq!(res.margin, 2.0);
        assert_eq!(res.comment, "Done");
        assert_eq!(
            pipe.requests_for(160)[0].len(),
            232,
            "request must be 232 bytes"
        );
    }

    #[test]
    fn roundtrip_order_send() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/order_send_done.bin");
        pipe.respond(161, fixture);
        let client = mock_client(pipe.clone());
        let req = TradeRequest {
            action: TradeAction::Deal,
            symbol: "EURUSD".into(),
            volume: 0.01,
            deviation: 20,
            r#type: OrderType::Buy,
            type_filling: OrderFilling::IOC,
            ..Default::default()
        };
        let res = client.order_send(&req).unwrap();
        assert_eq!(res.retcode, 10009);
        assert_eq!(res.deal, 18785220);
        assert_eq!(res.order, 27822128);
        assert_eq!(pipe.requests_for(161)[0].len(), 232);
    }

    #[test]
    fn last_error_is_local_state_no_cmd3() {
        // Regression: last_error() used to send cmd 3, which kills the pipe on
        // modern builds. It must only read local state and never touch the wire.
        let pipe = Arc::new(MockPipe::new());
        let client = mock_client(pipe.clone());
        // Trigger a failure: no canned response for cmd 190.
        assert!(client.account_info().is_err());
        let (code, msg) = client.last_error().unwrap();
        assert_eq!(code, -1);
        assert!(!msg.is_empty());
        // The failure itself is the only wire traffic; cmd 3 must never be sent.
        assert!(pipe.requests_for(3).is_empty());
    }

    // --- Round-trip coverage for the remaining commands ---

    fn utf16_fixed(s: &str, slot_bytes: usize) -> Vec<u8> {
        let mut out = vec![0u8; slot_bytes];
        let enc: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        out[..enc.len().min(slot_bytes)].copy_from_slice(&enc[..enc.len().min(slot_bytes)]);
        out
    }

    fn put_str16(buf: &mut [u8], offset: usize, s: &str) {
        let enc: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        buf[offset..offset + enc.len()].copy_from_slice(&enc);
    }

    fn build_order_record(ticket: i64, symbol: &str) -> Vec<u8> {
        // Field order mirrors parse_orders_response (and go-mt5 decodeOrder).
        let mut b = Vec::new();
        b.extend_from_slice(&ticket.to_le_bytes());
        b.extend_from_slice(&1_700_000_000i64.to_le_bytes()); // time_setup
        b.extend_from_slice(&0i64.to_le_bytes()); // time_setup_msc
        b.extend_from_slice(&0i64.to_le_bytes()); // time_done
        b.extend_from_slice(&0i64.to_le_bytes()); // time_done_msc
        b.extend_from_slice(&0i64.to_le_bytes()); // time_expiration
        b.extend_from_slice(&0u32.to_le_bytes()); // type
        b.extend_from_slice(&0u32.to_le_bytes()); // type_time
        b.extend_from_slice(&0u32.to_le_bytes()); // type_filling
        b.extend_from_slice(&0u32.to_le_bytes()); // state
        b.extend_from_slice(&0i64.to_le_bytes()); // magic
        b.extend_from_slice(&0i64.to_le_bytes()); // position_id
        b.extend_from_slice(&0i64.to_le_bytes()); // position_by_id
        b.extend_from_slice(&0u32.to_le_bytes()); // reason
        b.extend_from_slice(&0.01f64.to_le_bytes()); // volume_initial
        b.extend_from_slice(&0.01f64.to_le_bytes()); // volume_current
        b.extend_from_slice(&1.15f64.to_le_bytes()); // price_open
        b.extend_from_slice(&1.16f64.to_le_bytes()); // price_current
        b.extend_from_slice(&0f64.to_le_bytes()); // price_sl
        b.extend_from_slice(&0f64.to_le_bytes()); // price_tp
        b.extend_from_slice(&0f64.to_le_bytes()); // price_stoplimit
        b.extend_from_slice(&utf16_fixed(symbol, 64)); // symbol
        b.extend_from_slice(&utf16_fixed("", 64)); // comment
        b.extend_from_slice(&utf16_fixed("", 64)); // external_id
        b
    }

    #[test]
    fn roundtrip_login() {
        let pipe = Arc::new(MockPipe::new());
        pipe.respond(100, &0u32.to_le_bytes());
        let client = mock_client(pipe.clone());
        client.login(12345, "secret", "Server-Demo").unwrap();
        let req = &pipe.requests_for(100)[0];
        assert_eq!(req[0..8], 12345i64.to_le_bytes());
        // password and server travel as length-prefixed UTF-16 strings.
        assert_eq!(
            encode_string("secret"),
            req[8..8 + encode_string("secret").len()]
        );
    }

    #[test]
    fn roundtrip_positions_and_orders_total() {
        let pipe = Arc::new(MockPipe::new());
        pipe.respond(120, &2u32.to_le_bytes());
        pipe.respond(130, &1u32.to_le_bytes());
        let client = mock_client(pipe.clone());
        assert_eq!(client.positions_total().unwrap(), 2);
        assert_eq!(client.orders_total().unwrap(), 1);
        assert!(pipe.requests_for(120)[0].is_empty());
        assert!(pipe.requests_for(130)[0].is_empty());
    }

    #[test]
    fn roundtrip_symbols_total_and_get() {
        let pipe = Arc::new(MockPipe::new());
        pipe.respond(173, &2u32.to_le_bytes());
        let fixture = include_bytes!("../testdata/symbol_info_eurusd.bin");
        let record = &fixture[8..];
        let mut payload = Vec::new();
        payload.extend_from_slice(&2u32.to_le_bytes());
        payload.extend_from_slice(record);
        payload.extend_from_slice(record);
        pipe.respond(174, &payload);
        let client = mock_client(pipe.clone());
        assert_eq!(client.symbols_total().unwrap(), 2);
        let syms = client.symbols_get().unwrap();
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].name, "EURUSD");
        assert_eq!(syms[1].bid, syms[0].bid);
    }

    #[test]
    fn roundtrip_orders_get() {
        let pipe = Arc::new(MockPipe::new());
        let rec = build_order_record(999001, "EURUSD");
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&rec);
        pipe.respond(131, &payload);
        let client = mock_client(pipe.clone());
        let orders = client.orders_get(None).unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].ticket, 999001);
        assert_eq!(orders[0].symbol, "EURUSD");
        assert_eq!(orders[0].volume_initial, 0.01);
    }

    #[test]
    fn roundtrip_history_orders_get() {
        let pipe = Arc::new(MockPipe::new());
        let rec = build_order_record(555, "GBPUSD");
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&rec);
        pipe.respond(141, &payload);
        let client = mock_client(pipe.clone());
        let orders = client.history_orders_get(0, i64::MAX).unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].ticket, 555);
        assert_eq!(orders[0].symbol, "GBPUSD");
    }

    #[test]
    fn roundtrip_copy_rates_range_and_ticks_range() {
        let pipe = Arc::new(MockPipe::new());
        let rates = include_bytes!("../testdata/rates_h1_50_eurusd.bin");
        let ticks = include_bytes!("../testdata/ticks_100_eurusd.bin");
        pipe.respond(107, rates);
        pipe.respond(105, ticks);
        let client = mock_client(pipe.clone());
        assert_eq!(
            client
                .copy_rates_range("EURUSD", 1, 0, i64::MAX)
                .unwrap()
                .len(),
            50
        );
        assert_eq!(
            client
                .copy_ticks_range("EURUSD", 0, i64::MAX, 0)
                .unwrap()
                .len(),
            100
        );
        // Verify the request carries the timeframe and date range.
        let req = &pipe.requests_for(107)[0];
        assert_eq!(
            req[..encode_string("EURUSD").len()],
            encode_string("EURUSD")
        );
    }

    #[test]
    fn roundtrip_symbol_select() {
        let pipe = Arc::new(MockPipe::new());
        pipe.respond(171, &[]); // empty payload == success
        let client = mock_client(pipe.clone());
        assert!(client.symbol_select("EURUSD", true).unwrap());
        let req = &pipe.requests_for(171)[0];
        assert_eq!(*req, [encode_string("EURUSD"), vec![1u8]].concat());
    }

    #[test]
    fn roundtrip_market_book() {
        let pipe = Arc::new(MockPipe::new());
        // market_book_add (191) and release (192) return empty payload on success.
        pipe.respond(191, &[]);
        pipe.respond(192, &[]);
        // market_book_get (193): count + 2 records of (type i64, price f64, volume i64, volume_real f64).
        let mut payload = Vec::new();
        payload.extend_from_slice(&2u32.to_le_bytes());
        for _ in 0..2 {
            payload.extend_from_slice(&0i64.to_le_bytes());
            payload.extend_from_slice(&1.23456f64.to_le_bytes());
            payload.extend_from_slice(&5i64.to_le_bytes());
            payload.extend_from_slice(&5.0f64.to_le_bytes());
        }
        pipe.respond(193, &payload);
        let client = mock_client(pipe.clone());
        assert!(client.market_book_add("EURUSD").unwrap());
        assert!(client.market_book_release("EURUSD").unwrap());
        let book = client.market_book_get("EURUSD").unwrap();
        assert_eq!(book.len(), 2);
        assert_eq!(book[0].price, 1.23456);
        assert_eq!(book[1].volume, 5);
    }

    #[test]
    fn roundtrip_terminal_info_and_version() {
        let pipe = Arc::new(MockPipe::new());
        // 40-byte numeric block + UTF-16 strings at the fixed offsets.
        let mut payload = vec![0u8; 2800];
        payload[0..2].copy_from_slice(&6090u16.to_le_bytes()); // build
        payload[6] = 1; // connected
        payload[12..16].copy_from_slice(&100_000u32.to_le_bytes()); // max_bars
        payload[21..23].copy_from_slice(&10u16.to_le_bytes()); // ping_last
        payload[24..32].copy_from_slice(&1.5f64.to_le_bytes()); // community_balance
        put_str16(&mut payload, 41, "Company Inc.");
        put_str16(&mut payload, 561, "MetaTrader 5");
        put_str16(&mut payload, 1081, "English");
        put_str16(&mut payload, 1601, "C:\\MT5");
        put_str16(&mut payload, 2121, "C:\\MT5\\Data");
        put_str16(&mut payload, 2641, "C:\\MT5\\Common");
        pipe.respond(180, &payload);
        let client = mock_client(pipe.clone());
        let info = client.terminal_info().unwrap();
        assert!(info.connected);
        assert_eq!(info.build, 6090);
        assert_eq!(info.max_bars, 100_000);
        assert_eq!(info.company, "Company Inc.");
        assert_eq!(info.name, "MetaTrader 5");
        assert_eq!(info.path, "C:\\MT5");
        let ver = client.version().unwrap();
        assert_eq!(ver.version, 500);
        assert_eq!(ver.build, 6090);
    }

    #[test]
    fn order_calc_margin_and_profit() {
        let pipe = Arc::new(MockPipe::new());
        let fixture = include_bytes!("../testdata/symbol_info_eurusd.bin");
        pipe.respond(170, &fixture[8..]); // margin_initial in fixture is > 0? (see below)
        let acct = include_bytes!("../testdata/account_info.bin");
        pipe.respond(190, acct);
        let client = mock_client(pipe.clone());

        // Path A: margin_initial > 0 from symbol info.
        let info = client.symbol_info("EURUSD").unwrap().unwrap();
        if info.margin_initial > 0.0 {
            let m = client.order_calc_margin(0, "EURUSD", 0.01, 1.15).unwrap();
            assert!((m - 0.01 * 1.15 * info.margin_initial / 4.0).abs() < 1e-9);
        } else {
            // Path B: fallback to leverage (volume x price x contract / leverage).
            let m = client.order_calc_margin(0, "EURUSD", 0.01, 1.15).unwrap();
            assert!((m - 0.01 * 1.15 * 100000.0 / 500.0).abs() < 1e-6);
        }

        // Profit: volume x (close - open) x contract.
        let p = client
            .order_calc_profit(0, "EURUSD", 0.01, 1.15, 1.20)
            .unwrap();
        assert!((p - 0.01 * 0.05 * 100000.0).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_remaining_commands() {
        let pipe = Arc::new(MockPipe::new());
        let rates = include_bytes!("../testdata/rates_h1_50_eurusd.bin");
        pipe.respond(106, rates); // copy_rates_from
        pipe.respond(150, &3u32.to_le_bytes()); // history_deals_total
        pipe.respond(140, &2u32.to_le_bytes()); // history_orders_total
        pipe.respond(999, &[0xAA, 0xBB, 0xCC, 0xDD]); // send_raw_command
        let client = mock_client(pipe.clone());

        assert_eq!(
            client.copy_rates_from("EURUSD", 1, 0, 50).unwrap().len(),
            50
        );
        assert_eq!(client.history_deals_total(0, i64::MAX).unwrap(), 3);
        assert_eq!(client.history_orders_total(0, i64::MAX).unwrap(), 2);
        let raw = client.send_raw_command(999, &[1, 2, 3]).unwrap();
        assert_eq!(raw, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        // Request payloads must be forwarded untouched.
        assert_eq!(pipe.requests_for(999)[0], vec![1, 2, 3]);
        // history_deals_total carries the from/to range as two i64s.
        assert_eq!(pipe.requests_for(150)[0].len(), 16);
    }

    #[test]
    fn parse_deals_fixture_242() {
        // Older capture with a 12-byte header (from go-mt5 TestHistoryDealsDecodeRealCapture_242Deals).
        let fixture = include_bytes!("../testdata/history_deals_5833_242deals.bin");
        let deals = parse_deals_response(&fixture[12..]).unwrap();
        assert_eq!(deals.len(), 242);
        for d in &deals {
            assert!(d.ticket > 0 && d.time > 0);
            for v in [d.volume, d.price, d.commission, d.swap, d.profit, d.fee] {
                assert!(v.is_finite());
            }
        }
    }
}
