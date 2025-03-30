# snipt API Documentation

This document outlines all available API endpoints for the snipt text expansion tool.

## Base URL

By default, the snipt API server runs on `http://localhost:3000`, but the port may vary if the default port is in use. You can check the current port with:

```bash
snipt port
```

## Health Check

### GET /health

Check if the API server is running.

**Response:**
```
snipt API is running
```

## Snippet Endpoints

### GET /api/snippets

Retrieve all snippets.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "shortcut": "hello",
      "snippet": "Hello, World!",
      "timestamp": "2023-04-25T15:30:45+00:00"
    },
    ...
  ],
  "error": null
}
```

### GET /api/snippet?shortcut={shortcut}

Retrieve a specific snippet by its shortcut.

**Parameters:**
- `shortcut`: The shortcut identifier for the snippet

**Response:**
```json
{
  "success": true,
  "data": {
    "shortcut": "hello",
    "snippet": "Hello, World!",
    "timestamp": "2023-04-25T15:30:45+00:00"
  },
  "error": null
}
```

If the snippet doesn't exist:
```json
{
  "success": true,
  "data": null,
  "error": null
}
```

### POST /api/snippets

Add a new snippet.

**Request Body:**
```json
{
  "shortcut": "hello",
  "snippet": "Hello, World!"
}
```

**Response:**
```json
{
  "success": true,
  "data": null,
  "error": null
}
```

If the shortcut already exists:
```json
{
  "success": false,
  "data": null,
  "error": "Failed to add snippet: Shortcut 'hello' already exists"
}
```

### PUT /api/snippets

Update an existing snippet.

**Request Body:**
```json
{
  "shortcut": "hello",
  "snippet": "Updated content!"
}
```

**Response:**
```json
{
  "success": true,
  "data": null,
  "error": null
}
```

If the shortcut doesn't exist:
```json
{
  "success": false,
  "data": null,
  "error": "Failed to update snippet: Shortcut 'hello' not found"
}
```

### DELETE /api/snippets?shortcut={shortcut}

Delete a snippet by its shortcut.

**Parameters:**
- `shortcut`: The shortcut identifier for the snippet to delete

**Response:**
```json
{
  "success": true,
  "data": null,
  "error": null
}
```

## Daemon Endpoints

### GET /api/daemon/status

Check the status of the snipt daemon.

**Response:**
```json
{
  "success": true,
  "data": true,  // true if running, false if not running
  "error": null
}
```

### GET /api/daemon/details

Get detailed information about the snipt daemon and API server.

**Response:**
```json
{
  "success": true,
  "data": {
    "running": true,
    "pid": 12345,  // Process ID of the daemon, null if not running
    "config_path": "/home/user/.snipt/snipt.json",
    "api_server": {
      "port": 3000,
      "url": "http://localhost:3000"
    }
  },
  "error": null
}
```

## Response Format

All API endpoints return responses in the following format:

```json
{
  "success": boolean,   // true for successful operations, false for failures
  "data": object|null,  // operation result data, structure varies by endpoint
  "error": string|null  // error message when success is false, null otherwise
}
```
