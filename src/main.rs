use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower_http::cors::CorsLayer;

#[derive(Clone, Serialize, Deserialize)]
struct Post {
    id: usize,
    title: String,
    author: String,
    content: String,
    created_at: String,
}

#[derive(Deserialize)]
struct CreatePostRequest {
    title: String,
    author: String,
    content: String,
}

#[derive(Clone)]
struct AppState {
    posts: Arc<RwLock<Vec<Post>>>,
    next_id: Arc<RwLock<usize>>,
}

#[tokio::main]
async fn main() {
    let initial_posts = vec![
        Post {
            id: 1,
            title: "Welcome to Rust on Azure!".to_string(),
            author: "Kim".to_string(),
            content: "This blog backend is running natively with Axum on Azure Container Apps free tier.".to_string(),
            created_at: "Today".to_string(),
        },
        Post {
            id: 2,
            title: "Why Rust for Cloud?".to_string(),
            author: "Axum Team".to_string(),
            content: "Rust uses ~20MB of memory and boots in milliseconds, making it ideal for scale-to-zero serverless containers.".to_string(),
            created_at: "Today".to_string(),
        },
    ];

    let state = AppState {
        posts: Arc::new(RwLock::new(initial_posts)),
        next_id: Arc::new(RwLock::new(3)),
    };

    let app = Router::new()
        .route("/", get(index_page))
        .route("/health", get(health_check))
        .route("/api/posts", get(list_posts).post(create_post))
        .route("/api/posts/{id}", get(get_post).delete(delete_post))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "axum-blog",
        "runtime": "Rust 1.99"
    }))
}

async fn list_posts(State(state): State<AppState>) -> Json<Vec<Post>> {
    let posts = state.posts.read().unwrap();
    Json(posts.clone())
}

async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Result<Json<Post>, StatusCode> {
    let posts = state.posts.read().unwrap();
    posts
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_post(
    State(state): State<AppState>,
    Json(payload): Json<CreatePostRequest>,
) -> (StatusCode, Json<Post>) {
    let mut id_lock = state.next_id.write().unwrap();
    let new_id = *id_lock;
    *id_lock += 1;

    let new_post = Post {
        id: new_id,
        title: payload.title,
        author: payload.author,
        content: payload.content,
        created_at: "Just now".to_string(),
    };

    let mut posts = state.posts.write().unwrap();
    posts.insert(0, new_post.clone());

    (StatusCode::CREATED, Json(new_post))
}

async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> StatusCode {
    let mut posts = state.posts.write().unwrap();
    if let Some(pos) = posts.iter().position(|p| p.id == id) {
        posts.remove(pos);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn index_page() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rust Axum Blog | Azure</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #0d1117; color: #c9d1d9; padding: 2rem 1rem; }
        .container { max-width: 720px; margin: 0 auto; }
        header { text-align: center; margin-bottom: 2rem; }
        h1 { color: #58a6ff; font-size: 2.2rem; margin-bottom: 0.5rem; }
        .badge { background: #238636; color: white; padding: 0.2rem 0.6rem; border-radius: 12px; font-size: 0.8rem; }
        .card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 1.5rem; margin-bottom: 1.5rem; }
        input, textarea { width: 100%; background: #0d1117; border: 1px solid #30363d; color: #c9d1d9; padding: 0.75rem; border-radius: 6px; margin-bottom: 0.8rem; font-size: 1rem; }
        button { background: #238636; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; font-weight: bold; cursor: pointer; transition: background 0.2s; }
        button:hover { background: #2ea043; }
        .post { border-left: 3px solid #58a6ff; padding-left: 1rem; margin-bottom: 1.5rem; position: relative; }
        .post h3 { color: #f0f6fc; margin-bottom: 0.3rem; }
        .post .meta { font-size: 0.85rem; color: #8b949e; margin-bottom: 0.6rem; }
        .delete-btn { position: absolute; right: 0; top: 0; background: #da3633; padding: 0.3rem 0.6rem; font-size: 0.8rem; }
        .delete-btn:hover { background: #f85149; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🦀 Rust Axum Blog</h1>
            <p>Running on <strong>Azure Container Apps</strong> <span class="badge">Free Tier</span></p>
        </header>

        <div class="card">
            <h3>Publish a New Post</h3>
            <input id="title" placeholder="Post Title" required>
            <input id="author" placeholder="Author Name" required>
            <textarea id="content" rows="3" placeholder="Write your thoughts..." required></textarea>
            <button onclick="publish()">Publish Post</button>
        </div>

        <div id="posts"></div>
    </div>

    <script>
        async function loadPosts() {
            const res = await fetch('/api/posts');
            const posts = await res.json();
            const container = document.getElementById('posts');
            container.innerHTML = posts.map(p => `
                <div class="card post">
                    <button class="delete-btn" onclick="deletePost(${p.id})">Delete</button>
                    <h3>${escapeHtml(p.title)}</h3>
                    <div class="meta">By <strong>${escapeHtml(p.author)}</strong> • ${p.created_at}</div>
                    <p>${escapeHtml(p.content)}</p>
                </div>
            `).join('');
        }

        async function publish() {
            const title = document.getElementById('title').value;
            const author = document.getElementById('author').value;
            const content = document.getElementById('content').value;
            if (!title || !author || !content) return alert('Please fill all fields');

            await fetch('/api/posts', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ title, author, content })
            });

            document.getElementById('title').value = '';
            document.getElementById('author').value = '';
            document.getElementById('content').value = '';
            loadPosts();
        }

        async function deletePost(id) {
            await fetch('/api/posts/' + id, { method: 'DELETE' });
            loadPosts();
        }

        function escapeHtml(str) {
            return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        }

        loadPosts();
    </script>
</body>
</html>"#)
}
