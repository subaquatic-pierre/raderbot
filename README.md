# Raderbot

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Contributing](#contributing)
- [License](#license)

## Overview

Raderbot is a Rust-based trading bot designed to interact with the BingX WebSocket API for market data analysis and trading operations. It utilizes the Actix Web framework for building a WebSocket server and handling incoming messages. The bot connects to the BingX WebSocket API to receive real-time market data updates, including ticker information. It performs various operations on the received data, such as calculating time differences, generating unique identifiers, parsing JSON responses, and handling errors using custom error types. The bot also includes functionalities for file operations, such as creating and appending data to files. It leverages external crates like reqwest for making HTTP requests and tungstenite for WebSocket communication.

## Features

- Connects to the BingX WebSocket API to receive real-time market data updates.
- Calculates time differences between timestamps.
- Generates unique identifiers in a specific format.
- Parses JSON responses received from API calls.
- Handles errors using custom error types.
- Performs file operations, including creating and appending data to files.
- Utilizes external crates like reqwest and tungstenite.

## Roadmap

The project is currently in the development phase.

## Contributing

We welcome contributions from the community! If you would like to contribute to the project, please follow these guidelines:

- Fork the repository.
- Create a new branch: git checkout -b my-branch.
- Make your changes and commit them: git commit -am 'Add new feature'.
- Push to the branch: git push origin my-branch.
- Submit a pull request.

## License

This project is licensed under the MIT License.
