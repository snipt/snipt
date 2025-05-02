# Script Execution with Snipt

This guide demonstrates how to create and execute scripts and commands using Snipt.

## Basic Script Execution

### Creating a Simple Shell Script

1. Create a new snippet with:
   - Shortcut: `date-time`
   - Content:
   ```bash
   #!/bin/bash
   echo "Current date and time: $(date)"
   ```

2. When you type `!date-time`, it will execute the script and insert the result:
   ```
   Current date and time: Thu Jun 15 14:30:22 PDT 2023
   ```

### More Non-Parameterized Script Examples

**Shortcut**: `sys-info`
**Content**:
```bash
#!/bin/bash
echo "=== System Information ==="
echo "Hostname: $(hostname)"
echo "Kernel: $(uname -r)"
echo "OS: $(uname -s)"
echo "CPU: $(grep 'model name' /proc/cpuinfo | head -1 | cut -d':' -f2 | sed 's/^[ \t]*//')"
echo "Memory: $(free -h | grep Mem | awk '{print $2}')"
echo "Uptime: $(uptime -p)"
```

**Usage**: `!sys-info` →
```
=== System Information ===
Hostname: dev-machine
Kernel: 5.15.0-76-generic
OS: Linux
CPU: Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz
Memory: 32Gi
Uptime: up 3 days, 7 hours, 45 minutes
```

**Shortcut**: `git-status`
**Content**:
```bash
#!/bin/bash
echo "=== Git Status ==="
if [ -d .git ] || git rev-parse --git-dir > /dev/null 2>&1; then
  echo "Current branch: $(git branch --show-current)"
  echo "Modified files:"
  git status -s
else
  echo "Not a git repository."
fi
```

**Usage**: `!git-status` →
```
=== Git Status ===
Current branch: main
Modified files:
 M examples/script-execution.md
?? new-file.txt
```

**Shortcut**: `docker-ps`
**Content**:
```bash
#!/bin/bash
echo "=== Running Docker Containers ==="
if command -v docker &> /dev/null; then
  docker ps --format "table {{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}"
else
  echo "Docker is not installed or not in PATH."
fi
```

**Usage**: `!docker-ps` →
```
=== Running Docker Containers ===
NAMES               IMAGE                 STATUS              PORTS
postgres-db         postgres:14           Up 3 days           0.0.0.0:5432->5432/tcp
redis-cache         redis:7               Up 3 days           0.0.0.0:6379->6379/tcp
nginx-proxy         nginx:latest          Up 3 days           0.0.0.0:80->80/tcp, 0.0.0.0:443->443/tcp
```

## Parameterized Script Examples

### File Search Script

**Shortcut**: `find-file(directory,pattern)`
**Content**:
```bash
#!/bin/bash
# Search for files matching a pattern in the specified directory
find ${directory} -type f -name "${pattern}" 2>/dev/null | sort
```

**Usage**: `!find-file(/home/user/projects,*.rs)` →
```
/home/user/projects/snipt/src/main.rs
/home/user/projects/snipt/src/lib.rs
/home/user/projects/snipt/src/models.rs
```

### System Information Script

