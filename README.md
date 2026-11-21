# imsg-mcp

A Rust-based Model Context Protocol (MCP) server for accessing iMessage data on macOS.

## Features

- **Inbox Messages**: Retrieve received iMessages from the local iMessage database
- **Sent Messages**: Retrieve sent iMessages from the local iMessage database
- **Timestamp Decoding**: Properly decodes Apple's Snowflake timestamp format used in iMessage databases
- **Contact Lookup**: (Planned) Integration with macOS Contacts framework for contact name resolution

## Prerequisites

- **macOS**: This server only works on macOS as it relies on the iMessage database located at `~/Library/Messages/chat.db`
- **Rust**: Rust 1.75+ or later (for building the project)
- **iMessage Database Access**: The user running the server must have access to `~/Library/Messages/chat.db`

## Installation

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/imsg-mcp.git
cd imsg-mcp

# Build the release binary
cargo build --release
```

The compiled binary will be available at `target/release/imsg-mcp`.

### Using with an MCP Client

To use `imsg-mcp` with an MCP-compatible client (like Claude Desktop), add the following configuration to your client's MCP settings:

```json
{
  "mcpServers": {
    "imsg-mcp": {
      "command": "/path/to/imsg-mcp/target/release/imsg-mcp"
    }
  }
}
```

## Available Tools

### `inbox`

Lists the messages in the inbox (received messages).

**Parameters:**
- `limit` (integer, optional): Maximum number of messages to return. Default: `100`

**Example:**
```json
{
  "name": "inbox",
  "arguments": {
    "limit": 50
  }
}
```

### `sent`

Lists the messages in the sent folder.

**Parameters:**
- `limit` (integer, optional): Maximum number of messages to return. Default: `100`

**Example:**
```json
{
  "name": "sent",
  "arguments": {
    "limit": 50
  }
}
```

## Database Schema

The server queries the macOS iMessage database (`chat.db`) using the following tables:
- `message`: Contains message content, timestamps, and routing information
- `handle`: Contains sender/receiver identifier information (phone numbers, emails)
- `chat`: Contains conversation metadata

## Timestamp Decoding

The iMessage database uses Apple's "Snowflake" ID format for timestamps, which encodes:
- A timestamp (epoch: January 1, 2001)
- A worker ID
- A process ID
- A sequence number

The server properly decodes these values into standard UTC datetime objects.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the Python `mcp-server-imessage` project
- MCP (Model Context Protocol) specification by Anthropic
