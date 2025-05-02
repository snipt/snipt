# Project Documentation with Snipt

This guide demonstrates how to use Snipt to generate and maintain project documentation.

## Basic Documentation Templates

### README Template

**Shortcut**: `readme(name,description)`
**Content**:
```markdown
# ${name}

${description}

## Installation

```shell
# Add installation instructions here
```

## Usage

```rust
// Add example code here
```

## Features

- Feature 1
- Feature 2
- Feature 3

## License

MIT License - See [LICENSE](LICENSE) for details.
```

**Usage**: `!readme(Snipt,A powerful snippet manager for developers)` →
```markdown
# Snipt

A powerful snippet manager for developers

## Installation

```shell
# Add installation instructions here
```

## Usage

```rust
// Add example code here
```

## Features

- Feature 1
- Feature 2
- Feature 3

## License

MIT License - See [LICENSE](LICENSE) for details.
```

## API Documentation

### Rust Module Documentation

**Shortcut**: `rustdoc(module,description)`
**Content**:
```rust
//! # ${module}
//!
//! ${description}
//!
//! ## Examples
//!
//! ```
//! // Add example code here
//! ```

/// A function that does something
///
/// # Examples
///
/// ```
/// let result = ${module}::do_something();
/// assert!(result.is_ok());
/// ```
///
/// # Errors
///
/// This function will return an error if something goes wrong.
pub fn do_something() -> Result<(), Error> {
    // Function implementation here
    Ok(())
}
```

**Usage**: `!rustdoc(Config,Handles configuration loading and saving)` →
```rust
//! # Config
//!
//! Handles configuration loading and saving
//!
//! ## Examples
//!
//! ```
//! // Add example code here
//! ```

/// A function that does something
///
/// # Examples
///
/// ```
/// let result = Config::do_something();
/// assert!(result.is_ok());
/// ```
///
/// # Errors
///
/// This function will return an error if something goes wrong.
pub fn do_something() -> Result<(), Error> {
    // Function implementation here
    Ok(())
}
```

## Project Management Documentation

### Project Architecture Document

**Shortcut**: `architecture(name,components)`
**Content**:
```markdown
# ${name} Architecture

This document describes the high-level architecture of ${name}.

## System Components

${components}

## Data Flow

1. Describe the flow of data between components
2. Explain key interactions

## Technology Stack

- **Backend:** [Technology]
- **Frontend:** [Technology]
- **Database:** [Technology]
- **Deployment:** [Technology]

## Security Considerations

- Authentication
- Authorization
- Data protection

## Performance Considerations

- Caching strategy
- Performance bottlenecks
- Scalability approach
```

**Usage**: `!architecture(Snipt,- Core: handles snippet storage and retrieval\n- Daemon: runs in the background to detect triggers\n- CLI: provides command-line interface\n- UI: provides graphical user interface)` →
```markdown
# Snipt Architecture

This document describes the high-level architecture of Snipt.

## System Components

- Core: handles snippet storage and retrieval
- Daemon: runs in the background to detect triggers
- CLI: provides command-line interface
- UI: provides graphical user interface

## Data Flow

1. Describe the flow of data between components
2. Explain key interactions

## Technology Stack

- **Backend:** [Technology]
- **Frontend:** [Technology]
- **Database:** [Technology]
- **Deployment:** [Technology]

## Security Considerations

- Authentication
- Authorization
- Data protection

## Performance Considerations

- Caching strategy
- Performance bottlenecks
- Scalability approach
```

### API Endpoint Documentation

**Shortcut**: `endpoint(method,path,description,params)`
**Content**:
```markdown
## ${method} ${path}

${description}

### Parameters

${params}

### Response

```json
{
  "status": "success",
  "data": {}
}
```

### Error Responses

| Status Code | Description |
|-------------|-------------|
| 400 | Bad Request |
| 401 | Unauthorized |
| 404 | Not Found |
| 500 | Internal Server Error |

### Example

```bash
curl -X ${method} "https://api.example.com${path}" \\
  -H "Authorization: Bearer <token>" \\
  -H "Content-Type: application/json" \\
  -d '{}'
```
```

**Usage**: `!endpoint(POST,/api/snippets,Create a new snippet,- name: string (required) - Name of the snippet\n- content: string (required) - Content of the snippet\n- tags: string[] (optional) - Tags for the snippet)` →
```markdown
## POST /api/snippets

Create a new snippet

### Parameters

- name: string (required) - Name of the snippet
- content: string (required) - Content of the snippet
- tags: string[] (optional) - Tags for the snippet

### Response

```json
{
  "status": "success",
  "data": {}
}
```

### Error Responses

| Status Code | Description |
|-------------|-------------|
| 400 | Bad Request |
| 401 | Unauthorized |
| 404 | Not Found |
| 500 | Internal Server Error |

### Example

```bash
curl -X POST "https://api.example.com/api/snippets" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{}'
```
```

## Code Standards Documentation

**Shortcut**: `coding-standards(language)`
**Content**:
```markdown
# ${language} Coding Standards

This document outlines our team's coding standards for ${language}.

## Formatting

- Indentation: 4 spaces
- Line length: 100 characters maximum
- Files should end with a single newline

## Naming Conventions

- Variables: camelCase
- Functions: camelCase
- Classes: PascalCase
- Constants: UPPER_SNAKE_CASE
- Files: kebab-case.${language.toLowerCase() === 'typescript' ? 'ts' : language.toLowerCase() === 'javascript' ? 'js' : 'ext'}

## Documentation

- All public APIs should have documentation comments
- Include examples in documentation where appropriate
- Document parameters and return values

## Testing

- Write unit tests for all new functionality
- Aim for >80% code coverage
- Test edge cases and error scenarios

## Error Handling

- Use appropriate error handling mechanisms
- Provide meaningful error messages
- Log errors with appropriate context
```

**Usage**: `!coding-standards(Rust)` →
```markdown
# Rust Coding Standards

This document outlines our team's coding standards for Rust.

## Formatting

- Indentation: 4 spaces
- Line length: 100 characters maximum
- Files should end with a single newline

## Naming Conventions

- Variables: camelCase
- Functions: camelCase
- Classes: PascalCase
- Constants: UPPER_SNAKE_CASE
- Files: kebab-case.ext

## Documentation

- All public APIs should have documentation comments
- Include examples in documentation where appropriate
- Document parameters and return values

## Testing

- Write unit tests for all new functionality
- Aim for >80% code coverage
- Test edge cases and error scenarios

## Error Handling

- Use appropriate error handling mechanisms
- Provide meaningful error messages
- Log errors with appropriate context
```

## Tips for Documentation Templates

1. **Keep documentation templates up to date** with evolving project needs
2. **Use consistent formatting** across all documentation
3. **Parameterize elements that change frequently** in your templates
4. **Include placeholders** for sections that need manual completion
5. **Consider modular documentation** that can be composed from smaller templates 