**Shortcut**: `sysinfo(type)`
**Content**:
```bash
#!/bin/bash
# This script outputs system information
case "${type}" in
  "cpu")
    printf "CPU Information:\n"
    printf "Model: %s\n" "$(sysctl -n machdep.cpu.brand_string)"
    printf "CPU Cores: %s\n" "$(sysctl -n hw.physicalcpu)"
    printf "CPU Threads: %s\n" "$(sysctl -n hw.logicalcpu)"
    ;;
  "memory")
    printf "Memory Information:\n"
    printf "Total Memory: %s GB\n" "$(sysctl -n hw.memsize | awk '{printf "%.2f", $0/1073741824}')"
    vm_stat | perl -ne '/page size of (\d+)/ and $size=$1; /Pages free: (\d+)/ and printf("Free Memory: %.2f GB\n", $1 * $size / 1073741824); /Pages active: (\d+)/ and printf("Active Memory: %.2f GB\n", $1 * $size / 1073741824);'
    ;;
  "disk")
    printf "Disk Usage:\n"
    df -h | awk '{OFS="\t"; if (NR==1) {print $1, $2, $3, $4, $5, $6} else {sub(/%/, "", $5); print $1, $2, $3, $4, $5, $6}}'
    ;;
  "all")
    printf "=== CPU Information ===\n"
    printf "Model: %s\n" "$(sysctl -n machdep.cpu.brand_string)"
    printf "CPU Cores: %s\n" "$(sysctl -n hw.physicalcpu)"
    printf "CPU Threads: %s\n" "$(sysctl -n hw.logicalcpu)"
    
    printf "\n=== Memory Information ===\n"
    printf "Total Memory: %s GB\n" "$(sysctl -n hw.memsize | awk '{printf "%.2f", $0/1073741824}')"
    vm_stat | perl -ne '/page size of (\d+)/ and $size=$1; /Pages free: (\d+)/ and printf("Free Memory: %.2f GB\n", $1 * $size / 1073741824); /Pages active: (\d+)/ and printf("Active Memory: %.2f GB\n", $1 * $size / 1073741824);'
    
    printf "\n=== Disk Usage ===\n"
    df -h | awk '{OFS="\t"; if (NR==1) {print $1, $2, $3, $4, $5, $6} else {sub(/%/, "", $5); print $1, $2, $3, $4, $5, $6}}'
    ;;
  *)
    printf "Unknown system information type: %s\n" "${type}"
    printf "Available options: cpu, memory, disk, all\n"
    ;;
esac
```

**Usage**: `!sysinfo(memory)` →
```
Memory Information:
Total Memory: 16.00 GB
Free Memory: 2.34 GB
Active Memory: 5.67 GB
```

## Network and API Examples

### HTTP Request Script

**Shortcut**: `http-get(url)`
**Content**:
```bash
#!/bin/bash
curl -s "${url}" | jq .
```

**Usage**: `!http-get(https://jsonplaceholder.typicode.com/todos/1)` →
```json
{
  "userId": 1,
  "id": 1,
  "title": "delectus aut autem",
  "completed": false
}
```

### Weather Information

**Shortcut**: `weather(city)`
**Content**:
```bash
#!/bin/bash
# Get weather information for a city using wttr.in
curl -s "wttr.in/${city}?format=3"
```

**Usage**: `!weather(San+Francisco)` →
```
San Francisco: ⛅️  +16°C
```

## Utility Scripts

### Project Generation Script

**Shortcut**: `create-project(name,type)`
**Content**:
```bash
#!/bin/bash
# Create a new project with the specified name and type
mkdir -p "${name}"
cd "${name}"

case "${type}" in
  "rust")
    echo "Creating new Rust project: ${name}"
    cargo init --name "${name}"
    echo "Project created successfully at: $(pwd)"
    ;;
  "node")
    echo "Creating new Node.js project: ${name}"
    npm init -y
    echo "Project created successfully at: $(pwd)"
    ;;
  "python")
    echo "Creating new Python project: ${name}"
    python -m venv .venv
    touch requirements.txt
    mkdir -p "${name}"
    touch "${name}/__init__.py"
    touch README.md
    echo "Project created successfully at: $(pwd)"
    ;;
  *)
    echo "Unknown project type: ${type}"
    echo "Available options: rust, node, python"
    ;;
esac
```

**Usage**: `!create-project(my-app,rust)` →
```
Creating new Rust project: my-app
     Created binary (application) package
Project created successfully at: /home/user/my-app
```

## Tips for Script Execution

1. **Make scripts executable** by adding the shebang line (`#!/bin/bash`)
2. **Handle errors gracefully** by checking for failure conditions
3. **Use parameter validation** to ensure inputs are valid
4. **Provide feedback on execution progress** for longer-running scripts
5. **Consider security implications** when executing scripts with user-provided parameters 