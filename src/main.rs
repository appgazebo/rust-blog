use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
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
    tags: Vec<String>,
    likes: usize,
    created_at: String,
    read_time_minutes: usize,
}

#[derive(Deserialize)]
struct CreatePostRequest {
    title: String,
    author: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct FilterQuery {
    tag: Option<String>,
    search: Option<String>,
}

#[derive(Serialize)]
struct Stats {
    total_posts: usize,
    total_likes: usize,
    tags: Vec<(String, usize)>,
}

#[derive(Clone)]
struct AppState {
    posts: Arc<RwLock<Vec<Post>>>,
    next_id: Arc<RwLock<usize>>,
}

fn estimate_read_time(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    let mins = word_count / 180;
    if mins == 0 { 1 } else { mins }
}

#[tokio::main]
async fn main() {
    let initial_posts = vec![
        Post {
            id: 1,
            title: "Scaling Rust Microservices on Azure for $0/Month".to_string(),
            author: "Patric Kim".to_string(),
            content: "### Why Rust + Azure Container Apps is a Game Changer\n\nTraditional backends in Node.js or Python idle at **150MB - 300MB RAM**, which quickly eats through free tiers. \n\n```rust\n#[tokio::main]\nasync fn main() {\n    println!(\"Rust idles at only 18MB RAM!\");\n}\n```\n\nWith **Azure Container Apps**, we configure `--min-replicas 0`. When no requests arrive, the container sleeps at **$0.00 cost**. When traffic hits, Axum cold-boots in **under 40ms**!\n\n* 2,000,000 monthly requests free\n* Native HTTP/2 & automatic TLS certificates\n* Predictable, rock-solid stability.".to_string(),
            tags: vec!["Rust".to_string(), "Azure".to_string(), "DevOps".to_string()],
            likes: 24,
            created_at: "Today".to_string(),
            read_time_minutes: 2,
        },
        Post {
            id: 2,
            title: "Deep Dive: How Axum Handles 50,000 Concurrent Connections".to_string(),
            author: "Engineering Team".to_string(),
            content: "Axum is built on top of **Tokio**, **Tower**, and **Hyper**.\n\nUnlike thread-per-request architectures, Tokio uses non-blocking cooperative multitasking across an M:N work-stealing threadpool:\n\n1. Zero runtime overhead for routing (compile-time macro expansion)\n2. Memory-safe type extractors (`Json<T>`, `State<S>`, `Path<P>`)\n3. Ergonomic middleware chaining with Tower services.\n\nBenchmark tests show Axum routinely reaching **120,000+ requests/sec** on modest hardware without breaking a sweat.".to_string(),
            tags: vec!["Rust".to_string(), "Axum".to_string(), "Performance".to_string()],
            likes: 42,
            created_at: "Yesterday".to_string(),
            read_time_minutes: 3,
        },
        Post {
            id: 3,
            title: "Zero-Dependency Deployments with Single-Binary Architecture".to_string(),
            author: "Cloud Architect".to_string(),
            content: "The biggest headache in modern web operations is managing dependencies: `node_modules`, Python virtual environments, system glibc incompatibilities.\n\nWith Rust, our entire frontend HTML, CSS, JavaScript, and backend logic are bundled into a single **2.2 MB static ELF executable**.\n\nNo build packs, no Docker inside Docker, just pure portable machine code that runs anywhere.".to_string(),
            tags: vec!["Architecture".to_string(), "Rust".to_string(), "Web".to_string()],
            likes: 19,
            created_at: "Nov 11".to_string(),
            read_time_minutes: 1,
        },
    ];

    let state = AppState {
        posts: Arc::new(RwLock::new(initial_posts)),
        next_id: Arc::new(RwLock::new(4)),
    };

    let app = Router::new()
        .route("/", get(index_page))
        .route("/health", get(health_check))
        .route("/api/stats", get(get_stats))
        .route("/api/posts", get(list_posts).post(create_post))
        .route("/api/posts/{id}", get(get_post).delete(delete_post))
        .route("/api/posts/{id}/like", post(like_post))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Rust Axum Blog running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "rust-axum-blog",
        "runtime": "Rust 1.99 / Tokio / Axum 0.8",
        "cloud": "Azure Container Apps"
    }))
}

