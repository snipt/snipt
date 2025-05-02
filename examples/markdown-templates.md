# Markdown Templates with Snipt

This guide demonstrates how to create and use markdown templates for common documentation needs.

## Trigger Characters in Snipt

Snipt uses different trigger characters for different expansion purposes:

- `:shortcut` (colon) - For direct text insertion without processing
- `!shortcut` (exclamation mark) - For executing code or parameterized snippets

For most markdown templates, you can choose either trigger depending on your needs:
- Use `:` for static templates you want inserted exactly as written 
- Use `!` when you need parameter substitution or script processing

### Simple Colon Expansion Examples

**Shortcut**: `link`
**Content**:
```
[Link Text](https://example.com)
```

**Usage**: `:link` →
```
[Link Text](https://example.com)
```

**Shortcut**: `img`
**Content**:
```
![Alt text](image.jpg "Image Title")
```

**Usage**: `:img` →
```
![Alt text](image.jpg "Image Title")
```

**Shortcut**: `codeblock`
**Content**:
```
```rust
// Rust code here
```
```

**Usage**: `:codeblock` →
```
```rust
// Rust code here
```
```

## Simple Templates (No Parameters)

### Daily Standup Notes

**Shortcut**: `standup`
**Content**:
```markdown
# Daily Standup - Today's Date

## Yesterday
- 

## Today
- 

## Blockers
- None
```

**Usage**: `!standup` →
```markdown
# Daily Standup - Today's Date

## Yesterday
- 

## Today
- 

## Blockers
- None
```

### Pull Request Template

**Shortcut**: `pr-template`
**Content**:
```markdown
## Description
<!-- Describe your changes in detail -->

## Related Issue
<!-- Please link to the issue here -->

## Motivation and Context
<!-- Why is this change required? What problem does it solve? -->

## How Has This Been Tested?
<!-- Please describe how you tested your changes -->

## Screenshots (if appropriate)

## Types of changes
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)

## Checklist
- [ ] My code follows the code style of this project
- [ ] I have updated the documentation accordingly
- [ ] I have added tests to cover my changes
- [ ] All new and existing tests passed
```

**Usage**: `!pr-template` →
```markdown
## Description
<!-- Describe your changes in detail -->

## Related Issue
<!-- Please link to the issue here -->

## Motivation and Context
<!-- Why is this change required? What problem does it solve? -->

## How Has This Been Tested?
<!-- Please describe how you tested your changes -->

## Screenshots (if appropriate)

## Types of changes
- [ ] My code follows the code style of this project
- [ ] I have updated the documentation accordingly
- [ ] I have added tests to cover my changes
- [ ] All new and existing tests passed
```

## Basic Usage

### Create a Simple Markdown Header Template

1. Create a new snippet with:
   - Shortcut: `mdheader(level,title)`
   - Content:
   ```
   ${'#' * level} ${title}
   ```

2. When you type `!mdheader(3,Installation Guide)`, it will expand to:
   ```
   ### Installation Guide
   ```

## Common Markdown Templates

### Blog Post Template

**Shortcut**: `blogpost(title,author,date)`
**Content**:
```markdown
# ${title}

*By ${author} | ${date}*

## Introduction

Start with a brief introduction here...

## Main Content

Your main content goes here...

## Conclusion

Summarize your post here...

## About the Author

Information about ${author} goes here...
```

**Usage**: `!blogpost(Understanding Rust Ownership,Jane Smith,2023-06-15)` →
```markdown
# Understanding Rust Ownership

*By Jane Smith | 2023-06-15*

## Introduction

Start with a brief introduction here...

## Main Content

Your main content goes here...

## Conclusion

Summarize your post here...

## About the Author

Information about Jane Smith goes here...
```

### Issue Template for GitHub

**Shortcut**: `issue(title,type,description)`
**Content**:
```markdown
# ${title}

## Type
${type}

## Description
${description}

## Steps to Reproduce
1. 
2. 
3. 

## Expected Behavior


## Actual Behavior


## Environment
- OS: 
- Version: 
- Browser (if applicable): 
```

**Usage**: `!issue(App crashes on startup,Bug,The application crashes immediately after launching)` →
```markdown
# App crashes on startup

## Type
Bug

## Description
The application crashes immediately after launching

## Steps to Reproduce
1. 
2. 
3. 

## Expected Behavior


## Actual Behavior


## Environment
- OS: 
- Version: 
- Browser (if applicable): 
```

## Tips for Markdown Templates

1. **Use heading levels as parameters** to create flexible document structures
2. **Create templates for specific audiences** (developers, users, managers)
3. **Consider including placeholders** in your templates for manual completion
4. **Use parameterized lists** for creating structured documentation 