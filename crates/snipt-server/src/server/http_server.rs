//! HTTP server implementation for the snipt API.

use crate::api::{
    add_snippet_handler, delete_snippet_handler, get_daemon_details, get_daemon_status,
    get_snippet, get_snippets, update_snippet_handler, DeleteSnippetRequest, GetSnippetRequest,
    SnippetRequest,
};
use crate::server::utils::{port_is_available, save_api_port};

use snipt_core::{get_config_dir, is_daemon_running, SniptError, Result};
use std::fs;
use std::net::SocketAddr;
use warp::Filter;

use super::utils::get_api_server_port;

/// Start the HTTP API server on the specified port
pub async fn start_api_server(port: u16) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Save the port to file so we can find it later
    save_api_port(port)?;

    println!("┌─────────────────────────────────────────┐");
    println!("│          snipt API Server              │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Status: Running                         │");
    println!("│ Port:   {:<33} │", port);
    println!("│ URL:    http://localhost:{:<21} │", port);
    println!("└─────────────────────────────────────────┘");

    // CORS for development
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type"])
        .allow_methods(vec!["GET", "POST", "DELETE", "PUT"]);

    // API routes
    let get_snippets_route = warp::path!("api" / "snippets")
        .and(warp::get())
        .map(|| warp::reply::json(&get_snippets()));

    let get_snippet_route = warp::path!("api" / "snippet")
        .and(warp::get())
        .and(warp::query::<GetSnippetRequest>())
        .map(|query: GetSnippetRequest| warp::reply::json(&get_snippet(&query.shortcut)));

    let add_snippet_route = warp::path!("api" / "snippets")
        .and(warp::post())
        .and(warp::body::json())
        .map(|body: SnippetRequest| {
            warp::reply::json(&add_snippet_handler(body.shortcut, body.snippet))
        });

    let update_snippet_route = warp::path!("api" / "snippets")
        .and(warp::put())
        .and(warp::body::json())
        .map(|body: SnippetRequest| {
            warp::reply::json(&update_snippet_handler(body.shortcut, body.snippet))
        });

    let delete_snippet_route = warp::path!("api" / "snippets")
        .and(warp::delete())
        .and(warp::query::<DeleteSnippetRequest>())
        .map(|query: DeleteSnippetRequest| {
            warp::reply::json(&delete_snippet_handler(query.shortcut))
        });

    let daemon_status_route = warp::path!("api" / "daemon" / "status")
        .and(warp::get())
        .map(|| warp::reply::json(&get_daemon_status()));

    let daemon_details_route = warp::path!("api" / "daemon" / "details")
        .and(warp::get())
        .map(move || warp::reply::json(&get_daemon_details(port)));

    // Health check endpoint
    let health_route = warp::path!("health").map(|| "snipt API is running");

    // Combine routes
    let routes = get_snippets_route
        .or(get_snippet_route)
        .or(add_snippet_route)
        .or(update_snippet_route)
        .or(delete_snippet_route)
        .or(daemon_status_route)
        .or(daemon_details_route)
        .or(health_route)
        .with(cors);

    // Use warp's TcpListener creation to handle binding errors gracefully
    let server = warp::serve(routes).try_bind_with_graceful_shutdown(addr, async {
        // Set up shutdown signal handler
        tokio::signal::ctrl_c().await.ok();
        println!("Received shutdown signal, stopping API server...");
    });

    match server {
        Ok((addr, server)) => {
            println!("API server started successfully on {}", addr);

            // Actually run the server - this will block until shutdown
            server.await;
            Ok(())
        }
        Err(e) => Err(SniptError::Other(format!(
            "Failed to bind to port {}: {}",
            port, e
        ))),
    }
}

/// Check the health of a running API server
pub fn check_api_server_health() -> Result<()> {
    match get_api_server_port() {
        Ok(port) => {
            println!("Checking API server on port {}...", port);

            // Try to connect to the API server using standard TCP
            match std::net::TcpStream::connect(format!("127.0.0.1:{}", port)) {
                Ok(_) => {
                    println!("✅ API server is running on port {}", port);
                    println!("Connection successful (TCP port is open)");
                    Ok(())
                }
                Err(e) => {
                    println!("❌ Failed to connect to API server on port {}", port);
                    println!("Error: {}", e);
                    Err(SniptError::Other(format!(
                        "Failed to connect to API server: {}",
                        e
                    )))
                }
            }
        }
        Err(_) => {
            println!("❌ API server port information not found");
            println!(
                "The API server may not be running or was started without saving port information."
            );
            Err(SniptError::Other(
                "API server port information not found".to_string(),
            ))
        }
    }
}

