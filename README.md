# imsg-mcp

A Rust-based Model Context Protocol (MCP) server for accessing iMessage data on macOS.

## Features

- **Inbox Messages**: Retrieve received iMessages from the local iMessage database
- **Sent Messages**: Retrieve sent iMessages from the local iMessage database
- **Timestamp Decoding**: Properly decodes Apple's Snowflake timestamp format used in iMessage databases
- **Contact Lookup**: (Planned) Integration with macOS Contacts framework for contact name resolution
- **HTTP Streamable Transport**: MCP server supports HTTP streamable transport via `POST /mcp` and `GET /mcp` (SSE)

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

## MCP Server Configuration

### HTTP Streamable Transport (Recommended)

The MCP server supports the HTTP streamable transport protocol, which is the recommended approach for modern MCP clients.

#### Starting the Server

Run the server:
```bash
./target/release/imsg-mcp
```

The server will start listening on `http://127.0.0.1:3000/mcp`.

#### Server Configuration File

The server supports configuration via a JSON file for the `port` setting. It will look for configuration files in the following order:
1. `imsg-mcp.json` (current directory)
2. `.imsg-mcp.json` (current directory)
3. `~/.imsg-mcp.json` (home directory)
4. `~/.imsg-mcp/config.json` (home directory)

**Example configuration file (`imsg-mcp.json`):**
```json
{
  "port": 3000
}
```

**Configuration options:**
- `port` (integer, optional): The port the server listens on. Default: `3000`

**Authentication Token (Secure Method):**

For security, the authentication token should **not** be stored in the config file in clear text. Instead, use the `IMSG_MCP_AUTH_TOKEN` environment variable:

```bash
export IMSG_MCP_AUTH_TOKEN="your-secret-token-here"
./target/release/imsg-mcp
```

If an `auth_token` is found in the config file, the server will log a warning and ignore it, prioritizing the environment variable for security.

If no configuration file is found and no `IMSG_MCP_AUTH_TOKEN` environment variable is set, the server uses default values (port: 3000, no authentication).

#### Configuring for Claude Desktop

To use `imsg-mcp` with Claude Desktop using the HTTP transport:

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Start the MCP server:
   ```bash
   ./target/release/imsg-mcp
   ```

3. Locate or create the Claude Desktop MCP configuration file:
   - **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

4. Add the `imsg-mcp` server to the configuration using the HTTP transport:
   ```json
   {
     "mcpServers": {
       "imsg-mcp": {
         "url": "http://127.0.0.1:3000/mcp"
       }
     }
   }
   ```

5. Restart Claude Desktop to apply the changes.

#### Configuring for Other MCP Clients

**Cursor**

Cursor supports MCP servers via HTTP transport. Add the following to your Cursor MCP settings:

```json
{
  "mcpServers": {
    "imsg-mcp": {
      "url": "http://127.0.0.1:3000/mcp"
    }
  }
}
```

**Windsurf**

Add to your Windsurf MCP configuration file:

```json
{
  "mcpServers": {
    "imsg-mcp": {
      "url": "http://127.0.0.1:3000/mcp"
    }
  }
}
```

### macOS Permissions Required

For `imsg-mcp` to function properly on macOS, the following permissions may be required:

1. **iMessage Database Access**: The server reads from `~/Library/Messages/chat.db`. Ensure your user account has read access to this file.

2. **Contacts Framework Access** (Planned): Full contact name resolution requires the app to have Contacts framework entitlements and user permission. The current implementation includes a stub for contact lookup that returns `None` without proper entitlements.

### Troubleshooting

- **Server not starting**: Ensure port 3000 is not in use by another process. Check the server logs for any errors.
- **Database access denied**: Verify that the user running the MCP server has read permissions to `~/Library/Messages/chat.db`.
- **Client connection failed**: Ensure the server is running and accessible at `http://127.0.0.1:3000/mcp`. Check your MCP client's logs for connection errors.

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

## HTTP API

The MCP server exposes the following HTTP endpoints:

### `POST /mcp`

Handles JSON-RPC 2.0 requests. Accepts `application/json` content type and returns JSON responses by default, or `text/event-stream` (SSE) if the `Accept: text/event-stream` header is provided.

**Request headers:**
- `Content-Type: application/json`
- `Accept: application/json` (default) or `Accept: text/event-stream`

**Request body:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 1
}
```

### `GET /mcp`

Establishes an SSE (Server-Sent Events) connection for server-to-client messages. Returns `text/event-stream`.

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
