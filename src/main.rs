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
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::cors::CorsLayer;

#[derive(Clone, Serialize, Deserialize)]
struct Topic {
    slug: String,
    name: String,
    description: String,
    icon: String,
    color: String,
    members: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct Comment {
    id: usize,
    post_id: usize,
    parent_id: Option<usize>,
    author: String,
    content: String,
    upvotes: i32,
    downvotes: i32,
    score: i32,
    created_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct CommentNode {
    id: usize,
    post_id: usize,
    parent_id: Option<usize>,
    author: String,
    content: String,
    score: i32,
    created_at: String,
    replies: Vec<CommentNode>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Post {
    id: usize,
    topic_slug: String,
    title: String,
    author: String,
    content: String,
    upvotes: i32,
    downvotes: i32,
    score: i32,
    comment_count: usize,
    created_at: String,
    timestamp: u64,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct CreatePostRequest {
    topic_slug: String,
    title: String,
    author: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct CreateCommentRequest {
    parent_id: Option<usize>,
    author: String,
    content: String,
}

#[derive(Deserialize)]
struct VoteRequest {
    direction: i32, // 1 for upvote, -1 for downvote
}

#[derive(Deserialize)]
struct PostFilter {
    topic: Option<String>,
    sort: Option<String>, // "hot", "new", "top"
    search: Option<String>,
}

#[derive(Serialize)]
struct PostDetailResponse {
    post: Post,
    comments: Vec<CommentNode>,
}

#[derive(Serialize)]
struct StatsResponse {
    total_posts: usize,
    total_comments: usize,
    total_votes: usize,
    active_topics: usize,
}

#[derive(Clone)]
struct AppState {
    topics: Arc<RwLock<Vec<Topic>>>,
    posts: Arc<RwLock<Vec<Post>>>,
    comments: Arc<RwLock<Vec<Comment>>>,
    next_post_id: Arc<RwLock<usize>>,
    next_comment_id: Arc<RwLock<usize>>,
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn build_comment_tree(comments: &[Comment], parent_id: Option<usize>) -> Vec<CommentNode> {
    comments
        .iter()
        .filter(|c| c.parent_id == parent_id)
        .map(|c| {
            let replies = build_comment_tree(comments, Some(c.id));
            CommentNode {
                id: c.id,
                post_id: c.post_id,
                parent_id: c.parent_id,
                author: c.author.clone(),
                content: c.content.clone(),
                score: c.score,
                created_at: c.created_at.clone(),
                replies,
            }
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let now = now_epoch_secs();

    let initial_topics = vec![
        Topic {
            slug: "rust".to_string(),
            name: "r/rust".to_string(),
            description: "A community for systems programmers and fearless concurrency enthusiasts.".to_string(),
            icon: "🦀".to_string(),
            color: "#ff6b35".to_string(),
            members: 4200,
        },
        Topic {
            slug: "cloud".to_string(),
            name: "r/cloud".to_string(),
            description: "Serverless, containers, Azure Container Apps, and microservice infrastructure.".to_string(),
            icon: "☁️".to_string(),
            color: "#38bdf8".to_string(),
            members: 1850,
        },
        Topic {
            slug: "showcase".to_string(),
            name: "r/showcase".to_string(),
            description: "Show off your weekend projects, apps, and codebases built with Rust.".to_string(),
            icon: "🚀".to_string(),
            color: "#10b981".to_string(),
            members: 3100,
        },
        Topic {
            slug: "ask".to_string(),
            name: "r/ask".to_string(),
            description: "Ask questions, debug errors, and exchange knowledge with fellow developers.".to_string(),
            icon: "💡".to_string(),
            color: "#f59e0b".to_string(),
            members: 2400,
        },
    ];

    let initial_posts = vec![
        Post {
            id: 1,
            topic_slug: "rust".to_string(),
            title: "Why Axum is currently the most pleasant backend framework in Rust".to_string(),
            author: "patric_kim".to_string(),
            content: "Over the last few months I tested Actix-web, Rocket, and Axum for high-concurrency microservices. \n\nHere is why Axum won hands-down:\n\n1. **First-class Tokio Ecosystem:** It doesn't fight Tokio's runtime; it's maintained by the same team.\n2. **Type-Safe Extractors:** If a request has invalid JSON or missing headers, Axum rejects it before your handler even runs.\n3. **Tower Middleware:** Reusable rate limiting, compression, and tracing with zero boilerplate.\n\n```rust\nlet app = Router::new()\n    .route(\"/api/v1\", get(handler))\n    .layer(TraceLayer::new_for_http());\n```\n\nWhat has been your experience migrating from Node or Go?".to_string(),
            upvotes: 48,
            downvotes: 2,
            score: 46,
            comment_count: 4,
            created_at: "2 hours ago".to_string(),
            timestamp: now - 7200,
            tags: vec!["Axum".to_string(), "Performance".to_string(), "Web".to_string()],
        },
        Post {
            id: 2,
            topic_slug: "cloud".to_string(),
            title: "Running production Rust microservices on Azure Container Apps for literally $0".to_string(),
            author: "cloud_wizard".to_string(),
            content: "Azure Container Apps provides a free monthly grant:\n\n* **2,000,000 HTTP requests**\n* **180,000 vCPU-seconds**\n* **360,000 GiB-seconds**\n\nBecause our Axum binary uses only **18MB of RAM** and cold boots in **under 40ms**, we set `--min-replicas 0`. When there's no traffic, it costs nothing. When traffic hits, it responds instantaneously.\n\nCombined with GitHub Actions and GitHub Container Registry, the entire deployment pipeline is 100% free.".to_string(),
            upvotes: 63,
            downvotes: 1,
            score: 62,
            comment_count: 2,
            created_at: "4 hours ago".to_string(),
            timestamp: now - 14400,
            tags: vec!["Azure".to_string(), "Serverless".to_string(), "DevOps".to_string()],
        },
        Post {
            id: 3,
            topic_slug: "showcase".to_string(),
            title: "Show r/rust: We built a full Reddit-style threaded community engine in 1 binary".to_string(),
            author: "rust_builder".to_string(),
            content: "Hey everyone! We wanted to see how far we could push a single-binary fullstack Rust app.\n\nFeatures:\n- Threaded nested replies (tree structure)\n- Upvoting/Downvoting with real-time score updates\n- Community/Subreddit routing\n- Live Markdown renderer with syntax highlighting\n- Responsive shadcn-styled dark UI\n\nGive it a try and post your thoughts in the comments below!".to_string(),
            upvotes: 35,
            downvotes: 0,
            score: 35,
            comment_count: 1,
            created_at: "6 hours ago".to_string(),
            timestamp: now - 21600,
            tags: vec!["Project".to_string(), "Showcase".to_string()],
        },
    ];

    let initial_comments = vec![
        Comment {
            id: 1,
            post_id: 1,
            parent_id: None,
            author: "dev_alex".to_string(),
            content: "Totally agree on the extractors. Coming from Express.js where runtime validation requires lots of boilerplate, Axum's compile-time types feel like magic.".to_string(),
            upvotes: 14,
            downvotes: 0,
            score: 14,
            created_at: "1 hour ago".to_string(),
        },
        Comment {
            id: 2,
            post_id: 1,
            parent_id: Some(1),
            author: "patric_kim".to_string(),
            content: "Exactly! And the compiler guarantees you can't accidentally forget to validate a parameter before accessing it.".to_string(),
            upvotes: 8,
            downvotes: 0,
            score: 8,
            created_at: "45 mins ago".to_string(),
        },
        Comment {
            id: 3,
            post_id: 1,
            parent_id: Some(2),
            author: "systems_coder".to_string(),
            content: "Plus Tower layers can be shared across client and server. That code reuse is underrated.".to_string(),
            upvotes: 5,
            downvotes: 0,
            score: 5,
            created_at: "30 mins ago".to_string(),
        },
        Comment {
            id: 4,
            post_id: 1,
            parent_id: None,
            author: "hacker_sam".to_string(),
            content: "How does Axum compare with Actix under heavy synthetic load? Is Actix still faster in TechEmpower benchmarks?".to_string(),
            upvotes: 6,
            downvotes: 1,
            score: 5,
            created_at: "50 mins ago".to_string(),
        },
        Comment {
            id: 5,
            post_id: 2,
            parent_id: None,
            author: "devops_guru".to_string(),
            content: "Scale-to-zero with <40ms cold start is the holy grail. Python Lambdas take 2 seconds easily with dependencies.".to_string(),
            upvotes: 19,
            downvotes: 0,
            score: 19,
            created_at: "3 hours ago".to_string(),
        },
        Comment {
            id: 6,
            post_id: 2,
            parent_id: Some(5),
            author: "cloud_wizard".to_string(),
            content: "And no JVM warm-up penalties either. Rust containers start as fast as native Linux processes.".to_string(),
            upvotes: 9,
            downvotes: 0,
            score: 9,
            created_at: "2 hours ago".to_string(),
        },
        Comment {
            id: 7,
            post_id: 3,
            parent_id: None,
            author: "community_mod".to_string(),
            content: "This UI feels super slick and snappy. Love the nested reply tree!".to_string(),
            upvotes: 7,
            downvotes: 0,
            score: 7,
            created_at: "4 hours ago".to_string(),
        },
    ];

    let state = AppState {
        topics: Arc::new(RwLock::new(initial_topics)),
        posts: Arc::new(RwLock::new(initial_posts)),
        comments: Arc::new(RwLock::new(initial_comments)),
        next_post_id: Arc::new(RwLock::new(4)),
        next_comment_id: Arc::new(RwLock::new(8)),
    };

    let app = Router::new()
        .route("/", get(index_page))
        .route("/health", get(health_check))
        .route("/api/stats", get(get_stats))
        .route("/api/topics", get(list_topics))
        .route("/api/posts", get(list_posts).post(create_post))
        .route("/api/posts/{id}", get(get_post_detail))
        .route("/api/posts/{id}/vote", post(vote_post))
        .route("/api/posts/{id}/comments", post(create_comment))
        .route("/api/comments/{id}/vote", post(vote_comment))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🦀 RustReddit Community Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "rust-reddit-engine",
        "version": "1.0.0",
        "runtime": "Axum 0.8 + Tokio"
    }))
}

async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let posts = state.posts.read().unwrap();
    let comments = state.comments.read().unwrap();
    let topics = state.topics.read().unwrap();

    let total_votes: i32 = posts.iter().map(|p| p.upvotes + p.downvotes).sum::<i32>()
        + comments.iter().map(|c| c.upvotes + c.downvotes).sum::<i32>();

    Json(StatsResponse {
        total_posts: posts.len(),
        total_comments: comments.len(),
        total_votes: total_votes as usize,
        active_topics: topics.len(),
    })
}

async fn list_topics(State(state): State<AppState>) -> Json<Vec<Topic>> {
    let topics = state.topics.read().unwrap();
    Json(topics.clone())
}

async fn list_posts(
    State(state): State<AppState>,
    Query(filter): Query<PostFilter>,
) -> Json<Vec<Post>> {
    let posts = state.posts.read().unwrap();
    let now = now_epoch_secs();

    let mut filtered: Vec<Post> = posts
        .iter()
        .filter(|p| {
            if let Some(ref t) = filter.topic {
                if !t.is_empty() && !t.eq_ignore_ascii_case("all") && !p.topic_slug.eq_ignore_ascii_case(t) {
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

    let sort = filter.sort.unwrap_or_else(|| "hot".to_string());
    match sort.as_str() {
        "new" => filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)),
        "top" => filtered.sort_by(|a, b| b.score.cmp(&a.score)),
        _ => {
            // "hot" algorithm: score with exponential time decay
            filtered.sort_by(|a, b| {
                let age_a = (now.saturating_sub(a.timestamp) / 3600) as f64;
                let age_b = (now.saturating_sub(b.timestamp) / 3600) as f64;
                let hot_a = (a.score as f64) / (age_a + 2.0).powf(1.5);
                let hot_b = (b.score as f64) / (age_b + 2.0).powf(1.5);
                hot_b.partial_cmp(&hot_a).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    Json(filtered)
}

async fn get_post_detail(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Result<Json<PostDetailResponse>, StatusCode> {
    let posts = state.posts.read().unwrap();
    let post = posts.iter().find(|p| p.id == id).cloned().ok_or(StatusCode::NOT_FOUND)?;

    let comments = state.comments.read().unwrap();
    let post_comments: Vec<Comment> = comments
        .iter()
        .filter(|c| c.post_id == id)
        .cloned()
        .collect();

    let comment_tree = build_comment_tree(&post_comments, None);

    Ok(Json(PostDetailResponse {
        post,
        comments: comment_tree,
    }))
}

async fn create_post(
    State(state): State<AppState>,
    Json(payload): Json<CreatePostRequest>,
) -> (StatusCode, Json<Post>) {
    let mut id_lock = state.next_post_id.write().unwrap();
    let new_id = *id_lock;
    *id_lock += 1;

    let now = now_epoch_secs();
    let new_post = Post {
        id: new_id,
        topic_slug: payload.topic_slug,
        title: payload.title,
        author: if payload.author.is_empty() { "anonymous".to_string() } else { payload.author },
        content: payload.content,
        upvotes: 1,
        downvotes: 0,
        score: 1,
        comment_count: 0,
        created_at: "Just now".to_string(),
        timestamp: now,
        tags: payload.tags,
    };

    let mut posts = state.posts.write().unwrap();
    posts.insert(0, new_post.clone());

    (StatusCode::CREATED, Json(new_post))
}

async fn vote_post(
    State(state): State<AppState>,
    Path(id): Path<usize>,
    Json(payload): Json<VoteRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut posts = state.posts.write().unwrap();
    if let Some(p) = posts.iter_mut().find(|p| p.id == id) {
        if payload.direction > 0 {
            p.upvotes += 1;
        } else if payload.direction < 0 {
            p.downvotes += 1;
        }
        p.score = p.upvotes - p.downvotes;
        Ok(Json(serde_json::json!({
            "id": p.id,
            "score": p.score,
            "upvotes": p.upvotes,
            "downvotes": p.downvotes,
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn create_comment(
    State(state): State<AppState>,
    Path(post_id): Path<usize>,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<Comment>), StatusCode> {
    let mut posts = state.posts.write().unwrap();
    let post = posts.iter_mut().find(|p| p.id == post_id).ok_or(StatusCode::NOT_FOUND)?;
    post.comment_count += 1;

    let mut cid_lock = state.next_comment_id.write().unwrap();
    let new_cid = *cid_lock;
    *cid_lock += 1;

    let new_comment = Comment {
        id: new_cid,
        post_id,
        parent_id: payload.parent_id,
        author: if payload.author.is_empty() { "anonymous".to_string() } else { payload.author },
        content: payload.content,
        upvotes: 1,
        downvotes: 0,
        score: 1,
        created_at: "Just now".to_string(),
    };

    let mut comments = state.comments.write().unwrap();
    comments.push(new_comment.clone());

    Ok((StatusCode::CREATED, Json(new_comment)))
}

async fn vote_comment(
    State(state): State<AppState>,
    Path(id): Path<usize>,
    Json(payload): Json<VoteRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut comments = state.comments.write().unwrap();
    if let Some(c) = comments.iter_mut().find(|c| c.id == id) {
        if payload.direction > 0 {
            c.upvotes += 1;
        } else if payload.direction < 0 {
            c.downvotes += 1;
        }
        c.score = c.upvotes - c.downvotes;
        Ok(Json(serde_json::json!({
            "id": c.id,
            "score": c.score,
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn index_page() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RustReddit | High-Performance Community Hub</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
    <style>
        :root {
            --bg: #090d16;
            --surface: #111827;
            --surface-hover: #1e293b;
            --border: #1e293b;
            --border-hover: #334155;
            --accent: #ff6b35;
            --accent-glow: rgba(255, 107, 53, 0.2);
            --upvote: #ff4500;
            --downvote: #7193ff;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --text-sub: #64748b;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, sans-serif;
            background: var(--bg);
            color: var(--text-main);
            line-height: 1.5;
            overflow-x: hidden;
        }

        /* Top Bar */
        .top-navbar {
            background: rgba(17, 24, 39, 0.85);
            backdrop-filter: blur(16px);
            border-bottom: 1px solid var(--border);
            position: sticky;
            top: 0;
            z-index: 100;
        }
        .nav-content {
            max-width: 1200px;
            margin: 0 auto;
            padding: 0.75rem 1.25rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 1.5rem;
        }
        .brand-logo {
            display: flex;
            align-items: center;
            gap: 0.6rem;
            text-decoration: none;
            color: var(--text-main);
            font-weight: 800;
            font-size: 1.25rem;
            letter-spacing: -0.02em;
        }
        .brand-logo span.dot { color: var(--accent); }
        .search-container {
            flex: 1;
            max-width: 520px;
            position: relative;
        }
        .search-bar {
            width: 100%;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 999px;
            padding: 0.6rem 1.2rem;
            font-size: 0.9rem;
            color: var(--text-main);
            transition: all 0.2s;
        }
        .search-bar:focus {
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 0 3px var(--accent-glow);
        }
        .btn-create {
            background: var(--accent);
            color: white;
            border: none;
            border-radius: 999px;
            padding: 0.55rem 1.2rem;
            font-size: 0.88rem;
            font-weight: 700;
            cursor: pointer;
            display: flex;
            align-items: center;
            gap: 0.4rem;
            transition: all 0.2s;
        }
        .btn-create:hover { background: #ff5419; transform: translateY(-1px); }

        /* Main Container */
        .app-layout {
            max-width: 1200px;
            margin: 1.5rem auto;
            padding: 0 1.25rem;
            display: grid;
            grid-template-columns: 1fr 340px;
            gap: 1.75rem;
        }

        /* Feed Column */
        .feed-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 0.6rem 1rem;
            margin-bottom: 1.25rem;
        }
        .sort-tabs { display: flex; gap: 0.4rem; }
        .sort-tab {
            background: transparent;
            border: none;
            color: var(--text-muted);
            padding: 0.45rem 0.9rem;
            border-radius: 8px;
            font-size: 0.88rem;
            font-weight: 600;
            cursor: pointer;
            display: flex;
            align-items: center;
            gap: 0.35rem;
            transition: all 0.15s;
        }
        .sort-tab:hover { background: var(--surface-hover); color: var(--text-main); }
        .sort-tab.active { background: var(--border); color: var(--text-main); font-weight: 700; }
        .active-space-pill {
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            font-size: 0.82rem;
            font-weight: 700;
            color: var(--accent);
            background: rgba(255,107,53,0.12);
            padding: 0.3rem 0.75rem;
            border-radius: 999px;
        }

        /* Post Card (Reddit-Style) */
        .post-card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 12px;
            margin-bottom: 1rem;
            display: flex;
            overflow: hidden;
            transition: border-color 0.15s, box-shadow 0.15s;
            cursor: pointer;
        }
        .post-card:hover {
            border-color: var(--border-hover);
            box-shadow: 0 4px 20px rgba(0,0,0,0.4);
        }
        .vote-column {
            background: rgba(0, 0, 0, 0.2);
            width: 48px;
            padding: 0.75rem 0;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 0.2rem;
            border-right: 1px solid rgba(255,255,255,0.03);
            user-select: none;
        }
        .vote-btn {
            background: transparent;
            border: none;
            color: var(--text-sub);
            font-size: 1.1rem;
            line-height: 1;
            padding: 0.3rem;
            cursor: pointer;
            border-radius: 4px;
            transition: all 0.15s;
        }
        .vote-btn:hover { background: var(--surface-hover); }
        .vote-btn.up:hover, .vote-btn.up.active { color: var(--upvote); }
        .vote-btn.down:hover, .vote-btn.down.active { color: var(--downvote); }
        .vote-count {
            font-size: 0.85rem;
            font-weight: 700;
            color: var(--text-main);
        }

        .post-main {
            flex: 1;
            padding: 1rem 1.25rem;
        }
        .post-meta-row {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.8rem;
            color: var(--text-sub);
            margin-bottom: 0.4rem;
        }
        .space-tag {
            font-weight: 700;
            color: var(--text-main);
            text-decoration: none;
        }
        .space-tag:hover { text-decoration: underline; color: var(--accent); }
        .post-author { color: var(--text-muted); }
        .post-title {
            font-size: 1.25rem;
            font-weight: 700;
            letter-spacing: -0.01em;
            color: var(--text-main);
            margin-bottom: 0.6rem;
            line-height: 1.35;
        }
        .post-snippet {
            color: var(--text-muted);
            font-size: 0.95rem;
            max-height: 90px;
            overflow: hidden;
            position: relative;
            margin-bottom: 0.9rem;
        }
        .post-snippet::after {
            content: "";
            position: absolute;
            bottom: 0; left: 0; right: 0;
            height: 36px;
            background: linear-gradient(to bottom, transparent, var(--surface));
        }
        .post-footer-actions {
            display: flex;
            align-items: center;
            gap: 1rem;
            font-size: 0.82rem;
            font-weight: 600;
            color: var(--text-sub);
        }
        .action-chip {
            display: flex;
            align-items: center;
            gap: 0.35rem;
            background: transparent;
            border: none;
            color: var(--text-sub);
            padding: 0.3rem 0.6rem;
            border-radius: 6px;
            cursor: pointer;
            transition: all 0.15s;
        }
        .action-chip:hover { background: var(--surface-hover); color: var(--text-main); }

        /* Sidebar Column */
        .sidebar { display: flex; flex-direction: column; gap: 1.25rem; }
        .widget {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 1.25rem;
        }
        .widget-title {
            font-size: 0.95rem;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-muted);
            margin-bottom: 1rem;
        }
        .community-item {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0.6rem 0.75rem;
            border-radius: 8px;
            cursor: pointer;
            transition: background 0.15s;
            margin-bottom: 0.25rem;
        }
        .community-item:hover, .community-item.active { background: var(--surface-hover); }
        .comm-info { display: flex; align-items: center; gap: 0.6rem; }
        .comm-icon { font-size: 1.2rem; }
        .comm-name { font-weight: 700; font-size: 0.9rem; color: var(--text-main); }
        .comm-subs { font-size: 0.75rem; color: var(--text-sub); }

        .stat-item {
            display: flex;
            justify-content: space-between;
            padding: 0.5rem 0;
            border-bottom: 1px solid var(--border);
            font-size: 0.88rem;
        }
        .stat-item:last-child { border-bottom: none; }
        .stat-value { font-weight: 700; color: var(--accent); }

        /* Thread Detail Modal / View */
        .thread-modal-overlay {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0, 0, 0, 0.85);
            backdrop-filter: blur(10px);
            z-index: 200;
            display: none;
            overflow-y: auto;
            padding: 2rem 1rem;
        }
        .thread-view {
            max-width: 860px;
            margin: 0 auto;
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 16px;
            box-shadow: 0 25px 60px rgba(0,0,0,0.8);
            position: relative;
        }
        .thread-topbar {
            padding: 1rem 1.5rem;
            border-bottom: 1px solid var(--border);
            display: flex;
            align-items: center;
            justify-content: space-between;
            background: rgba(0,0,0,0.15);
            border-radius: 16px 16px 0 0;
        }
        .btn-close {
            background: transparent;
            border: none;
            color: var(--text-muted);
            font-size: 1.5rem;
            cursor: pointer;
            padding: 0.2rem 0.5rem;
            border-radius: 6px;
        }
        .btn-close:hover { background: var(--surface-hover); color: var(--text-main); }

        .thread-body { padding: 1.5rem 2rem; }
        .op-title { font-size: 1.75rem; font-weight: 800; margin-bottom: 0.75rem; }
        .op-meta { font-size: 0.85rem; color: var(--text-sub); margin-bottom: 1.5rem; }
        .op-content {
            font-size: 1.05rem;
            color: #cbd5e1;
            line-height: 1.7;
            padding-bottom: 1.5rem;
            border-bottom: 1px solid var(--border);
        }
        .op-content pre {
            background: #090d16;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 1rem;
            overflow-x: auto;
            margin: 1rem 0;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
        }
        .op-content code {
            font-family: 'JetBrains Mono', monospace;
            background: rgba(255,255,255,0.08);
            padding: 0.15rem 0.35rem;
            border-radius: 4px;
        }

        /* Comment Composer */
        .composer-section { margin: 1.75rem 0; }
        .composer-textarea {
            width: 100%;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 10px;
            color: var(--text-main);
            padding: 0.85rem 1rem;
            font-family: inherit;
            font-size: 0.95rem;
            min-height: 90px;
            resize: vertical;
        }
        .composer-textarea:focus { outline: none; border-color: var(--accent); }
        .composer-footer {
            display: flex;
            justify-content: flex-end;
            margin-top: 0.6rem;
        }

        /* Nested Comments Tree */
        .comments-section h3 { font-size: 1.15rem; font-weight: 700; margin-bottom: 1.25rem; }
        .comment-node {
            position: relative;
            margin-top: 1rem;
        }
        .comment-inner {
            display: flex;
            gap: 0.75rem;
        }
        .comment-spine {
            width: 2px;
            background: #27272a;
            cursor: pointer;
            transition: background 0.2s;
            margin-left: 12px;
            border-radius: 2px;
        }
        .comment-spine:hover { background: var(--accent); }
        .comment-body-area {
            flex: 1;
            background: rgba(0,0,0,0.15);
            border: 1px solid rgba(255,255,255,0.03);
            border-radius: 10px;
            padding: 0.75rem 1rem;
        }
        .comment-header {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.8rem;
            margin-bottom: 0.4rem;
        }
        .comment-author { font-weight: 700; color: var(--text-main); }
        .comment-author.op { color: var(--accent); }
        .comment-time { color: var(--text-sub); }
        .comment-text {
            color: #cbd5e1;
            font-size: 0.92rem;
            margin-bottom: 0.6rem;
            white-space: pre-wrap;
        }
        .comment-actions {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-size: 0.8rem;
            color: var(--text-sub);
        }
        .comment-vote {
            display: inline-flex;
            align-items: center;
            gap: 0.3rem;
            font-weight: 700;
        }
        .comment-replies {
            margin-left: 1.5rem;
            border-left: 2px solid #27272a;
            padding-left: 1rem;
        }

        .reply-box-active {
            margin-top: 0.75rem;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 0.75rem;
        }

        /* Post Creation Modal */
        .post-modal-overlay {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0,0,0,0.8);
            backdrop-filter: blur(8px);
            z-index: 300;
            display: none;
            align-items: center;
            justify-content: center;
            padding: 1rem;
        }
        .post-modal {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 16px;
            width: 100%;
            max-width: 680px;
            padding: 2rem;
            max-height: 90vh;
            overflow-y: auto;
        }
        .form-group { margin-bottom: 1.2rem; }
        .form-group label {
            display: block;
            font-size: 0.85rem;
            font-weight: 700;
            color: var(--text-muted);
            margin-bottom: 0.4rem;
        }
        .form-select, .form-input, .form-textarea {
            width: 100%;
            background: var(--bg);
            border: 1px solid var(--border);
            color: var(--text-main);
            padding: 0.75rem 1rem;
            border-radius: 8px;
            font-size: 0.95rem;
            font-family: inherit;
        }
        .form-select:focus, .form-input:focus, .form-textarea:focus {
            outline: none;
            border-color: var(--accent);
        }
        .form-textarea { min-height: 160px; font-family: 'JetBrains Mono', monospace; }

        @media (max-width: 840px) {
            .app-layout { grid-template-columns: 1fr; }
            .sidebar { display: none; }
        }
    </style>
</head>
<body>

    <!-- Top Nav -->
    <header class="top-navbar">
        <div class="nav-content">
            <a href="/" class="brand-logo" onclick="selectTopic('all', event)">
                <span>🦀 RustReddit</span>
                <span class="dot">•</span>
                <span style="font-size:0.8rem; font-weight:600; color:var(--text-sub);">Azure Edition</span>
            </a>

            <div class="search-container">
                <input type="text" id="global-search" class="search-bar" placeholder="Search discussions, topics, or authors..." oninput="handleSearch()">
            </div>

            <button class="btn-create" onclick="openCreateModal()">
                <span>+</span>
                <span>Create Topic</span>
            </button>
        </div>
    </header>

    <!-- Main Layout -->
    <div class="app-layout">
        <!-- Feed Column -->
        <main>
            <div class="feed-header">
                <div class="sort-tabs">
                    <button class="sort-tab active" id="sort-hot" onclick="changeSort('hot')">🔥 Hot</button>
                    <button class="sort-tab" id="sort-new" onclick="changeSort('new')">⚡ New</button>
                    <button class="sort-tab" id="sort-top" onclick="changeSort('top')">🏆 Top</button>
                </div>
                <div id="active-space-label" class="active-space-pill">
                    <span>🌐</span>
                    <span id="active-space-name">All Spaces</span>
                </div>
            </div>

            <div id="posts-container">
                <!-- Posts Feed dynamically loaded -->
            </div>
        </main>

        <!-- Sidebar Column -->
        <aside class="sidebar">
            <!-- Active Community Card -->
            <div class="widget" id="community-card">
                <div class="widget-title">About Community</div>
                <h3 id="current-comm-title" style="margin-bottom:0.4rem;">RustReddit Global</h3>
                <p id="current-comm-desc" style="color:var(--text-muted); font-size:0.88rem; margin-bottom:1rem;">
                    The lightning-fast discussion network built with Axum 0.8 and Tokio.
                </p>
                <div class="stat-item">
                    <span style="color:var(--text-sub);">Platform Cost</span>
                    <span class="stat-value">$0.00 / month</span>
                </div>
                <div class="stat-item">
                    <span style="color:var(--text-sub);">Latency / Cold Start</span>
                    <span class="stat-value">&lt; 40ms</span>
                </div>
                <button class="btn-create" style="width:100%; justify-content:center; margin-top:1rem;" onclick="openCreateModal()">
                    Create Post in Space
                </button>
            </div>

            <!-- Explore Communities Widget -->
            <div class="widget">
                <div class="widget-title">Explore Spaces</div>
                <div id="communities-list">
                    <!-- Dynamic communities list -->
                </div>
            </div>

            <!-- Platform Stats Widget -->
            <div class="widget">
                <div class="widget-title">Live Engine Stats</div>
                <div class="stat-item">
                    <span style="color:var(--text-sub);">Discussions</span>
                    <span class="stat-value" id="stat-posts">-</span>
                </div>
                <div class="stat-item">
                    <span style="color:var(--text-sub);">Total Replies</span>
                    <span class="stat-value" id="stat-comments">-</span>
                </div>
                <div class="stat-item">
                    <span style="color:var(--text-sub);">Karma Votes</span>
                    <span class="stat-value" id="stat-votes">-</span>
                </div>
            </div>
        </aside>
    </div>

    <!-- Thread Detail Modal (Reddit Style) -->
    <div class="thread-modal-overlay" id="thread-modal" onclick="handleOverlayClick(event)">
        <div class="thread-view">
            <div class="thread-topbar">
                <div id="thread-header-badge" class="active-space-pill">r/rust</div>
                <button class="btn-close" onclick="closeThread()">&times;</button>
            </div>
            <div class="thread-body">
                <div id="thread-op-area">
                    <!-- OP loaded dynamically -->
                </div>

                <!-- Top level comment composer -->
                <div class="composer-section">
                    <textarea class="composer-textarea" id="top-comment-input" placeholder="Join the discussion... (Markdown supported)"></textarea>
                    <div class="composer-footer">
                        <button class="btn-create" onclick="submitTopComment()">Post Comment</button>
                    </div>
                </div>

                <!-- Threaded comments section -->
                <div class="comments-section">
                    <h3 id="comments-count-header">Comments</h3>
                    <div id="thread-comments-container">
                        <!-- Nested comments tree -->
                    </div>
                </div>
            </div>
        </div>
    </div>

    <!-- Create Post Modal -->
    <div class="post-modal-overlay" id="create-modal" onclick="handleCreateOverlayClick(event)">
        <div class="post-modal">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:1.5rem;">
                <h2 style="font-weight:800; font-size:1.35rem;">Create a Post</h2>
                <button class="btn-close" onclick="closeCreateModal()">&times;</button>
            </div>
            <form onsubmit="handlePostSubmit(event)">
                <div class="form-group">
                    <label>Choose a Space</label>
                    <select class="form-select" id="post-space-select" required>
                        <!-- Dynamic options -->
                    </select>
                </div>
                <div class="form-group">
                    <label>Title</label>
                    <input class="form-input" id="post-title-input" placeholder="An interesting title that starts a discussion..." required>
                </div>
                <div class="form-group">
                    <label>Your Username</label>
                    <input class="form-input" id="post-author-input" placeholder="e.g. patric_kim (optional)">
                </div>
                <div class="form-group">
                    <label>Tags (comma separated)</label>
                    <input class="form-input" id="post-tags-input" placeholder="Rust, Web, Cloud">
                </div>
                <div class="form-group">
                    <label>Body (Markdown & Code supported)</label>
                    <textarea class="form-textarea" id="post-content-input" placeholder="What are your thoughts, benchmarks, or code snippets?" required></textarea>
                </div>
                <div style="display:flex; justify-content:flex-end; gap:0.75rem; margin-top:1.5rem;">
                    <button type="button" class="action-chip" style="font-size:0.9rem;" onclick="closeCreateModal()">Cancel</button>
                    <button type="submit" class="btn-create">Publish Topic</button>
                </div>
            </form>
        </div>
    </div>

    <script>
        let currentTopic = 'all';
        let currentSort = 'hot';
        let searchQuery = '';
        let currentPostId = null;
        let topicsData = [];

        marked.setOptions({
            highlight: function(code, lang) {
                const language = highlight.getLanguage(lang) ? lang : 'plaintext';
                return hljs.highlight(code, { language }).value;
            }
        });

        async function init() {
            await fetchTopics();
            await fetchStats();
            await loadFeed();
        }

        async function fetchTopics() {
            try {
                const res = await fetch('/api/topics');
                topicsData = await res.json();

                const select = document.getElementById('post-space-select');
                select.innerHTML = topicsData.map(t => `<option value="${t.slug}">${t.icon} ${t.name}</option>`).join('');

                const list = document.getElementById('communities-list');
                list.innerHTML = `
                    <div class="community-item ${currentTopic === 'all' ? 'active' : ''}" onclick="selectTopic('all', event)">
                        <div class="comm-info">
                            <span class="comm-icon">🌐</span>
                            <div>
                                <div class="comm-name">All Communities</div>
                                <div class="comm-subs">Global aggregate feed</div>
                            </div>
                        </div>
                    </div>
                ` + topicsData.map(t => `
                    <div class="community-item ${currentTopic === t.slug ? 'active' : ''}" onclick="selectTopic('${t.slug}', event)">
                        <div class="comm-info">
                            <span class="comm-icon">${t.icon}</span>
                            <div>
                                <div class="comm-name">${t.name}</div>
                                <div class="comm-subs">${t.members.toLocaleString()} members</div>
                            </div>
                        </div>
                    </div>
                `).join('');
            } catch (err) {
                console.error(err);
            }
        }

        async function fetchStats() {
            try {
                const res = await fetch('/api/stats');
                const data = await res.json();
                document.getElementById('stat-posts').innerText = data.total_posts;
                document.getElementById('stat-comments').innerText = data.total_comments;
                document.getElementById('stat-votes').innerText = data.total_votes;
            } catch (err) {}
        }

        async function loadFeed() {
            const container = document.getElementById('posts-container');
            let url = `/api/posts?sort=${currentSort}&`;
            if (currentTopic !== 'all') url += `topic=${encodeURIComponent(currentTopic)}&`;
            if (searchQuery) url += `search=${encodeURIComponent(searchQuery)}`;

            try {
                const res = await fetch(url);
                const posts = await res.json();

                if (posts.length === 0) {
                    container.innerHTML = `
                        <div style="background:var(--surface); border:1px solid var(--border); border-radius:12px; padding:3.5rem 1rem; text-align:center; color:var(--text-muted);">
                            <div style="font-size:2rem; margin-bottom:0.5rem;">🦗</div>
                            <h3>No discussions yet in this space</h3>
                            <p style="font-size:0.9rem; margin-top:0.3rem;">Be the pioneer and start the first topic!</p>
                            <button class="btn-create" style="margin:1rem auto 0;" onclick="openCreateModal()">Start Topic</button>
                        </div>
                    `;
                    return;
                }

                container.innerHTML = posts.map(p => `
                    <article class="post-card" onclick="openThread(${p.id})">
                        <div class="vote-column" onclick="event.stopPropagation()">
                            <button class="vote-btn up" onclick="votePost(${p.id}, 1)">▲</button>
                            <span class="vote-count" id="post-score-${p.id}">${p.score}</span>
                            <button class="vote-btn down" onclick="votePost(${p.id}, -1)">▼</button>
                        </div>
                        <div class="post-main">
                            <div class="post-meta-row">
                                <span class="space-tag" onclick="selectTopic('${p.topic_slug}', event)">r/${p.topic_slug}</span>
                                <span>•</span>
                                <span class="post-author">Posted by u/${escapeHtml(p.author)}</span>
                                <span>•</span>
                                <span>${p.created_at}</span>
                            </div>
                            <h2 class="post-title">${escapeHtml(p.title)}</h2>
                            <div class="post-snippet">${marked.parse(p.content.slice(0, 240))}</div>
                            <div class="post-footer-actions">
                                <button class="action-chip">
                                    <span>💬</span>
                                    <span>${p.comment_count} comments</span>
                                </button>
                                <button class="action-chip" onclick="copyPostLink(${p.id}, event)">
                                    <span>🔗</span>
                                    <span>Share</span>
                                </button>
                            </div>
                        </div>
                    </article>
                `).join('');

            } catch (err) {
                container.innerHTML = `<div style="color:#ef4444;">Failed to load feed.</div>`;
            }
        }

        async function votePost(postId, direction) {
            try {
                const res = await fetch(`/api/posts/${postId}/vote`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ direction })
                });
                if (res.ok) {
                    const data = await res.json();
                    const el = document.getElementById(`post-score-${postId}`);
                    if (el) el.innerText = data.score;
                    const opEl = document.getElementById(`op-score-${postId}`);
                    if (opEl) opEl.innerText = data.score;
                    fetchStats();
                }
            } catch (err) {}
        }

        async function openThread(postId) {
            currentPostId = postId;
            const modal = document.getElementById('thread-modal');
            modal.style.display = 'block';
            document.body.style.overflow = 'hidden';

            const opArea = document.getElementById('thread-op-area');
            const commentsContainer = document.getElementById('thread-comments-container');
            opArea.innerHTML = `<div style="color:var(--text-muted); padding:2rem 0;">Loading discussion...</div>`;
            commentsContainer.innerHTML = '';

            try {
                const res = await fetch(`/api/posts/${postId}`);
                if (!res.ok) throw new Error();
                const data = await res.json();
                const p = data.post;

                document.getElementById('thread-header-badge').innerText = `r/${p.topic_slug}`;
                document.getElementById('comments-count-header').innerText = `${p.comment_count} Comments`;

                opArea.innerHTML = `
                    <div style="display:flex; gap:1.25rem;">
                        <div class="vote-column" style="background:transparent; border:none; padding:0;">
                            <button class="vote-btn up" onclick="votePost(${p.id}, 1)">▲</button>
                            <span class="vote-count" id="op-score-${p.id}">${p.score}</span>
                            <button class="vote-btn down" onclick="votePost(${p.id}, -1)">▼</button>
                        </div>
                        <div style="flex:1;">
                            <div class="post-meta-row">
                                <span class="space-tag">r/${p.topic_slug}</span>
                                <span>•</span>
                                <span>Posted by u/${escapeHtml(p.author)}</span>
                                <span>•</span>
                                <span>${p.created_at}</span>
                            </div>
                            <h1 class="op-title">${escapeHtml(p.title)}</h1>
                            <div class="op-content">${marked.parse(p.content)}</div>
                        </div>
                    </div>
                `;

                commentsContainer.innerHTML = renderCommentTree(data.comments, p.author);
                hljs.highlightAll();

            } catch (err) {
                opArea.innerHTML = `<div style="color:#ef4444;">Failed to load thread.</div>`;
            }
        }

        function renderCommentTree(comments, opAuthor) {
            if (!comments || comments.length === 0) {
                return `<div style="color:var(--text-sub); font-size:0.9rem; padding:1.5rem 0;">No comments yet. Start the conversation!</div>`;
            }

            return comments.map(c => {
                const isOp = c.author.toLowerCase() === opAuthor.toLowerCase();
                return `
                    <div class="comment-node" id="comment-node-${c.id}">
                        <div class="comment-inner">
                            <div class="comment-spine" onclick="toggleCollapse(${c.id})"></div>
                            <div class="comment-body-area">
                                <div class="comment-header">
                                    <span class="comment-author ${isOp ? 'op' : ''}">u/${escapeHtml(c.author)} ${isOp ? '[OP]' : ''}</span>
                                    <span>•</span>
                                    <span class="comment-time">${c.created_at}</span>
                                </div>
                                <div class="comment-text">${escapeHtml(c.content)}</div>
                                <div class="comment-actions">
                                    <div class="comment-vote">
                                        <button class="vote-btn up" style="font-size:0.9rem;" onclick="voteComment(${c.id}, 1)">▲</button>
                                        <span id="comment-score-${c.id}">${c.score}</span>
                                        <button class="vote-btn down" style="font-size:0.9rem;" onclick="voteComment(${c.id}, -1)">▼</button>
                                    </div>
                                    <span>•</span>
                                    <button class="action-chip" onclick="openInlineReply(${c.id})">💬 Reply</button>
                                </div>

                                <div id="reply-box-${c.id}"></div>
                            </div>
                        </div>

                        ${c.replies && c.replies.length > 0 ? `
                            <div class="comment-replies" id="replies-for-${c.id}">
                                ${renderCommentTree(c.replies, opAuthor)}
                            </div>
                        ` : ''}
                    </div>
                `;
            }).join('');
        }

        function toggleCollapse(id) {
            const replies = document.getElementById(`replies-for-${id}`);
            if (replies) {
                replies.style.display = replies.style.display === 'none' ? 'block' : 'none';
            }
        }

        function openInlineReply(commentId) {
            const container = document.getElementById(`reply-box-${commentId}`);
            if (container.innerHTML) {
                container.innerHTML = '';
                return;
            }

            container.innerHTML = `
                <div class="reply-box-active">
                    <textarea class="composer-textarea" id="reply-input-${commentId}" placeholder="Write your reply..." style="min-height:70px;"></textarea>
                    <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:0.5rem;">
                        <button class="action-chip" onclick="this.parentElement.parentElement.remove()">Cancel</button>
                        <button class="btn-create" style="padding:0.35rem 0.9rem; font-size:0.8rem;" onclick="submitInlineReply(${commentId})">Reply</button>
                    </div>
                </div>
            `;
            document.getElementById(`reply-input-${commentId}`).focus();
        }

        async function submitInlineReply(parentId) {
            const input = document.getElementById(`reply-input-${parentId}`);
            const content = input.value.trim();
            if (!content) return;

            try {
                const res = await fetch(`/api/posts/${currentPostId}/comments`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ parent_id: parentId, author: 'anonymous', content })
                });

                if (res.ok) {
                    await openThread(currentPostId);
                    await loadFeed();
                    fetchStats();
                }
            } catch (err) {}
        }

        async function submitTopComment() {
            const input = document.getElementById('top-comment-input');
            const content = input.value.trim();
            if (!content) return;

            try {
                const res = await fetch(`/api/posts/${currentPostId}/comments`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ parent_id: null, author: 'patric_kim', content })
                });

                if (res.ok) {
                    input.value = '';
                    await openThread(currentPostId);
                    await loadFeed();
                    fetchStats();
                }
            } catch (err) {}
        }

        async function voteComment(commentId, direction) {
            try {
                const res = await fetch(`/api/comments/${commentId}/vote`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ direction })
                });
                if (res.ok) {
                    const data = await res.json();
                    const el = document.getElementById(`comment-score-${commentId}`);
                    if (el) el.innerText = data.score;
                    fetchStats();
                }
            } catch (err) {}
        }

        function closeThread() {
            document.getElementById('thread-modal').style.display = 'none';
            document.body.style.overflow = 'auto';
            currentPostId = null;
        }

        function handleOverlayClick(e) {
            if (e.target.id === 'thread-modal') closeThread();
        }

        function selectTopic(topicSlug, e) {
            if (e) e.preventDefault();
            currentTopic = topicSlug;

            const label = document.getElementById('active-space-name');
            const commTitle = document.getElementById('current-comm-title');
            const commDesc = document.getElementById('current-comm-desc');

            if (topicSlug === 'all') {
                label.innerText = 'All Spaces';
                commTitle.innerText = 'RustReddit Global';
                commDesc.innerText = 'The lightning-fast discussion network built with Axum 0.8 and Tokio.';
            } else {
                const found = topicsData.find(t => t.slug === topicSlug);
                if (found) {
                    label.innerText = found.name;
                    commTitle.innerText = found.name;
                    commDesc.innerText = found.description;
                }
            }

            fetchTopics();
            loadFeed();
        }

        function changeSort(sort) {
            currentSort = sort;
            document.querySelectorAll('.sort-tab').forEach(b => b.classList.remove('active'));
            document.getElementById(`sort-${sort}`).classList.add('active');
            loadFeed();
        }

        let searchTimer;
        function handleSearch() {
            clearTimeout(searchTimer);
            searchTimer = setTimeout(() => {
                searchQuery = document.getElementById('global-search').value.trim();
                loadFeed();
            }, 250);
        }

        function openCreateModal() {
            document.getElementById('create-modal').style.display = 'flex';
        }

        function closeCreateModal() {
            document.getElementById('create-modal').style.display = 'none';
        }

        function handleCreateOverlayClick(e) {
            if (e.target.id === 'create-modal') closeCreateModal();
        }

        async function handlePostSubmit(e) {
            e.preventDefault();
            const topic_slug = document.getElementById('post-space-select').value;
            const title = document.getElementById('post-title-input').value.trim();
            const author = document.getElementById('post-author-input').value.trim() || 'anonymous';
            const rawTags = document.getElementById('post-tags-input').value.trim();
            const content = document.getElementById('post-content-input').value.trim();

            const tags = rawTags.split(',').map(t => t.trim()).filter(Boolean);

            try {
                const res = await fetch('/api/posts', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ topic_slug, title, author, tags, content })
                });

                if (res.ok) {
                    closeCreateModal();
                    document.getElementById('post-title-input').value = '';
                    document.getElementById('post-author-input').value = '';
                    document.getElementById('post-tags-input').value = '';
                    document.getElementById('post-content-input').value = '';
                    loadFeed();
                    fetchStats();
                }
            } catch (err) {
                alert('Error submitting post');
            }
        }

        function copyPostLink(id, e) {
            e.stopPropagation();
            const url = window.location.origin + `?post=${id}`;
            navigator.clipboard.writeText(url);
            alert('Link copied to clipboard!');
        }

        // Boot
        init();
    </script>
</body>
</html>"#)
}