async fn get_stats(State(state): State<AppState>) -> Json<Stats> {
    let posts = state.posts.read().unwrap();
    let total_posts = posts.len();
    let total_likes: usize = posts.iter().map(|p| p.likes).sum();

    let mut tag_counts = std::collections::HashMap::new();
    for post in posts.iter() {
        for tag in &post.tags {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
    tags.sort_by(|a, b| b.1.cmp(&a.1));

    Json(Stats {
        total_posts,
        total_likes,
        tags,
    })
}

async fn list_posts(
    State(state): State<AppState>,
    Query(filter): Query<FilterQuery>,
) -> Json<Vec<Post>> {
    let posts = state.posts.read().unwrap();
    let filtered: Vec<Post> = posts
        .iter()
        .filter(|p| {
            if let Some(ref tag) = filter.tag {
                if !tag.is_empty() && !p.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                    return false;
                }
            }
            if let Some(ref q) = filter.search {
                if !q.is_empty() {
                    let search_lower = q.to_lowercase();
                    let match_title = p.title.to_lowercase().contains(&search_lower);
                    let match_author = p.author.to_lowercase().contains(&search_lower);
                    let match_content = p.content.to_lowercase().contains(&search_lower);
                    if !match_title && !match_author && !match_content {
                        return false;
                    }
                }
            }
            true
        })
        .cloned()
        .collect();

    Json(filtered)
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

    let read_time = estimate_read_time(&payload.content);
    let new_post = Post {
        id: new_id,
        title: payload.title,
        author: payload.author,
        read_time_minutes: read_time,
        tags: payload.tags,
        likes: 0,
        content: payload.content,
        created_at: "Just now".to_string(),
    };

    let mut posts = state.posts.write().unwrap();
    posts.insert(0, new_post.clone());

    (StatusCode::CREATED, Json(new_post))
}

async fn like_post(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut posts = state.posts.write().unwrap();
    if let Some(p) = posts.iter_mut().find(|p| p.id == id) {
        p.likes += 1;
        Ok(Json(serde_json::json!({ "id": id, "likes": p.likes })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
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
    <title>Rust Axum Cloud Blog | Azure Container Apps</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
    <style>
        :root {
            --bg: #090d16;
            --surface: #111827;
            --surface-hover: #1f2937;
            --border: #1e293b;
            --accent: #ff6b35;
            --accent-glow: rgba(255, 107, 53, 0.25);
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --text-sub: #64748b;
            --badge-bg: #1e293b;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, sans-serif;
            background: var(--bg);
            color: var(--text-main);
            line-height: 1.6;
            padding-bottom: 4rem;
        }

        /* Top Banner */
        .top-nav {
            background: rgba(17, 24, 39, 0.8);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid var(--border);
            position: sticky;
            top: 0;
            z-index: 50;
        }
        .nav-inner {
            max-width: 960px;
            margin: 0 auto;
            padding: 0.9rem 1.25rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }
        .brand {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-size: 1.2rem;
            font-weight: 800;
            color: var(--text-main);
            text-decoration: none;
        }
        .brand .crab { font-size: 1.5rem; }
        .tag-free {
            background: linear-gradient(135deg, #10b981 0%, #059669 100%);
            color: white;
            font-size: 0.7rem;
            font-weight: 700;
            padding: 0.2rem 0.55rem;
            border-radius: 999px;
            letter-spacing: 0.05em;
            text-transform: uppercase;
        }

        .container { max-width: 960px; margin: 0 auto; padding: 2rem 1.25rem; }

        /* Hero */
        .hero {
            text-align: center;
            padding: 2.5rem 1rem 3rem;
        }
        .hero h1 {
            font-size: 2.75rem;
            font-weight: 800;
            letter-spacing: -0.03em;
            background: linear-gradient(135deg, #ffffff 30%, #94a3b8 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 0.75rem;
        }
        .hero p {
            color: var(--text-muted);
            font-size: 1.15rem;
            max-width: 620px;
            margin: 0 auto 1.75rem;
        }
        .stats-bar {
            display: inline-flex;
            gap: 1.5rem;
            background: var(--surface);
            border: 1px solid var(--border);
            padding: 0.6rem 1.25rem;
            border-radius: 999px;
            font-size: 0.9rem;
            color: var(--text-muted);
        }
        .stats-bar strong { color: var(--accent); }

        /* Controls / Search & Actions */
        .action-bar {
            display: flex;
            gap: 1rem;
            margin-bottom: 2rem;
            flex-wrap: wrap;
        }
        .search-input {
            flex: 1;
            min-width: 260px;
            background: var(--surface);
            border: 1px solid var(--border);
            color: var(--text-main);
            padding: 0.85rem 1.2rem;
            border-radius: 10px;
            font-size: 0.95rem;
            transition: all 0.2s;
        }
        .search-input:focus {
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 0 3px var(--accent-glow);
        }
        .btn {
            background: var(--accent);
            color: white;
            border: none;
            padding: 0.85rem 1.5rem;
            border-radius: 10px;
            font-weight: 700;
            font-size: 0.95rem;
            cursor: pointer;
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            transition: all 0.2s;
        }
        .btn:hover { background: #ff5419; transform: translateY(-1px); }
        .btn-secondary {
            background: var(--surface);
            border: 1px solid var(--border);
            color: var(--text-main);
        }
        .btn-secondary:hover { background: var(--surface-hover); }

        /* Tag Filter Chips */
        .tags-nav {
            display: flex;
            gap: 0.5rem;
            margin-bottom: 2rem;
            overflow-x: auto;
            padding-bottom: 0.5rem;
        }
        .tag-chip {
            background: var(--surface);
            border: 1px solid var(--border);
            color: var(--text-muted);
            padding: 0.4rem 0.9rem;
            border-radius: 999px;
            font-size: 0.85rem;
            font-weight: 600;
            cursor: pointer;
            white-space: nowrap;
            transition: all 0.2s;
        }
        .tag-chip:hover, .tag-chip.active {
            background: var(--accent);
            color: white;
            border-color: var(--accent);
        }

        /* Post Card */
        .posts-grid {
            display: flex;
            flex-direction: column;
            gap: 1.5rem;
        }
        .post-card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 14px;
            padding: 1.75rem;
            transition: all 0.2s;
            position: relative;
        }
        .post-card:hover {
            border-color: #334155;
            box-shadow: 0 10px 30px -10px rgba(0,0,0,0.5);
        }
        .post-header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            gap: 1rem;
            margin-bottom: 0.75rem;
        }
        .post-title {
            font-size: 1.45rem;
            font-weight: 700;
            color: var(--text-main);
            letter-spacing: -0.01em;
        }
        .post-meta {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            color: var(--text-sub);
            font-size: 0.85rem;
            margin-bottom: 1.25rem;
        }
        .author-badge {
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            font-weight: 600;
            color: var(--text-muted);
        }
        .author-avatar {
            width: 24px;
            height: 24px;
            border-radius: 50%;
            background: linear-gradient(135deg, #ff6b35, #f59e0b);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 0.75rem;
            font-weight: 800;
            color: white;
        }
        .post-body {
            color: #cbd5e1;
            font-size: 1rem;
            margin-bottom: 1.5rem;
        }
        .post-body pre {
            background: #0d1117;
            border: 1px solid #30363d;
            padding: 1rem;
            border-radius: 8px;
            overflow-x: auto;
            margin: 1rem 0;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
        }
        .post-body code {
            font-family: 'JetBrains Mono', monospace;
            background: rgba(255,255,255,0.08);
            padding: 0.15rem 0.35rem;
            border-radius: 4px;
            font-size: 0.9em;
        }
        .post-footer {
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-top: 1px solid var(--border);
            padding-top: 1rem;
            flex-wrap: wrap;
            gap: 0.75rem;
        }
        .post-tags {
            display: flex;
            gap: 0.4rem;
            flex-wrap: wrap;
        }
        .tag-pill {
            background: var(--badge-bg);
            color: #38bdf8;
            font-size: 0.78rem;
            font-weight: 600;
            padding: 0.2rem 0.55rem;
            border-radius: 6px;
        }
        .post-actions {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }
        .like-btn {
            background: rgba(255, 107, 53, 0.1);
            color: var(--accent);
            border: 1px solid rgba(255, 107, 53, 0.2);
            padding: 0.4rem 0.8rem;
            border-radius: 8px;
            font-weight: 700;
            font-size: 0.85rem;
            cursor: pointer;
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            transition: all 0.2s;
        }
        .like-btn:hover {
            background: var(--accent);
            color: white;
            transform: scale(1.05);
        }
        .delete-btn {
            background: transparent;
            color: #ef4444;
            border: 1px solid transparent;
            padding: 0.4rem 0.6rem;
            border-radius: 8px;
            font-size: 0.85rem;
            cursor: pointer;
            transition: all 0.2s;
        }
        .delete-btn:hover {
            background: rgba(239, 68, 68, 0.1);
            border-color: rgba(239, 68, 68, 0.2);
        }

        /* Modal */
        .modal-overlay {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0,0,0,0.75);
            backdrop-filter: blur(8px);
            display: none;
            align-items: center;
            justify-content: center;
            z-index: 100;
            padding: 1rem;
        }
        .modal {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 16px;
            width: 100%;
            max-width: 640px;
            max-height: 90vh;
            overflow-y: auto;
            padding: 2rem;
            box-shadow: 0 25px 50px -12px rgba(0,0,0,0.7);
        }
        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1.5rem;
        }
        .modal-title { font-size: 1.4rem; font-weight: 800; }
        .close-btn {
            background: transparent;
            border: none;
            color: var(--text-sub);
            font-size: 1.5rem;
            cursor: pointer;
        }
        .form-group { margin-bottom: 1.25rem; }
        .form-group label {
            display: block;
            font-size: 0.85rem;
            font-weight: 600;
            color: var(--text-muted);
            margin-bottom: 0.4rem;
        }
        .form-control {
            width: 100%;
            background: var(--bg);
            border: 1px solid var(--border);
            color: var(--text-main);
            padding: 0.75rem 1rem;
            border-radius: 8px;
            font-size: 0.95rem;
            font-family: inherit;
        }
        .form-control:focus {
            outline: none;
            border-color: var(--accent);
        }
        textarea.form-control {
            font-family: 'JetBrains Mono', monospace;
            resize: vertical;
            min-height: 140px;
        }
        .tabs {
            display: flex;
            gap: 0.5rem;
            margin-bottom: 0.5rem;
        }
        .tab-btn {
            background: transparent;
            border: none;
            color: var(--text-sub);
            padding: 0.3rem 0.6rem;
            font-size: 0.85rem;
            font-weight: 600;
            cursor: pointer;
            border-radius: 6px;
        }
        .tab-btn.active {
            background: var(--surface-hover);
            color: var(--text-main);
        }
        .preview-box {
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1rem;
            min-height: 140px;
            display: none;
        }
    </style>
</head>
<body>
    <nav class="top-nav">
        <div class="nav-inner">
            <a href="/" class="brand">
                <span class="crab">🦀</span>
                <span>Axum Cloud Blog</span>
            </a>
            <div style="display: flex; align-items: center; gap: 0.75rem;">
                <span class="tag-free">Azure Free Tier</span>
                <button class="btn" onclick="openModal()">+ Write Post</button>
            </div>
        </div>
    </nav>

    <div class="container">
        <header class="hero">
            <h1>High-Speed Rust Web Service</h1>
            <p>Native compiled performance. Serves both frontend and JSON API directly from a single 2.2MB binary in South Africa North.</p>
            <div class="stats-bar" id="stats-bar">
                <span>Loading cluster stats...</span>
            </div>
        </header>

        <div class="action-bar">
            <input type="text" id="search-input" class="search-input" placeholder="Search posts by title, author, or keyword..." oninput="handleSearch()">
        </div>

        <div class="tags-nav" id="tags-nav">
            <button class="tag-chip active" onclick="filterByTag('')">All Topics</button>
        </div>

        <div class="posts-grid" id="posts-container">
            <!-- Dynamic Post Cards -->
        </div>
    </div>

    <!-- Create Post Modal -->
    <div class="modal-overlay" id="modal-overlay" onclick="handleOverlayClick(event)">
        <div class="modal">
            <div class="modal-header">
                <h3 class="modal-title">Publish New Story</h3>
                <button class="close-btn" onclick="closeModal()">&times;</button>
            </div>
            <form onsubmit="handlePublish(event)">
                <div class="form-group">
                    <label>Post Title</label>
                    <input class="form-control" id="post-title" required placeholder="e.g. Building Scalable APIs with Axum">
                </div>
                <div class="form-group">
                    <label>Author</label>
                    <input class="form-control" id="post-author" required placeholder="e.g. Patric Kim">
                </div>
                <div class="form-group">
                    <label>Tags (comma separated)</label>
                    <input class="form-control" id="post-tags" placeholder="Rust, Cloud, Azure, Web">
                </div>
                <div class="form-group">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <label>Content (Markdown Supported)</label>
                        <div class="tabs">
                            <button type="button" class="tab-btn active" id="write-tab" onclick="switchTab('write')">Write</button>
                            <button type="button" class="tab-btn" id="preview-tab" onclick="switchTab('preview')">Preview</button>
                        </div>
                    </div>
                    <textarea class="form-control" id="post-content" required placeholder="Write markdown content here..."></textarea>
                    <div class="preview-box" id="preview-box"></div>
                </div>
                <div style="display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 1.5rem;">
                    <button type="button" class="btn btn-secondary" onclick="closeModal()">Cancel</button>
                    <button type="submit" class="btn">Publish to Azure</button>
                </div>
            </form>
        </div>
    </div>

    <script>
        let currentTag = '';
        let searchQuery = '';

        marked.setOptions({
            highlight: function(code, lang) {
                const language = highlight.getLanguage(lang) ? lang : 'plaintext';
                return hljs.highlight(code, { language }).value;
            }
        });

        async function fetchStats() {
            try {
                const res = await fetch('/api/stats');
                const data = await res.json();
                document.getElementById('stats-bar').innerHTML = `
                    <span>📝 <strong>${data.total_posts}</strong> Articles</span>
                    <span>❤️ <strong>${data.total_likes}</strong> Total Likes</span>
                    <span>⚡ <strong>&lt;40ms</strong> Cold Start</span>
                `;

                const tagsNav = document.getElementById('tags-nav');
                tagsNav.innerHTML = `<button class="tag-chip ${currentTag === '' ? 'active' : ''}" onclick="filterByTag('')">All Topics (${data.total_posts})</button>` +
                    data.tags.map(([t, count]) => `
                        <button class="tag-chip ${currentTag.toLowerCase() === t.toLowerCase() ? 'active' : ''}" onclick="filterByTag('${t}')">
                            #${t} (${count})
                        </button>
                    `).join('');
            } catch (err) {
                console.error('Failed to load stats', err);
            }
        }

        async function loadPosts() {
            const container = document.getElementById('posts-container');
            let url = '/api/posts?';
            if (currentTag) url += `tag=${encodeURIComponent(currentTag)}&`;
            if (searchQuery) url += `search=${encodeURIComponent(searchQuery)}`;

            try {
                const res = await fetch(url);
                const posts = await res.json();

                if (posts.length === 0) {
                    container.innerHTML = `
                        <div style="text-align:center; padding: 4rem 1rem; color: var(--text-sub);">
                            <h3>No articles found</h3>
                            <p>Try searching for another topic or create your first post!</p>
                        </div>
                    `;
                    return;
                }

                container.innerHTML = posts.map(p => {
                    const initials = p.author ? p.author.split(' ').map(n=>n[0]).join('').slice(0, 2).toUpperCase() : '🦀';
                    return `
                        <article class="post-card">
                            <div class="post-header">
                                <h2 class="post-title">${escapeHtml(p.title)}</h2>
                            </div>
                            <div class="post-meta">
                                <span class="author-badge">
                                    <span class="author-avatar">${initials}</span>
                                    <span>${escapeHtml(p.author)}</span>
                                </span>
                                <span>•</span>
                                <span>${p.created_at}</span>
                                <span>•</span>
                                <span>⏱️ ${p.read_time_minutes} min read</span>
                            </div>
                            <div class="post-body">
                                ${marked.parse(p.content)}
                            </div>
                            <div class="post-footer">
                                <div class="post-tags">
                                    ${p.tags.map(t => `<span class="tag-pill">#${escapeHtml(t)}</span>`).join('')}
                                </div>
                                <div class="post-actions">
                                    <button class="like-btn" onclick="likePost(${p.id})">
                                        ❤️ <span>${p.likes}</span>
                                    </button>
                                    <button class="delete-btn" onclick="deletePost(${p.id})">Delete</button>
                                </div>
                            </div>
                        </article>
                    `;
                }).join('');

                hljs.highlightAll();
            } catch (err) {
                container.innerHTML = `<div style="color:#ef4444;">Failed to fetch posts.</div>`;
            }
        }

        function filterByTag(tag) {
            currentTag = tag;
            fetchStats();
            loadPosts();
        }

        let debounceTimeout;
        function handleSearch() {
            clearTimeout(debounceTimeout);
            debounceTimeout = setTimeout(() => {
                searchQuery = document.getElementById('search-input').value.trim();
                loadPosts();
            }, 250);
        }

        async function likePost(id) {
            try {
                const res = await fetch(`/api/posts/${id}/like`, { method: 'POST' });
                if (res.ok) {
                    loadPosts();
                    fetchStats();
                }
            } catch (err) {
                console.error(err);
            }
        }

        async function deletePost(id) {
            if (!confirm('Are you sure you want to delete this post?')) return;
            try {
                const res = await fetch(`/api/posts/${id}`, { method: 'DELETE' });
                if (res.ok) {
                    loadPosts();
                    fetchStats();
                }
            } catch (err) {
                console.error(err);
            }
        }

        async function handlePublish(e) {
            e.preventDefault();
            const title = document.getElementById('post-title').value.trim();
            const author = document.getElementById('post-author').value.trim();
            const rawTags = document.getElementById('post-tags').value.trim();
            const content = document.getElementById('post-content').value.trim();

            const tags = rawTags.split(',').map(t => t.trim()).filter(Boolean);

            try {
                const res = await fetch('/api/posts', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ title, author, tags, content })
                });

                if (res.ok) {
                    closeModal();
                    document.getElementById('post-title').value = '';
                    document.getElementById('post-author').value = '';
                    document.getElementById('post-tags').value = '';
                    document.getElementById('post-content').value = '';
                    loadPosts();
                    fetchStats();
                }
            } catch (err) {
                alert('Error publishing post');
            }
        }

        function openModal() {
            document.getElementById('modal-overlay').style.display = 'flex';
        }

        function closeModal() {
            document.getElementById('modal-overlay').style.display = 'none';
            switchTab('write');
        }

        function handleOverlayClick(e) {
            if (e.target.id === 'modal-overlay') closeModal();
        }

        function switchTab(mode) {
            const writeBtn = document.getElementById('write-tab');
            const prevBtn = document.getElementById('preview-tab');
            const textarea = document.getElementById('post-content');
            const previewBox = document.getElementById('preview-box');

            if (mode === 'write') {
                writeBtn.classList.add('active');
                prevBtn.classList.remove('active');
                textarea.style.display = 'block';
                previewBox.style.display = 'none';
            } else {
                prevBtn.classList.add('active');
                writeBtn.classList.remove('active');
                textarea.style.display = 'none';
                previewBox.style.display = 'block';
                previewBox.innerHTML = marked.parse(textarea.value || '*No content to preview*');
                hljs.highlightAll();
            }
        }

        function escapeHtml(str) {
            return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        }

        // Initialize
        fetchStats();
        loadPosts();
    </script>
</body>
</html>"#)
}
