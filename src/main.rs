use tracing_subscriber::fmt;

mod environment;
mod router;

use environment::Environment;


#[tokio::main]
async fn main() {
    // build our application with a single route

    fmt()
        // .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let env = Environment::build("./templates").await;

    let app = router::router(env);

    // let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Landing running at http://{}/", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
