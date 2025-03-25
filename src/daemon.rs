use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    // Check if daemon is already running
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    if !pid_dir.exists() {
        fs::create_dir_all(&pid_dir)?;
    }

    let pid_file = pid_dir.join("scribe-daemon.pid");

    if pid_file.exists() {
        let pid_str = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            #[cfg(unix)]
            {
                let status = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .status();

                if status.is_ok() && status.unwrap().success() {
                    println!("scribe daemon is already running with PID {}", pid);
                    return Ok(());
                }
            }

            #[cfg(not(unix))]
            {
                println!("Cannot verify if daemon is running, assuming it's not");
            }
        }
    }

    // Fork to background on Unix systems
    #[cfg(unix)]
    {
        use daemonize::Daemonize;
        println!("Starting scribe daemon in the background");

        let pid_dir = env::var("HOME")
            .map(|home| PathBuf::from(home).join(".scribe"))
            .unwrap_or_else(|_| PathBuf::from(".scribe"));

        let pid_file = pid_dir.join("scribe-daemon.pid");

        // Create a new daemonize process
        let daemonize = Daemonize::new()
            .pid_file(&pid_file)
            .chown_pid_file(true)
            .working_directory("/tmp")
            .stdout(std::fs::File::create("/dev/null")?)
            .stderr(std::fs::File::create("/dev/null")?);

        match daemonize.start() {
            Ok(_) => {
                // We're now in the daemon process
                run_daemon_worker()
            }
            Err(e) => {
                eprintln!("Error starting daemon: {}", e);
                Err(e.into())
            }
        }
    }

    // For non-Unix systems, just continue execution
    #[cfg(not(unix))]
    {
        println!("Starting scribe daemon in the foreground (background not supported on this OS)");
        return run_daemon_worker();
    }
}

// The actual daemon worker process
pub fn run_daemon_worker() -> Result<(), Box<dyn std::error::Error>> {
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let pid_file = pid_dir.join("scribe-daemon.pid");
    let mut file = File::create(&pid_file)?;
    write!(file, "{}", process::id())?;

    let scribe_db_path = pid_dir.join("scribe.json");

    if !scribe_db_path.exists() {
        println!("Scribe database not found at: {:?}", scribe_db_path);
        return Ok(());
    }

    // Load the scribe database
    let scribe_db = load_scribe_db(&scribe_db_path)?;

    // Create a buffer to read keystrokes
    let mut buffer = String::new();
    let (tx, rx) = std::sync::mpsc::channel();
    let tx = Arc::new(Mutex::new(tx));

    let tx_clone = Arc::clone(&tx);
    let handle: JoinHandle<_> = thread::spawn(move || {
        let mut child = Command::new("cat")
            .args(&["/dev/tty"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start cat command");

        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut stderr = BufReader::new(child.stderr.take().unwrap());

        loop {
            if let Some(byte) = read_byte(&mut stdout, &mut stderr) {
                buffer.push(byte as char);
                if let Ok(tx) = tx_clone.lock() {
                    tx.send(buffer.clone()).unwrap();
                }
                buffer.clear(); // Reset the buffer after sending to prevent overlapping
            }
        }
    });

    // Monitor keystrokes and handle ":sc" shortcut
    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(msg) => {
                let mut words: Vec<&str> = msg.split_whitespace().collect();
                while let Some(word) = words.pop() {
                    if word == ":sc" && !words.is_empty() {
                        if let Some(snippet) = get_snippet(&scribe_db, words.last().unwrap()) {
                            insert_snippet(&msg, &snippet, Arc::clone(&tx))?;
                        }
                        break;
                    }
                }
            }
            Err(_) => break,
        }
        thread::sleep(Duration::from_millis(50));
    }

    // Cleanup
    handle.join().unwrap();
    fs::remove_file(&pid_file)?;

    Ok(())
}

fn load_scribe_db(
    path: &Path,
) -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.into())
}

fn get_snippet<'a>(
    scribe_db: &'a serde_json::Map<String, serde_json::Value>,
    word: &str,
) -> Option<&'a serde_json::Value> {
    scribe_db.get(word)
}

fn insert_snippet(
    input: &str,
    snippet: &serde_json::Value,
    tx: Arc<Mutex<std::sync::mpsc::Sender<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pos = input.rfind(':').unwrap();
    let new_input = format!("{} {}", &input[0..pos], snippet);
    tx.lock().unwrap().send(new_input).unwrap();

    Ok(())
}

fn read_byte<R1, R2>(reader: &mut BufReader<R1>, _stderr: &mut BufReader<R2>) -> Option<u8>
where
    R1: std::io::Read,
    R2: std::io::Read,
{
    let mut buf = [0; 1];
    if reader.read_exact(&mut buf).is_ok() {
        Some(buf[0])
    } else {
        None
    }
}

pub fn stop_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let pid_file = pid_dir.join("scribe-daemon.pid");

    if !pid_file.exists() {
        println!("scribe daemon is not running");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    if let Ok(pid) = pid_str.trim().parse::<u32>() {
        // Send termination signal
        #[cfg(unix)]
        {
            let status = std::process::Command::new("kill")
                .arg(pid.to_string())
                .status();

            if status.is_ok() && status.unwrap().success() {
                println!("Stopped scribe daemon with PID {}", pid);
                // Remove PID file
                fs::remove_file(&pid_file)?;
            } else {
                println!("Failed to stop scribe daemon with PID {}", pid);
            }
        }

        // For Windows
        #[cfg(windows)]
        {
            use std::process::Command;
            let status = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .status();

            if status.is_ok() && status.unwrap().success() {
                println!("Stopped scribe daemon with PID {}", pid);
                // Remove PID file
                fs::remove_file(&pid_file)?;
            } else {
                println!("Failed to stop scribe daemon with PID {}", pid);
            }
        }
    } else {
        println!("Invalid PID in daemon file");
    }

    Ok(())
}

pub fn daemon_status() -> Result<(), Box<dyn std::error::Error>> {
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let pid_file = pid_dir.join("scribe-daemon.pid");

    if !pid_file.exists() {
        println!("scribe daemon is not running");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    if let Ok(pid) = pid_str.trim().parse::<u32>() {
        // Check if process is running
        #[cfg(unix)]
        {
            let status = std::process::Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .status();

            if status.is_ok() && status.unwrap().success() {
                println!("scribe daemon is running with PID {}", pid);
            } else {
                println!("scribe daemon is not running (stale PID file)");
                // Remove stale PID file
                fs::remove_file(&pid_file)?;
            }
        }

        // For Windows or fallback
        #[cfg(not(unix))]
        {
            println!("scribe daemon appears to be running with PID {}", pid);
        }
    } else {
        println!("Invalid PID in daemon file");
    }

    Ok(())
}
