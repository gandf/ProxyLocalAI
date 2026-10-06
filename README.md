# ProxyIA

English | [Français](README.fr.md)

ProxyIA is a lightweight HTTP proxy that intercepts requests sent to an LLM backend, applies configurable text replacements, and forwards the modified requests to the target server.

It is useful for:
- cleaning or removing system instructions or prompt fragments
- injecting generation parameters on demand
- logging incoming requests and their transformed versions
- automatically retrying requests after an error or an empty response

## Features

### 1. Simple HTTP proxy

The service listens on a configurable local address and forwards requests to a target backend, for example:
- llama.cpp
- vLLM
- another OpenAI-compatible Chat Completions endpoint

### 2. Request replacements

You can define a list of replacement rules in the `proxyia.toml` configuration file.

Each rule supports:
- `from`: text to search for
- `to`: replacement text
- `first_only`: replace only the first occurrence when `true`
- `from_end`: replace the last matching occurrence when `true`

Example:

```toml
[[request_replacements]]
from = "Follow Microsoft content policies.\n"
to = ""
first_only = true
from_end = false
```

The engine only modifies the body of incoming requests before forwarding them to the backend.

### 3. Logging

The proxy can write requests and responses to a log file.

Available options:
- `log_enabled = true|false`
- `log_file = "proxyia.log"`
- `log_max_bytes = 10485760`
- `log_max_files = 5`
- `max_body_log_bytes = 800000`

The log contains:
- the received request
- the request after replacements, if it changed
- any errors and retries
- responses from the backend

Timestamps use the time zone configured on the PC.

### 4. Automatic retries

If the response is empty or a connection error occurs, the proxy can automatically retry several times.

Parameters:
- `max_retries`
- `retry_delay_ms`
- `request_timeout_s`

## Project files

- `proxyia.toml`: main configuration file
- `build.ps1`: builds the release version
- `run.ps1`: checks the executable and then starts the proxy
- `proxyia.exe`: final executable generated in the project root

## Usage

### 1. Configure the proxy

Edit `proxyia.toml` for your environment:

```toml
listen = "127.0.0.1:8000"
target = "192.168.1.50:8000"
```

- `listen`: address where the proxy listens locally
- `target`: address of the LLM backend

### 2. Build the executable

From PowerShell:

```powershell
./build.ps1
```

The executable is copied to the project root:

```text
proxyia.exe
```

### 3. Start the proxy

```powershell
./run.ps1
```

The script:
- checks whether the executable exists
- builds it if necessary
- loads the `proxyia.toml` configuration
- starts the server

## Configuration examples

### Remove a system instruction

```toml
[[request_replacements]]
from = "Follow Microsoft content policies.\n"
to = ""
first_only = true
```

### Remove an instruction always found at the end of a request

```toml
[[request_replacements]]
from = "..."
to = ""
first_only = true
from_end = true
```

### Add a generation parameter

```toml
[[request_replacements]]
from = "\"max_completion_tokens\":"
to = "\"thinking_token_budget\":50000,\"max_completion_tokens\":"
first_only = true
from_end = true
```

## Notes

- Replacements are applied only to incoming requests.
- Logging can be disabled with `log_enabled = false`.
- The project is intended for local use in a development or testing environment with an LLM backend.

## Development

To run the tests:

```powershell
cargo test -- --nocapture
```

To rebuild the final release version:

```powershell
./build.ps1
```