# mt5-rs

[![CI](https://github.com/nirvagold/mt5-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/nirvagold/mt5-rs/actions/workflows/ci.yml)

A pure Rust library for MetaTrader 5 IPC communication. No Python dependency.

Compatible with Python `MetaTrader5` library API.

## Features

- Pure Rust implementation, no Python or C++ dependency
- Windows named pipe IPC communication with MT5 terminal
- Full compatibility with Python `MetaTrader5` library (32/32 functions)
- Support MT5 Build 5836+

## Quick Start

```rust
use mt5_rs::{Mt5Client, discover_mt5_pipe};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-discover MT5 pipe
    let pipe_name = discover_mt5_pipe();
    
    // Initialize client
    let mut client = Mt5Client::new();
    client.initialize(Some(&pipe_name))?;
    
    // Get account info
    let account = client.account_info()?;
    println!("Balance: {}", account.balance);
    println!("Equity: {}", account.equity);
    println!("Margin Free: {}", account.margin_free);
    
    Ok(())
}
```

## API Reference

### Initialization

| Function | Description |
|----------|-------------|
| `Mt5Client::new()` | Create a new client |
| `initialize(pipe_name)` | Initialize connection to MT5 |
| `shutdown()` | Close connection |
| `login(login, password, server)` | Login to MT5 account |

### Account & Terminal

| Function | Description |
|----------|-------------|
| `account_info()` | Get account information |
| `terminal_info()` | Get terminal information |
| `version()` | Get MT5 version |
| `last_error()` | Get last error code and message |

### Symbols

| Function | Description |
|----------|-------------|
| `symbol_info(symbol)` | Get symbol information |
| `symbol_info_tick(symbol)` | Get current tick for symbol |
| `symbols_get()` | Get all available symbols |
| `symbol_select(symbol, enable)` | Select/deselect symbol in Market Watch |

### Market Data

| Function | Description |
|----------|-------------|
| `copy_rates_from_pos(symbol, timeframe, start_pos, count)` | Copy bars from position |
| `copy_rates_from(symbol, timeframe, date_from, count)` | Copy bars from date |
| `copy_rates_range(symbol, timeframe, date_from, date_to)` | Copy bars in range |
| `copy_ticks_from(symbol, from, count, flags)` | Copy ticks from time |
| `copy_ticks_range(symbol, from, to, flags)` | Copy ticks in range |

### Positions & Orders

| Function | Description |
|----------|-------------|
| `positions_total()` | Get total number of open positions |
| `positions_get(symbol)` | Get open positions |
| `orders_total()` | Get total number of pending orders |
| `orders_get(symbol)` | Get pending orders |

### History

| Function | Description |
|----------|-------------|
| `history_deals_total(from, to)` | Get total number of deals in range |
| `history_deals_get(from, to)` | Get deals in range |
| `history_orders_total(from, to)` | Get total number of orders in range |
| `history_orders_get(from, to)` | Get orders in range |

### Market Depth

| Function | Description |
|----------|-------------|
| `market_book_add(symbol)` | Subscribe to market depth |
| `market_book_get(symbol)` | Get market depth data |
| `market_book_release(symbol)` | Unsubscribe from market depth |

### Trading

| Function | Description |
|----------|-------------|
| `order_check(&TradeRequest) -> TradeCheckResult` | Check whether a trade request is valid, without placing it |
| `order_send(&TradeRequest) -> TradeResult` | Send a trade request to the terminal for execution |
| `order_calc_margin(action, symbol, volume, price)` | Calculate required margin (local calculation) |
| `order_calc_profit(action, symbol, volume, price_open, price_close)` | Calculate expected profit (local calculation) |

Trade request fields mirror the Python `mt5.TradeRequest` structure (`action`, `magic`, `order`, `symbol`, `volume`, `price`, `stoplimit`, `sl`, `tp`, `deviation`, `type`, `type_filling`, `type_time`, `expiration`, `comment`, `position`, `position_by`). Use `TradeRequest::new(action, symbol, volume, type)` to build a market order, then set `sl`/`tp`/`price`/`deviation` as needed. `TradeResult::is_ok()` / `TradeCheckResult::is_ok()` report whether the terminal accepted the request.

```rust
use mt5_rs::{Mt5Client, discover_mt5_pipe, TradeRequest, TradeAction, OrderType, OrderFilling};

let mut client = Mt5Client::new();
client.initialize(Some(&discover_mt5_pipe()))?;

let tick = client.symbol_info_tick("EURUSD")?.unwrap();
let mut req = TradeRequest::new(TradeAction::Deal, "EURUSD", 0.01, OrderType::Buy);
req.price = tick.ask;
req.type_filling = OrderFilling::FOK;
req.comment = "my bot".into();

let result = client.order_send(&req)?;
println!("retcode={} comment={}", result.retcode, result.comment);
```

## Implementation Notes

### Local Calculation for `order_calc_margin` and `order_calc_profit`

Unlike `go-mt5` which sends commands 202/203 via named pipe, this library uses **local calculation** (same as Python `MetaTrader5` library):

- `order_calc_margin`: `margin = volume × price × margin_initial / 4`
- `order_calc_profit`: `profit = volume × (price_close - price_open) × trade_contract_size`

This approach avoids the "pipe closed" error that occurs with MT5 Build 5836+ when using IPC for these commands.

## Requirements

- Rust 2021 edition
- Windows OS (named pipe IPC is Windows-specific)
- MT5 terminal must be running

## License

MIT
