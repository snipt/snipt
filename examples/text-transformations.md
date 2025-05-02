# Text Transformations with Snipt

This guide demonstrates how to create and use text transformation snippets in Snipt.

## Text Expansion with Colon Trigger

Snipt supports two main trigger characters:
- `:` (colon) - For simple text expansion (inserts text directly)
- `!` (exclamation mark) - For executing snippets as scripts

### Basic Colon Expansion Examples

**Shortcut**: `thx`
**Content**:
```
Thank you for your assistance. I really appreciate your help.
```

**Usage**: `:thx` →
```
Thank you for your assistance. I really appreciate your help.
```

**Shortcut**: `addr`
**Content**:
```
123 Main Street
Anytown, CA 94043
United States
```

**Usage**: `:addr` →
```
123 Main Street
Anytown, CA 94043
United States
```

**Shortcut**: `lorem`
**Content**:
```
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
```

**Usage**: `:lorem` →
```
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
```

### When to Use Colon vs. Exclamation Mark

- Use `:` for static text insertion that doesn't require processing (faster and simpler)
- Use `!` when you need to execute code, run scripts, or perform transformations

## Basic Text Transformations

### Simple Text Insertion (No Parameters)

**Shortcut**: `signature`
**Content**:
```
Best regards,
John Smith
Software Engineer
Example Corp.
john.smith@example.com
```

**Usage**: `:signature` →
```
Best regards,
John Smith
Software Engineer
Example Corp.
john.smith@example.com
```

**Shortcut**: `meeting-notes`
**Content**:
```
# Meeting Notes

Date: 
Participants:
- 
- 

## Agenda

1. 
2. 
3. 

## Action Items

- [ ] 
- [ ] 

## Next Steps

```

**Usage**: `:meeting-notes` →
```
# Meeting Notes

Date: 
Participants:
- 
- 

## Agenda

1. 
2. 
3. 

## Action Items

- [ ] 
- [ ] 

## Next Steps

```

### Date and Time (No Parameters)

**Shortcut**: `now`
**Content**:
```bash
#!/bin/bash
date "+%Y-%m-%d %H:%M:%S"
```

**Usage**: `!now` →
```
2023-06-15 14:30:22
```

**Shortcut**: `today`
**Content**:
```bash
#!/bin/bash
date "+%A, %B %d, %Y"
```

**Usage**: `!today` →
```
Thursday, June 15, 2023
```

### Case Conversion

**Shortcut**: `uppercase(text)`
**Content**:
```bash
#!/bin/bash
echo "${text}" | tr '[:lower:]' '[:upper:]'
```

**Usage**: `!uppercase(hello world)` →
```
HELLO WORLD
```

**Shortcut**: `lowercase(text)`
**Content**:
```bash
#!/bin/bash
echo "${text}" | tr '[:upper:]' '[:lower:]'
```

**Usage**: `!lowercase(HELLO WORLD)` →
```
hello world
```

**Shortcut**: `titlecase(text)`
**Content**:
```bash
#!/bin/bash
echo "${text}" | awk '{for(i=1;i<=NF;i++)sub(/./,toupper(substr($i,1,1)),$i)}1'
```

**Usage**: `!titlecase(hello world)` →
```
Hello World
```

## Text Formatting Examples

### CSV to Markdown Table

**Shortcut**: `csv2md(header,data)`
**Content**:
```bash
#!/bin/bash

# Convert CSV to markdown table
HEADER="${header}"
DATA="${data}"

# Create header row
echo "| $(echo $HEADER | sed 's/,/ | /g') |"

# Create separator row
echo "| $(echo $HEADER | sed 's/[^,]*/---/g; s/,/ | /g') |"

# Create data rows
echo "$DATA" | sed 's/^/| /; s/$/ |/; s/,/ | /g'
```

**Usage**: `!csv2md(Name,Age,City,John,30,New York,Jane,25,San Francisco)` →
```
| Name | Age | City |
| --- | --- | --- |
| John | 30 | New York |
| Jane | 25 | San Francisco |
```

### Text Indentation

**Shortcut**: `indent(level,text)`
**Content**:
```bash
#!/bin/bash

# Add indentation to text
LEVEL=${level}
TEXT="${text}"

# Spaces per level
SPACES_PER_LEVEL=2

# Calculate total spaces
TOTAL_SPACES=$((LEVEL * SPACES_PER_LEVEL))

# Create indent string
INDENT=$(printf "%${TOTAL_SPACES}s" "")

# Add indentation to each line
echo "$TEXT" | sed "s/^/$INDENT/"
```

**Usage**: `!indent(2,This is a test\nwith multiple lines)` →
```
    This is a test
    with multiple lines
```

## Advanced Text Processing

### Text Extraction

**Shortcut**: `extract-emails(text)`
**Content**:
```bash
#!/bin/bash
# Extract all email addresses from text
echo "${text}" | grep -Eo '[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}' | sort | uniq
```

**Usage**: `!extract-emails(Contact us at support@example.com or sales@example.com)` →
```
sales@example.com
support@example.com
```

### Word Count

**Shortcut**: `wordcount(text)`
**Content**:
```bash
#!/bin/bash
# Count words, lines, and characters
TEXT="${text}"
WORDS=$(echo "$TEXT" | wc -w)
LINES=$(echo "$TEXT" | wc -l)
CHARS=$(echo "$TEXT" | wc -m)

echo "Word count: $WORDS"
echo "Line count: $LINES"
echo "Character count: $CHARS"
```

**Usage**: `!wordcount(This is a test.\nIt has multiple lines.\nThree lines total.)` →
```
Word count: 11
Line count: 3
Character count: 60
```

## Text Generation

### Lorem Ipsum Generator

**Shortcut**: `lorem(paragraphs)`
**Content**:
```bash
#!/bin/bash
# Generate Lorem Ipsum paragraphs
PARAGRAPHS=${paragraphs}

for i in $(seq 1 $PARAGRAPHS); do
  echo "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
  
  if [ $i -lt $PARAGRAPHS ]; then
    echo ""
  fi
done
```

**Usage**: `!lorem(2)` →
```
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.
```

## Tips for Text Transformations

1. **Chain transformations together** by creating snippets that call other snippets
2. **Use powerful text processing tools** like `sed`, `awk`, and `grep` in your scripts
3. **Create snippets for common text operations** you perform frequently
4. **Consider input validation** to handle edge cases
5. **Document your snippets** with examples to remember how to use them 