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

## MCP Server Configuration

### Configuring for Claude Desktop

To use `imsg-mcp` with Claude Desktop, you need to add it to your MCP servers configuration file.

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Locate or create the Claude Desktop MCP configuration file:
   - **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

3. Add the `imsg-mcp` server to the configuration:
   ```json
   {
     "mcpServers": {
       "imsg-mcp": {
         "command": "/Users/yourusername/development/imsg-mcp/target/release/imsg-mcp",
         "env": {
           "TERM_PROGRAM": "Apple_Terminal"
         }
       }
     }
   }
   ```

   **Important**: Replace `/Users/yourusername/development/imsg-mcp/target/release/imsg-mcp` with the actual path to your compiled binary.

4. Restart Claude Desktop to apply the changes.

### Configuring for Other MCP Clients

#### Cursor

Cursor supports MCP servers through its settings. Add the following to your Cursor MCP settings:

```json
{
  "mcpServers": {
    "imsg-mcp": {
      "command": "/Users/yourusername/development/imsg-mcp/target/release/imsg-mcp"
    }
  }
}
```

#### Windsurf

Add to your Windsurf MCP configuration file:

```json
{
  "mcpServers": {
    "imsg-mcp": {
      "command": "/Users/yourusername/development/imsg-mcp/target/release/imsg-mcp"
    }
  }
}
```

### macOS Permissions Required

For `imsg-mcp` to function properly on macOS, the following permissions may be required:

1. **iMessage Database Access**: The server reads from `~/Library/Messages/chat.db`. Ensure your user account has read access to this file.

2. **Contacts Framework Access** (Planned): Full contact name resolution requires the app to have Contacts framework entitlements and user permission. The current implementation includes a stub for contact lookup that returns `None` without proper entitlements.

### Troubleshooting

- **Binary not found**: Ensure the `command` path in your MCP configuration points to the actual location of the compiled `imsg-mcp` binary.
- **Database access denied**: Verify that the user running the MCP server has read permissions to `~/Library/Messages/chat.db`.
- **Server not starting**: Check the MCP client's logs for any JSON-RPC errors or initialization failures.

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
