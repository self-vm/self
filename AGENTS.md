# AGENTS.md

This file contains guidelines and commands for agentic coding agents working on the **self** project - an AI-native virtual machine with the Ego programming language.

## Project Overview

**self** is a Rust-based workspace implementing:
- **CLI** (`selfcli`) - Ego language compiler and command-line interface
- **Core** (`self-vm`) - Virtual machine runtime with native AI integration
- **Emulator** - Bytecode execution environment

The project implements **Ego** (`.ego` files), an AI-first programming language that compiles to bytecode and runs on the self-VM.

## Build Commands

```bash
# Build entire workspace
cargo build

# Build with optimizations
cargo build --release

# Build specific component
cargo build -p selfcli
cargo build -p self-vm
cargo build -p emulator

# Run CLI compiler
cargo run -p selfcli -- [args]

# Run emulator
cargo run -p emulator -- [args]

# Format code
cargo fixfmt
cargo fmtc  # check formatting

# Clean build artifacts
cargo clean
```

## Testing

This project uses manual testing via `.ego` files rather than automated test suites:

```bash
# Run test files manually
cargo run -p selfcli -- run test.ego
cargo run -p selfcli -- run test_bound_access.ego
cargo run -p selfcli -- run main.ego

# Test with emulator
cargo run -p emulator -- [bytecode_file]
```

**Note**: No automated test framework is configured. Tests are manual `.ego` files in the root directory.

## Code Style Guidelines

### Rust Code Style

#### Imports and Modules
- Use `use crate::` for internal imports
- Group external imports separately
- Module organization with `mod.rs` files
- Extensive use of `pub` visibility modifiers

```rust
// Preferred import style
use std::io::{self, Write};

use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    std::{heap_utils::put_string, NativeMember},
    types::{
        object::func::{Engine, Function},
        Value,
    },
    vm::Vm,
};
```

#### Naming Conventions
- **Functions/Variables**: `snake_case`
- **Types/Structs**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`

#### Error Handling
- Custom error enums: `VMError`, `ErrorType`
- Use `Result<T, Error>` for recoverable errors
- Use `throw()` function for fatal errors (exits process)
- Error handling via `std::process::exit(1)`

```rust
// Custom error pattern
pub fn read_line(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    if params.is_empty() {
        return Err(VMError::RuntimeError(
            "read_line requires no parameters".to_string(),
        ));
    }
    // ... implementation
}
```

#### Function Patterns
- Native functions follow three-part pattern:
  1. `*_def()` - Metadata definition
  2. `*_obj()` - Object creation  
  3. `*()` - Native implementation

```rust
pub fn print_line_def() -> NativeMember { /* ... */ }
pub fn print_line_obj() -> MemObject { /* ... */ }
pub fn print_line(vm: &mut Vm, _self: Option<Handle>, params: Vec<Value>, debug: bool) -> Result<Value, VMError> { /* ... */ }
```

### Ego Language Style

#### File Structure
- Import statements at top: `import ai`, `import env`, `import io`
- Function declarations with `fn identifier() {}`
- Use `let` for variable declarations
- Arrow functions: `(params) -> { body }`

#### Code Patterns
```ego
// chain usage
import ai
import env
import io

env.read()

let chain = ai.chain(
    "prompt description",
    "error description"
)

let result = chain.unfold((link) -> {
    println("THOUGHT: ", link.def)
    println(" `- ", link.action.module, ".", link.action.member)
    
    let out = link.action.exec()
    return { continue: true, resolved: out }
})


// function to send emails
fn send_email(to, subject, body) {
    // send an emails with the findings
    let resend_api_key = env.get("RESEND_API_KEY")
    let res = http.post("https://api.resend.com/emails", {
        headers: {
            "content-type": "application/json", 
            "authorization": "Bearer "+resend_api_key
        }, 
        body: json.encode({
            from: "friend@self.computer", 
            to: to, 
            subject: subject,  
            html: body
        })
    })
}

send_email("test@gmail.com", "self test", "<p>Hi, friend</p>")
```

## Architecture Guidelines

### Module Organization
- **CLI**: AST, parser, compiler, commands
- **Core**: VM, instructions, types, runtime, stdlib, error handling
- **Stdlib**: Native modules (ai, io, env, http, fs, etc.)

### Native Module Implementation
When adding new stdlib functions:

1. Create function in `core/src/std/[module]/members.rs`
2. Add to `core/src/std/[module]/mod.rs`
3. Update module exports in `core/src/std/mod.rs`
4. Follow three-part pattern: `*_def()`, `*_obj()`, `*()`

### Error Types
- **VM Level**: `VMError` for runtime errors
- **CLI Level**: `ErrorType` for compilation errors
- **Module-specific**: `AIError`, `IOError`, `NetError`, etc.

## Development Workflow

1. **Build First**: Always run `cargo build` before testing
2. **Manual Testing**: Use `.ego` files for testing functionality
3. **Format Code**: Run `cargo fixfmt` before committing
4. **Cross-Platform**: Consider macOS, Linux, Windows compatibility

## Key Dependencies

- **Runtime**: `tokio` with multi-threaded runtime
- **Web**: `reqwest`, `chromiumoxide` for browser automation
- **Serialization**: `serde`, `serde_json`
- **AI**: `rmcp` for AI model communication
- **WASM**: `wasm-bindgen` for web support

## Important Notes

- **Warning Suppression**: `#![allow(warnings)]` is used in main.rs
- **No Formal Tests**: Manual testing via `.ego` files only
- **AI-First Design**: Native AI integration is a core feature
- **Cross-Platform**: Supports desktop and WebAssembly targets
- **Error Handling**: Fatal errors use process exit, not Result propagation

## File Extensions

- **Rust**: `.rs`
- **Ego Language**: `.ego`
- **Bytecode**: (varies, typically no extension or `.bin`)
- **Configuration**: `.toml` (Cargo), `.env` (environment)

## Common Patterns

### Function Declaration (Ego)
```ego
let result = chain.unfold((link) -> {
    // implementation
    return { continue: true, resolved: out }
})
```

### Native Function (Rust)
```rust
pub fn function_name(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // implementation
}
```

### Module Export
```rust
pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];
    fields.push(("function_name".to_string(), function_name_obj()));
    ("module_name".to_string(), fields)
}
```