/// Attempt to stop any running API server process
pub fn stop_api_server() -> Result<()> {
    // Try to get the port
    if let Ok(port) = get_api_server_port() {
        println!("Stopping API server on port {}...", port);

        // Try to connect to signal shutdown
        let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", port));

        // Remove the port file
        let port_file_path = get_config_dir().join("api_port.txt");
        if port_file_path.exists() {
            let _ = fs::remove_file(port_file_path);
        }

        println!("API server port file removed.");

        // Try to kill processes listening on that port (platform-specific)
        #[cfg(unix)]
        {
            use std::process::Command;
            // Try using lsof to find and kill process using that port
            let _ = Command::new("bash")
                .arg("-c")
                .arg(format!("lsof -ti:{} | xargs kill -9", port))
                .status();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            // Try using netstat and taskkill on Windows
            let _ = Command::new("cmd")
                .arg("/C")
                .arg(format!("for /f \"tokens=5\" %a in ('netstat -aon ^| findstr :{} ^| findstr LISTENING') do taskkill /F /PID %a", port))
                .status();
        }
    }

    Ok(())
}

/// Run a diagnostic on the API server
pub fn diagnose_api_server() -> Result<()> {
    println!("snipt API Server Diagnostics");
    println!("============================");

    // Check if daemon is running
    match is_daemon_running()? {
        Some(pid) => println!("✅ Daemon is running with PID {}", pid),
        None => println!("❌ Daemon is not running"),
    }

    // Check if port file exists
    let port_file = get_config_dir().join("api_port.txt");
    if port_file.exists() {
        println!("✅ API port file exists at {}", port_file.display());

        // Check the port
        match std::fs::read_to_string(&port_file) {
            Ok(content) => {
                match content.trim().parse::<u16>() {
                    Ok(port) => {
                        println!("✅ Port file contains valid port: {}", port);

                        // Check if the port is in use
                        match std::net::TcpStream::connect(format!("127.0.0.1:{}", port)) {
                            Ok(_) => {
                                println!("✅ API server is responsive on port {}", port);
                                println!("✅ Server URL: http://localhost:{}", port);
                            }
                            Err(_) => {
                                println!(
                                    "❌ Port {} is not in use - API server may not be running",
                                    port
                                );

                                // Check if the port is available
                                if port_is_available(port) {
                                    println!("✅ Port {} is available", port);
                                    println!("ℹ️  You can start the API server with: snipt serve --port {}", port);
                                } else {
                                    println!(
                                        "❌ Port {} is in use but not responding as API server",
                                        port
                                    );
                                    println!(
                                        "ℹ️  You may need to free this port or choose another one"
                                    );
                                }
                            }
                        }
                    }
                    Err(_) => println!("❌ Port file contains invalid port number"),
                }
            }
            Err(e) => println!("❌ Failed to read port file: {}", e),
        }
    } else {
        println!("❌ API port file does not exist at {}", port_file.display());
        println!("ℹ️  API server may not have been started, or the file was deleted");
    }

    // Check API server logs if they exist
    let log_file = get_config_dir().join("api_server_log.txt");
    if log_file.exists() {
        println!("✅ API server log file exists at {}", log_file.display());

        // Display the last few lines of the log
        #[cfg(unix)]
        {
            use std::process::Command;
            println!("\nLast 10 lines of API server log:");
            let _ = Command::new("sh")
                .arg("-c")
                .arg(format!("tail -n 10 \"{}\"", log_file.display()))
                .status();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            println!("\nLast 5 lines of API server log:");
            let _ = Command::new("cmd")
                .arg("/C")
                .arg(format!(
                    "type \"{}\" | findstr /n . | findstr /r \"[1-5]:\"",
                    log_file.display()
                ))
                .status();
        }
    } else {
        println!(
            "❌ API server log file does not exist at {}",
            log_file.display()
        );
    }

    println!("\nTo start the API server, you can run: snipt serve --port <port>");
    println!("To restart daemon and API: snipt start");
    println!("To stop all services: snipt stop");

    Ok(())
}
