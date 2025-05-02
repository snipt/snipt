# Parameterized Shortcuts in Snipt

This guide shows you how to define and use snippets with parameters directly in the shortcut.

## Basic Usage

### Define a snippet with parameters in the shortcut name:

1. Create a new snippet with:
   - Shortcut: `sum(a,b)`
   - Content:
   ```bash
   #!/bin/bash
   # Simple addition function
   echo $((${a} + ${b}))
   ```

2. When you type `!sum(5,3)`, it will expand to `8`

## How It Works

- Parameters in shortcuts are defined as placeholders: `shortcut(param1,param2)`
- When you type `!shortcut(value1,value2)`:
  - The system matches your input to the defined shortcut
  - It extracts the values and maps them to the placeholders
  - The snippet content is evaluated with the parameter values

## Parameter Substitution

You can reference parameters in your snippet content in two ways:

1. With braces (recommended for clarity): `${paramName}`
2. Without braces (simpler syntax): `$paramName`

## Examples

### Mathematical Expression Evaluator

**Shortcut**: `calc(expression)`
**Content**:
```bash
#!/bin/bash
echo $(($expression))
```

**Usage**: `!calc(5*10+2)` → `52`

### SQL Query Generator

**Shortcut**: `select(table,columns,condition)`
**Content**:
```sql
SELECT ${columns}
FROM ${table}
WHERE ${condition};
```

**Usage**: `!select(users,name|email,id=42)` → 
```sql
SELECT name|email
FROM users
WHERE id=42;
```

### Greeting Template

**Shortcut**: `hello(name,title)`
**Content**:
```
Dear ${title} ${name},

Thank you for your interest in our services.

Best regards,
The Team
```

**Usage**: `!hello(Smith,Mr.)` →
```
Dear Mr. Smith,

Thank you for your interest in our services.

Best regards,
The Team
```

## Tips for Using Parameterized Shortcuts

1. **Use descriptive parameter names** to make your snippets more maintainable
2. **Consider the default order** of your parameters based on importance
3. **Don't use too many parameters** - 2-4 is usually ideal for usability
4. **For complex scripts**, include parameter documentation as comments

## Advanced Usage: Parameter Transformation

If you need to transform parameters, you can do so in your script:

**Shortcut**: `uppercase(text)`
**Content**:
```bash
#!/bin/bash
echo "${text}" | tr '[:lower:]' '[:upper:]'
```

**Usage**: `!uppercase(hello world)` → `HELLO WORLD` 