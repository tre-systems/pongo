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

    // DIAGNOSTIC (temporary): observability is mid-outage, so surface which DO
    // call fails and its exact error in the response body. Revert with the real fix.
    let ns = match ctx.env.durable_object("MATCH") {
        Ok(ns) => ns,
        Err(e) => return Response::error(format!("DIAG durable_object: {e:?}"), 500),
    };
    let id = match ns.id_from_name(&code) {
        Ok(id) => id,
        Err(e) => return Response::error(format!("DIAG id_from_name: {e:?}"), 500),
    };
    if let Err(e) = id.get_stub() {
        return Response::error(format!("DIAG get_stub: {e:?}"), 500);
    }

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

    // Get DO stub by name. id_from_name + get_stub, not get_by_name/getByName —
    // see handle_create for why getByName is unavailable in production.
    let _stub = match_do.id_from_name(code)?.get_stub()?;

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

    // id_from_name + get_stub, not get_by_name/getByName — see handle_create.
    let stub = ctx
        .env
        .durable_object("MATCH")?
        .id_from_name(code)?
        .get_stub()?;

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
