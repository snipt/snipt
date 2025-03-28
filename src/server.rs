use crate::api::{
    api_add_snippet, api_daemon_details, api_daemon_status, api_delete_snippet, api_get_snippet,
    api_get_snippets, api_update_snippet,
};
use crate::error::Result;
use serde::Deserialize;
use std::net::SocketAddr;
use warp::Filter;

#[derive(Deserialize)]
struct AddSnippet {
    shortcut: String,
    snippet: String,
}

#[derive(Deserialize)]
struct DeleteSnippet {
    shortcut: String,
}

#[derive(Deserialize)]
struct GetSnippet {
    shortcut: String,
}

pub async fn start_api_server(port: u16) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Starting API server on http://{}", addr);

    // CORS for development
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type"])
        .allow_methods(vec!["GET", "POST", "DELETE", "PUT"]);

    // API routes
    let get_snippets = warp::path!("api" / "snippets")
        .and(warp::get())
        .map(|| warp::reply::json(&api_get_snippets()));

    let get_snippet = warp::path!("api" / "snippet")
        .and(warp::get())
        .and(warp::query::<GetSnippet>())
        .map(|query: GetSnippet| warp::reply::json(&api_get_snippet(&query.shortcut)));

    let add_snippet = warp::path!("api" / "snippets")
        .and(warp::post())
        .and(warp::body::json())
        .map(|body: AddSnippet| warp::reply::json(&api_add_snippet(body.shortcut, body.snippet)));

    let update_snippet = warp::path!("api" / "snippets")
        .and(warp::put())
        .and(warp::body::json())
        .map(|body: AddSnippet| {
            warp::reply::json(&api_update_snippet(body.shortcut, body.snippet))
        });

    let delete_snippet = warp::path!("api" / "snippets")
        .and(warp::delete())
        .and(warp::query::<DeleteSnippet>())
        .map(|query: DeleteSnippet| warp::reply::json(&api_delete_snippet(query.shortcut)));

    let daemon_status = warp::path!("api" / "daemon" / "status")
        .and(warp::get())
        .map(|| warp::reply::json(&api_daemon_status()));

    let daemon_details = warp::path!("api" / "daemon" / "details")
        .and(warp::get())
        .map(|| warp::reply::json(&api_daemon_details()));

    // Health check endpoint
    let health = warp::path!("health").map(|| "Scribe API is running");

    // Combine routes
    let routes = get_snippets
        .or(get_snippet)
        .or(add_snippet)
        .or(update_snippet)
        .or(delete_snippet)
        .or(daemon_status)
        .or(daemon_details)
        .or(health)
        .with(cors);

    // Start the server
    warp::serve(routes).run(addr).await;

    Ok(())
}
