# Raderbot

## Table of Contents

- [Overview](#overview)
- [Account Features](#account-features)
- [Market Data Features](#market-data-features)
- [Strategy Management Features](#strategy-management-features)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## Overview

Raderbot is a Rust-based trading bot designed to interact with the BingX WebSocket API for market data analysis and trading operations. It utilizes the Actix Web framework for building a WebSocket server and handling incoming messages. The bot connects to the BingX WebSocket API to receive real-time market data updates, including ticker information. It performs various operations on the received data, such as calculating time differences, generating unique identifiers, parsing JSON responses, and handling errors using custom error types. The bot also includes functionalities for file operations, such as creating and appending data to files. It leverages external crates like reqwest for making HTTP requests and tungstenite for WebSocket communication.

## Account Features

### Position Management

- **Open Positions**: Allows opening positions with detailed parameters (symbol, margin, leverage, order side, and optional stop loss).
- **Close Position**: Enables closing an individual position using its ID, with automatic handling of price lookup and trade execution.
- **Close All Positions**: Offers the capability to close all open positions with a single request, facilitating quick portfolio adjustments or strategy changes.

### Account Information Retrieval

- **Account Snapshot**: Provides real-time account information, including current holdings, positions, and trading capabilities.
- **List Active Positions**: Lists all active positions to provide insights into market exposure and position specifics.
- **Recent Trades**: Retrieves a list of recent trades, aiding in the analysis of trading performance and strategy outcomes.

### Exchange API Flexibility

- **Dynamic API Switching**: Features an endpoint to dynamically set the exchange API, supporting transitions between live and mock environments or different exchanges without downtime.
- **Multiple Exchange Support**: Accommodates various exchange APIs, including a mock interface for risk-free testing and strategy development.

### Real-time Market Data Integration

- Leverages live market data for informed decision-making in trading operations, ensuring that actions reflect the latest market conditions.

### Safety Features

- **Stop Loss Functionality**: Incorporates stop-loss options in position opening, enhancing risk management through predefined loss limits.
- **Robust Error Handling**: Delivers comprehensive error handling and response messaging, clearly indicating the outcomes of API requests.

## Market Data Features

### Kline Data

- **Retrieve Kline Data**: Fetch historical k-line (candlestick) data for a given symbol and interval, allowing users to analyze past market movements.
- **Range-based Kline Data Retrieval**: Obtain k-line data within a specified date range, supporting in-depth analysis and backtesting strategies over specific periods.

### Ticker Data

- **Last Price Query**: Access the most recent trading price for a specified symbol, crucial for real-time decision making.
- **Ticker Data Access**: Get detailed ticker information, including price changes, high, low, and volume data, providing a comprehensive market overview.

### Stream Management

- **Open Data Stream**: Initiate a live data stream for k-lines or ticker updates, enabling real-time market monitoring and analysis.
- **Close Data Stream**: Terminate an active data stream, managing resource usage and focusing on relevant market data.
- **Active Streams Information**: List all active streams, offering insights into currently monitored symbols and intervals.

### Market Information

- **Market Summary**: Present a general market overview, including available symbols, trading pairs, and other relevant information, helping users stay informed about market conditions.

## Strategy Management Features

### Strategy Operations

- **Create New Strategies**: Initiate new trading strategies with customized settings including symbols, strategy names, algorithm parameters, intervals, margins, and leverage.
- **Stop Strategy**: Terminate an active strategy optionally closing all associated positions.
- **List Strategy Positions**: Retrieve all positions opened under a specific strategy, facilitating detailed performance analysis.
- **Active Strategy Summary**: Generate summaries for active strategies, providing insights into their current state and effectiveness.
- **Strategy Information**: Fetch detailed information about specific strategies, including configuration and performance metrics.
- **List Active and Historical Strategies**: View active strategies for ongoing monitoring and historical strategies for post-analysis.
- **Stop All Strategies**: Conveniently stop all active strategies with an option to close all positions, useful for rapid response to market changes or strategy realignment.

### Strategy Configuration

- **Set Strategy Parameters**: Dynamically adjust strategy parameters to adapt to changing market conditions or refine strategy logic.
- **Change Strategy Settings**: Modify strategy settings such as maximum open orders, margin, leverage, and stop-loss thresholds.

### Strategy Testing

- **Run Backtest**: Perform backtesting on strategies with specific parameters over designated time frames, aiding in strategy validation and optimization.

## Roadmap

The project is currently in the development phase.

## Development

### Prerequisites

Make sure you have the following tools installed on your system:

- [Rust](https://www.rust-lang.org/): The programming language used for the project.
- [cargo-watch](https://crates.io/crates/cargo-watch): Used for automatically restarting the server during development.
- [make](https://www.gnu.org/software/make/): Used for managing development tasks.

### Getting Started

1. Clone the repository:

   ```bash
   git clone https://github.com/subaquatic-pierre/raderbot.git

   ```

2. Navigate to the project directory:

```bash
cd raderbot
```

3. Build and run the project:

```bash
make watch
```

This command will start the Actix server and automatically restart it when code changes are detected.

### Cleaning Up

To clean up build artifacts, run:

```bash
make clean
```

## Contributing

We welcome contributions from the community! If you would like to contribute to the project, please follow these guidelines:

- Fork the repository.
- Create a new branch: git checkout -b my-branch.
- Make your changes and commit them: git commit -am 'Add new feature'.
- Push to the branch: git push origin my-branch.
- Submit a pull request.

## License

This project is licensed under the MIT License.

## Trading Algorithms

Given the data available in the Kline struct (which includes the symbol, interval, open, high, low, close prices, volume, open time, and close time), there are several types of trading algorithms you could implement. These algorithms can vary greatly in complexity and approach, depending on the trading strategy you wish to pursue. Here's a list of potential algorithm types based on the Kline data:

### Moving Average Crossover:

This strategy uses two moving averages (MA), a short-term MA and a long-term MA. A "buy" signal is generated when the short-term MA crosses above the long-term MA, and a "sell" signal is generated when the short-term MA crosses below the long-term MA.

### Relative Strength Index (RSI):

The RSI is a momentum oscillator that measures the speed and change of price movements. It oscillates between 0 and 100. Traditionally, and according to Wilder, RSI is considered overbought when above 70 and oversold when below 30.

### Bollinger Bands:

This method involves calculating a moving average of the security's price, and then creating two bands (the Bollinger Bands) above and below the moving average. The bands expand and contract based on the volatility of the prices.

### MACD (Moving Average Convergence Divergence):

The MACD is calculated by subtracting the long-term EMA (exponential moving average) from the short-term EMA. The MACD line is then plotted against a signal line (the EMA of the MACD) to identify potential buy or sell signals.

### Volume Weighted Average Price (VWAP):

VWAP is calculated by adding up the money traded for every transaction (price multiplied by the number of shares traded) and then dividing by the total shares traded. This gives you an average price a security has traded at throughout the day, based on both volume and price. It is important for assessing whether a security was bought or sold at a good price.

### Breakout Strategy:

This strategy involves identifying a range or a price level that a security has failed to exceed previously (resistance) or fall below (support), and then opening positions as the price breaks out from that range or level.

### Mean Reversion:

Based on the assumption that prices and returns eventually move back towards the mean or average. This strategy might involve buying securities that have performed poorly recently and selling those that have performed well.

### Momentum Trading:

Identifies securities moving in one direction with high volume and continues to buy/sell them until they start showing signs of reversal.

### Pattern Recognition:

This involves identifying patterns within the price charts (like head and shoulders, triangles, flags, etc.) that can indicate potential bullish or bearish movements.

use admin
db.createUser(
{
user: "rootuser",
pwd: "rootpass",
roles: [ { role: "userAdminAnyDatabase", db: "admin" } ]
}
)

150000000
103393842
