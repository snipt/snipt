# URL Commands with Snipt

This guide demonstrates how to use Snipt to open URLs and perform web automation tasks.

## Colon vs. Exclamation Mark for URLs

Snipt allows you to use two different triggers when working with URLs:

- `:shortcut` (colon) - Inserts the URL as text (doesn't open the URL)
- `!shortcut` (exclamation mark) - Opens the URL in your browser

When to use each:
- Use `:` when you want to insert a URL into your document (e.g., when writing emails or documentation)
- Use `!` when you want to actually open the URL in your browser

### URL Insertion with Colon

**Shortcut**: `github`
**Content**:
```
https://github.com
```

**Usage**: `:github` → Inserts "https://github.com" as text

**Shortcut**: `so`
**Content**:
```
https://stackoverflow.com
```

**Usage**: `:so` → Inserts "https://stackoverflow.com" as text

## Basic URL Commands

### Open Website URLs

**Shortcut**: `open-gh`
**Content**:
```
https://github.com
```

**Usage**: `!open-gh` → Opens GitHub in your default browser

### More Common Website Shortcuts (No Parameters)

**Shortcut**: `gmail`
**Content**:
```
https://mail.google.com
```

**Usage**: `!gmail` → Opens Gmail in your default browser

**Shortcut**: `calendar`
**Content**:
```
https://calendar.google.com
```

**Usage**: `!calendar` → Opens Google Calendar in your default browser

**Shortcut**: `jira-dashboard`
**Content**:
```
https://your-company.atlassian.net/jira/dashboards
```

**Usage**: `!jira-dashboard` → Opens your Jira dashboard

**Shortcut**: `confluence`
**Content**:
```
https://your-company.atlassian.net/wiki
```

**Usage**: `!confluence` → Opens your Confluence wiki

**Shortcut**: `rust-playground`
**Content**:
```
https://play.rust-lang.org/
```

**Usage**: `!rust-playground` → Opens the Rust Playground

### Search Queries

**Shortcut**: `google(query)`
**Content**:
```
https://www.google.com/search?q=${query}
```

**Usage**: `!google(rust programming)` → Opens Google search for "rust programming"

**Shortcut**: `stackoverflow(query)`
**Content**:
```
https://stackoverflow.com/search?q=${query}
```

**Usage**: `!stackoverflow(rust error handling)` → Opens Stack Overflow search for "rust error handling"

## Documentation Access

### Language Documentation

**Shortcut**: `rustdoc(topic)`
**Content**:
```
https://doc.rust-lang.org/std/?search=${topic}
```

**Usage**: `!rustdoc(Result)` → Opens Rust documentation search for "Result"

**Shortcut**: `mdn(topic)`
**Content**:
```
https://developer.mozilla.org/en-US/search?q=${topic}
```

**Usage**: `!mdn(fetch api)` → Opens MDN search for "fetch api"

### Package Documentation

**Shortcut**: `crates(package)`
**Content**:
```
https://crates.io/crates/${package}
```

**Usage**: `!crates(tokio)` → Opens crates.io page for "tokio"

**Shortcut**: `npm(package)`
**Content**:
```
https://www.npmjs.com/package/${package}
```

**Usage**: `!npm(react)` → Opens npm page for "react"

## Social Media and Communities

**Shortcut**: `reddit(subreddit)`
**Content**:
```
https://www.reddit.com/r/${subreddit}
```

**Usage**: `!reddit(rust)` → Opens r/rust subreddit

**Shortcut**: `twitter(username)`
**Content**:
```
https://twitter.com/${username}
```

**Usage**: `!twitter(rustlang)` → Opens Twitter profile for "@rustlang"

## Development Resources

### GitHub Repositories and Issues

**Shortcut**: `repo(user,repo)`
**Content**:
```
https://github.com/${user}/${repo}
```

**Usage**: `!repo(rust-lang,rust)` → Opens GitHub repository "rust-lang/rust"

**Shortcut**: `issue(user,repo,number)`
**Content**:
```
https://github.com/${user}/${repo}/issues/${number}
```

**Usage**: `!issue(rust-lang,rust,12345)` → Opens issue #12345 in "rust-lang/rust"

### CI/CD and Project Management

**Shortcut**: `actions(user,repo)`
**Content**:
```
https://github.com/${user}/${repo}/actions
```

**Usage**: `!actions(rust-lang,rust)` → Opens GitHub Actions page for "rust-lang/rust"

**Shortcut**: `jira(project,ticket)`
**Content**:
```
https://your-jira-instance.atlassian.net/browse/${project}-${ticket}
```

**Usage**: `!jira(RUST,123)` → Opens Jira ticket "RUST-123"

## Web Tools and Services

**Shortcut**: `excalidraw`
**Content**:
```
https://excalidraw.com/
```

**Usage**: `!excalidraw` → Opens Excalidraw drawing tool

**Shortcut**: `regex(pattern)`
**Content**:
```
https://regex101.com/?regex=${pattern}
```

**Usage**: `!regex([a-z]+)` → Opens regex101 with pattern "[a-z]+"

## Advanced Browser Scripts

### Automate Browser Forms

**Shortcut**: `login(site,username,password)`
**Content**:
```javascript
javascript:(function(){
  if (window.location.hostname.includes('${site}')) {
    document.querySelector('input[type="email"], input[name="email"], input[id="email"], input[type="text"], input[name="username"], input[id="username"]').value = '${username}';
    document.querySelector('input[type="password"], input[name="password"], input[id="password"]').value = '${password}';
    // Don't auto-submit for security reasons, but focus the submit button
    document.querySelector('button[type="submit"], input[type="submit"]').focus();
  } else {
    alert('Please navigate to ${site} first');
  }
})()
```

**Usage**: `!login(example.com,myuser,mypassword)` → Fills in login form on example.com

## Tips for URL Commands

1. **Use URL encoding** for special characters in URLs
2. **Group related URLs** with similar prefixes (`gh-` for GitHub links)
3. **Consider creating organization-specific shortcuts** for internal tools
4. **Use parameterized URLs** to make them more flexible
5. **Remember that browser scripts run in the context of the current page** 