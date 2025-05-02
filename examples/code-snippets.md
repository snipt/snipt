# Code Snippets with Snipt

This guide shows how to use Snipt to manage and insert code snippets for various programming languages.

## Text Expansion with Colon Trigger for Code

The colon (`:`) trigger is ideal for inserting static code snippets directly without execution. This is perfect for common code patterns, boilerplate, and templates.

### Rust Colon Expansions

**Shortcut**: `fn`
**Content**:
```rust
fn main() {
    
}
```

**Usage**: `:fn` →
```rust
fn main() {
    
}
```

**Shortcut**: `test`
**Content**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test implementation
        assert_eq!(2 + 2, 4);
    }
}
```

**Usage**: `:test` →
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test implementation
        assert_eq!(2 + 2, 4);
    }
}
```

### JavaScript/TypeScript Colon Expansions

**Shortcut**: `react-component`
**Content**:
```jsx
import React from 'react';

const Component = () => {
  return (
    <div>
      
    </div>
  );
};

export default Component;
```

**Usage**: `:react-component` →
```jsx
import React from 'react';

const Component = () => {
  return (
    <div>
      
    </div>
  );
};

export default Component;
```

### Differences Between Colon and Exclamation Mark

- `:shortcut` (colon) inserts the snippet content directly without processing
- `!shortcut` (exclamation mark) executes the snippet content (useful for parameterized snippets)
- For code snippets, use colon when you want template code inserted exactly as written

## Basic Code Snippets (No Parameters)

### Rust Common Imports

**Shortcut**: `rustimports`
**Content**:
```rust
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::thread;
```

**Usage**: `!rustimports` →
```rust
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::thread;
```

### Rust CLI Boilerplate 

**Shortcut**: `rusttokio`
**Content**:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    env_logger::init();
    
    // CLI configuration
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: {} <argument>", args[0]);
        std::process::exit(1);
    }
    
    // Main program logic
    println!("Starting application...");
    
    // Successful completion
    Ok(())
}
```

**Usage**: `!rusttokio` →
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    env_logger::init();
    
    // CLI configuration
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: {} <argument>", args[0]);
        std::process::exit(1);
    }
    
    // Main program logic
    println!("Starting application...");
    
    // Successful completion
    Ok(())
}
```

## Rust Examples

### Rust Error Handling Pattern

**Shortcut**: `rusterr(type)`
**Content**:
```rust
match ${type} {
    Ok(value) => {
        // Handle successful case
        println!("Success: {:?}", value);
    }
    Err(e) => {
        // Handle error case
        eprintln!("Error: {:?}", e);
    }
}
```

**Usage**: `!rusterr(file.read_to_string(&path))` →
```rust
match file.read_to_string(&path) {
    Ok(value) => {
        // Handle successful case
        println!("Success: {:?}", value);
    }
    Err(e) => {
        // Handle error case
        eprintln!("Error: {:?}", e);
    }
}
```

### Rust Custom Error Type Definition

**Shortcut**: `rusterrtype(name)`
**Content**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ${name}Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    
    #[error("Custom error: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, ${name}Error>;
```

**Usage**: `!rusterrtype(App)` →
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    
    #[error("Custom error: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

## JavaScript/TypeScript Examples

### React Component Template

**Shortcut**: `reactcomp(name,props)`
**Content**:
```tsx
import React from 'react';

interface ${name}Props {
  ${props}
}

export const ${name}: React.FC<${name}Props> = ({${props.split(',').map(p => p.trim()).join(', ')}}) => {
  return (
    <div className="${name.toLowerCase()}-container">
      {/* Component content goes here */}
    </div>
  );
};

export default ${name};
```

**Usage**: `!reactcomp(UserProfile,name: string,age: number,isAdmin: boolean)` →
```tsx
import React from 'react';

interface UserProfileProps {
  name: string,
  age: number,
  isAdmin: boolean
}

export const UserProfile: React.FC<UserProfileProps> = ({name, age, isAdmin}) => {
  return (
    <div className="userprofile-container">
      {/* Component content goes here */}
    </div>
  );
};

export default UserProfile;
```

## SQL Examples

### SQL Table Creation

**Shortcut**: `sqltable(name,columns)`
**Content**:
```sql
CREATE TABLE ${name} (
  id SERIAL PRIMARY KEY,
  ${columns},
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

**Usage**: `!sqltable(users,name VARCHAR(255) NOT NULL,email VARCHAR(255) UNIQUE NOT NULL,password_hash TEXT NOT NULL)` →
```sql
CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Python Examples

### Python File Processing Template

**Shortcut**: `pyfile(filename,mode,operation)`
**Content**:
```python
try:
    with open(${filename}, '${mode}') as f:
        ${operation}
except FileNotFoundError:
    print(f"Error: The file {${filename}} was not found.")
except PermissionError:
    print(f"Error: No permission to access {${filename}}.")
except Exception as e:
    print(f"An unexpected error occurred: {e}")
```

**Usage**: `!pyfile("data.txt","r","content = f.read()")` →
```python
try:
    with open("data.txt", 'r') as f:
        content = f.read()
except FileNotFoundError:
    print(f"Error: The file {"data.txt"} was not found.")
except PermissionError:
    print(f"Error: No permission to access {"data.txt"}.")
except Exception as e:
    print(f"An unexpected error occurred: {e}")
```

## Tips for Code Snippets

1. **Create snippets for repetitive boilerplate code** to increase productivity
2. **Parameterize variable parts** of snippets to make them reusable
3. **Include helpful comments** in your snippets to explain their usage
4. **Group related snippets** with similar naming conventions (e.g., `py-` prefix for Python snippets)
5. **Use consistent formatting** in your snippets to maintain code style