# Charybdis

Go bindings for ScyllaDB/Cassandra benchmarking via Rust FFI. Provides a clean API wrapper around [latte](https://github.com/scylladb/latte) for validating CQL workload scripts and running benchmarks.

## Features

- Validate CQL workload scripts written in Rune
- Get detailed diagnostics for syntax errors and runtime issues
- Comprehensive error reporting with line numbers and suggestions
- Cross-platform support (macOS, Linux)

## Installation

```bash
go get github.com/tentacle-scylla/charybdis
```

## Prerequisites

Charybdis uses Rust FFI under the hood. The compiled static libraries are included for:
- macOS ARM64 (`lib/darwin_arm64/libcharybdis.a`)
- Linux x86_64 (coming soon)

To rebuild the Rust library:

```bash
cd libs/charybdis
cargo build --release -p charybdis-ffi
cp target/release/libcharybdis_ffi.a lib/darwin_arm64/libcharybdis.a
```

## Usage

### Validate CQL workload scripts

```go
import "github.com/tentacle-scylla/charybdis"

// Validate a workload script
result, err := charybdis.ValidateScript("path/to/workload.rn")
if err != nil {
    log.Fatal(err)
}

if !result.IsValid {
    for _, diag := range result.Diagnostics {
        fmt.Printf("%s at line %d: %s\n", diag.Severity, diag.Line, diag.Message)
    }
}
```

### Script validation example

```go
script := `
pub async fn run(ctx, i) {
    ctx.execute("SELECT * FROM system.local").await
}
`

result, err := charybdis.ValidateScriptFromString(script)
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Valid: %v\n", result.IsValid)
for _, diag := range result.Diagnostics {
    fmt.Printf("[%s] %s (line %d)\n", diag.Severity, diag.Message, diag.Line)
}
```

## Architecture

Charybdis consists of three components:

1. **adapter** (`adapter/`) - Rust library providing a clean API around latte-core
2. **ffi** (`ffi/`) - C FFI bindings exposing the adapter to Go
3. **Go bindings** (`bridge.go`, `charybdis.go`, `types.go`) - Go wrapper with idiomatic API

The upstream latte project is maintained in `upstream/` as a git submodule.

## License

Apache 2.0

This project is a derivative work of [latte](https://github.com/scylladb/latte) by Piotr Kołaczkowski, also licensed under Apache 2.0.
