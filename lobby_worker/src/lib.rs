use worker::*;

// Export the Durable Object from server_do
pub use server_do::MatchDO;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/", handle_index)
        .get_async("/create", handle_create)
        .get_async("/join/:code", handle_join)
        .get_async("/ws/:code", handle_websocket)
        .run(req, env)
        .await
}

async fn handle_index(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let html = include_str!("../index.html");
    Response::from_html(html)
}

async fn handle_create(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = generate_match_code();

    // The DO is created lazily on first stub use; fetching the stub here verifies
    // the MATCH binding is configured before handing the code to the client.
    let match_do = ctx.env.durable_object("MATCH")?;
    let _stub = match_do.get_by_name(&code)?;

    Response::from_json(&serde_json::json!({
        "code": code
    }))
}

async fn handle_join(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = ctx.param("code").map_or("", |v| v);

    if code.len() != 5 {
        return Response::error("Invalid match code", 400);
    }

    // Get the MATCH Durable Object namespace
    let match_do = ctx.env.durable_object("MATCH")?;

    // Get DO stub by name (this creates the DO if it doesn't exist)
    let _stub = match_do.get_by_name(code)?;

    // Return response with WebSocket URL
    Response::ok(format!(
        "Match {code} found. Connect via WebSocket at /ws/{code}"
    ))
}

async fn handle_websocket(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = ctx.param("code").map_or("", |v| v);

    if code.len() != 5 {
        return Response::error("Invalid match code", 400);
    }

    let stub = ctx.env.durable_object("MATCH")?.get_by_name(code)?;

    // Ensure request method is GET (required for WebSocket upgrade)
    if req.method() != Method::Get {
        console_error!(
            "Worker: WebSocket upgrade requires GET method, got: {:?}",
            req.method()
        );
        return Response::error("WebSocket upgrade requires GET method", 405);
    }

    // Forward the original Request so the Upgrade/Connection headers reach the DO.
    match stub.fetch_with_request(req).await {
        Ok(resp) => Ok(resp),
        Err(err) => {
            let err_str = format!("{err:?}");
            console_error!(
                "Worker: Error forwarding WebSocket upgrade for code {}: {}",
                code,
                err_str
            );

            // Best-effort detection of the Cloudflare free-tier cap by matching the
            // error's Debug string; the wording is not a stable contract.
            if err_str.contains("Exceeded allowed volume") || err_str.contains("free tier") {
                Response::error(
                    "Service temporarily unavailable due to rate limits. Please try again later.",
                    503,
                )
            } else {
                Response::error(
                    format!("Worker failed to forward WebSocket request: {err_str}"),
                    500,
                )
            }
        }
    }
}

/// Generate a random 5-character match code (A-Z, 0-9)
fn generate_match_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..5)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}
