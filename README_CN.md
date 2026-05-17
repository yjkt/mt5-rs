# mt5-rs

纯 Rust 实现的 MetaTrader 5 IPC 通信库，无需 Python 依赖。

与 Python `MetaTrader5` 库 API 完全兼容。

## 特性

- 纯 Rust 实现，无 Python 或 C++ 依赖
- 通过 Windows 命名管道与 MT5 终端进行 IPC 通信
- 与 Python `MetaTrader5` 库完全兼容（32个函数中的30个）
- 支持 MT5 Build 5836+

## 快速开始

```rust
use mt5_rs::{Mt5Client, discover_mt5_pipe};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 自动发现 MT5 管道
    let pipe_name = discover_mt5_pipe();
    
    // 初始化客户端
    let mut client = Mt5Client::new();
    client.initialize(Some(&pipe_name))?;
    
    // 获取账户信息
    let account = client.account_info()?;
    println!("Balance: {}", account.balance);
    println!("Equity: {}", account.equity);
    println!("Margin Free: {}", account.margin_free);
    
    Ok(())
}
```

## API 参考

### 初始化

| 函数 | 描述 |
|------|------|
| `Mt5Client::new()` | 创建新客户端 |
| `initialize(pipe_name)` | 初始化与 MT5 的连接 |
| `shutdown()` | 关闭连接 |
| `login(login, password, server)` | 登录 MT5 账户 |

### 账户与终端

| 函数 | 描述 |
|------|------|
| `account_info()` | 获取账户信息 |
| `terminal_info()` | 获取终端信息 |
| `version()` | 获取 MT5 版本 |
| `last_error()` | 获取最后错误代码和消息 |

### 交易品种

| 函数 | 描述 |
|------|------|
| `symbol_info(symbol)` | 获取品种信息 |
| `symbol_info_tick(symbol)` | 获取品种当前报价 |
| `symbols_get()` | 获取所有可用品种 |
| `symbol_select(symbol, enable)` | 在市场报价中选择/取消选择品种 |

### 行情数据

| 函数 | 描述 |
|------|------|
| `copy_rates_from_pos(symbol, timeframe, start_pos, count)` | 从指定位置复制K线 |
| `copy_rates_from(symbol, timeframe, date_from, count)` | 从指定日期复制K线 |
| `copy_rates_range(symbol, timeframe, date_from, date_to)` | 复制指定范围的K线 |
| `copy_ticks_from(symbol, from, count, flags)` | 从指定时间复制Tick |
| `copy_ticks_range(symbol, from, to, flags)` | 复制指定范围的Tick |

### 持仓与订单

| 函数 | 描述 |
|------|------|
| `positions_total()` | 获取未平仓持仓总数 |
| `positions_get(symbol)` | 获取未平仓持仓 |
| `orders_total()` | 获取挂单总数 |
| `orders_get(symbol)` | 获取挂单 |

### 历史数据

| 函数 | 描述 |
|------|------|
| `history_deals_total(from, to)` | 获取指定范围内成交总数 |
| `history_deals_get(from, to)` | 获取指定范围内成交记录 |
| `history_orders_total(from, to)` | 获取指定范围内订单总数 |
| `history_orders_get(from, to)` | 获取指定范围内订单记录 |

### 市场深度

| 函数 | 描述 |
|------|------|
| `market_book_add(symbol)` | 订阅市场深度 |
| `market_book_get(symbol)` | 获取市场深度数据 |
| `market_book_release(symbol)` | 取消订阅市场深度 |

### 交易计算

| 函数 | 描述 |
|------|------|
| `order_calc_margin(action, symbol, volume, price)` | 计算所需保证金（本地计算） |
| `order_calc_profit(action, symbol, volume, price_open, price_close)` | 计算预期利润（本地计算） |

### 未实现

| 函数 | 状态 |
|------|------|
| `order_check(request)` | TODO - 检查交易请求有效性 |
| `order_send(request)` | TODO - 发送交易请求到 MT5 |

## 实现说明

### `order_calc_margin` 和 `order_calc_profit` 的本地计算

与通过命名管道发送命令 202/203 的 `go-mt5` 不同，本库使用**本地计算**（与 Python `MetaTrader5` 库行为一致）：

- `order_calc_margin`: `margin = volume × price × margin_initial / 4`
- `order_calc_profit`: `profit = volume × (price_close - price_open) × trade_contract_size`

此方法避免了 MT5 Build 5836+ 使用 IPC 执行这些命令时的"管道已关闭"错误。

## 系统要求

- Rust 2021 edition
- Windows 操作系统（命名管道 IPC 仅限 Windows）
- MT5 终端必须正在运行

## 许可证

MIT
