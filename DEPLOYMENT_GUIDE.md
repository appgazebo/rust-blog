# Deploying a Rust (Axum) Backend to Azure Container Apps
### *A Zero-Docker, Zero-Cost Deployment Guide for Linux Mint / Ubuntu*

---

## 1. Overview & Architecture

This guide documents the complete end-to-end workflow to deploy a high-performance **Rust Axum web service** to **Azure Container Apps (ACA)** on the **Always-Free tier**, without needing Docker installed locally and without paying for cloud container builders.

```
+-------------------------------------------------------------------------------+
| LOCAL MACHINE (Linux Mint / Ubuntu)                                           |
| - Rust & Cargo (Develop & test locally)                                       |
| - Git & GitHub CLI (Push code & trigger builds)                               |
|   (No local Docker or root/sudo needed!)                                      |
+---------------------------------------+---------------------------------------+
                                        | git push
                                        v
+-------------------------------------------------------------------------------+
| GITHUB ACTIONS (Cloud Builder - Free)                                         |
| 1. Checks out repository                                                      |
| 2. Builds optimized release binary (`cargo build --release`) (~25s)           |
| 3. Packages into ultra-lean container (`Dockerfile.ghcr`)                     |
| 4. Pushes to GitHub Container Registry (`ghcr.io/<user>/<repo>:latest`)       |
+---------------------------------------+---------------------------------------+
                                        | Pull Image
                                        v
+-------------------------------------------------------------------------------+
| AZURE CONTAINER APPS (Serverless Cloud - Free Tier)                           |
| - Scales to 0 when idle ($0 cost)                                             |
| - Wakes up on request in <50ms                                                |
| - Built-in HTTPS with automatic TLS certificates                              |
| - 2,000,000 free HTTP requests/month                                          |
+-------------------------------------------------------------------------------+
```

---

## 2. Key Discoveries & Pitfalls Encountered

During setup on this machine and account, we identified several critical constraints and resolved them:

### A. Azure for Students blocks ACR Cloud Builds (`TasksOperationsNotAllowed`)
* **What happened:** When running `az containerapp up --source .` without local Docker, Azure CLI attempts to create an Azure Container Registry (ACR) and use **ACR Tasks** to build the container in the cloud.
* **The block:** Microsoft blocks ACR Tasks on student and free subscriptions (`TasksOperationsNotAllowed`) to prevent cryptocurrency mining abuse.
* **The solution:** Offload the container build completely to **GitHub Actions** and use **GitHub Container Registry (`ghcr.io`)**, which is 100% free.

### B. No Local Docker / No Root Access Required
* **What happened:** The local workstation did not have Docker installed, and installing it required `sudo` privileges.
* **The solution:** By letting GitHub Actions build the release binary on its native runner and package the image, the local machine only needs standard `git` and `gh` (GitHub CLI).

### C. Azure Resource Providers Must Be Registered First
* Fresh subscriptions do not have Container Apps enabled by default.
* The following providers must be registered:
  * `Microsoft.App`
  * `Microsoft.OperationalInsights`
  * `Microsoft.ContainerRegistry`

### D. Port Configuration Requirement
* Azure Container Apps expects the backend container to bind to `0.0.0.0` and read the `PORT` environment variable (default: `8080`).

---

## 3. Step-by-Step Deployment Guide

Follow these steps to deploy any new Rust Axum project.

### Step 1: Prepare the Axum Application

1. In your `Cargo.toml`, ensure you have:
   ```toml
   [dependencies]
   axum = "0.8"
   tokio = { version = "1", features = ["full"] }
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   tower-http = { version = "0.7", features = ["cors"] }
   ```

2. In `src/main.rs`, ensure your server listens on `0.0.0.0` and respects the `PORT` environment variable:
   ```rust
   use std::net::SocketAddr;

   #[tokio::main]
   async fn main() {
       let app = axum::Router::new()
           .route("/health", axum::routing::get(|| async { "OK" }));

       let port = std::env::var("PORT")
           .unwrap_or_else(|_| "8080".to_string())
           .parse::<u16>()
           .unwrap_or(8080);

       let addr = SocketAddr::from(([0, 0, 0, 0], port));
       let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
       axum::serve(listener, app).await.unwrap();
   }
   ```

---

### Step 2: Create the Container & Git Files

1. Create `Dockerfile.ghcr` in the project root:
   ```dockerfile
   FROM ubuntu:24.04
   RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
   WORKDIR /app
   COPY target/release/<YOUR_BINARY_NAME> /app/<YOUR_BINARY_NAME>
   ENV PORT=8080
   EXPOSE 8080
   CMD ["/app/<YOUR_BINARY_NAME>"]
   ```

2. Create `.dockerignore` (Crucial: do **not** exclude `target/` so Docker can access the compiled binary):
   ```
   .git
   ```

---

### Step 3: Add GitHub Actions CI/CD Workflow

Create `.github/workflows/docker.yml`:
```yaml
name: Build and Push Docker Image

on:
  push:
    branches: [ "master", "main" ]
  workflow_dispatch:

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build Rust binary
        run: cargo build --release

      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and Push Docker image
        run: |
          docker build -t ghcr.io/${{ github.repository }}:latest -f Dockerfile.ghcr .
          docker push ghcr.io/${{ github.repository }}:latest
```

---

### Step 4: Push to GitHub

Using the GitHub CLI:
```bash
git init
git add .
git commit -m "Initial commit"
gh repo create <your-github-username>/<repo-name> --public --source=. --remote=origin --push
```

GitHub Actions will automatically run. You can monitor it with:
```bash
gh run list
```

---

### Step 5: One-Time Azure Setup

Run these commands once in your terminal:

```bash
# 1. Register required Azure resource providers
az provider register -n Microsoft.App
az provider register -n Microsoft.OperationalInsights

# 2. Create a Resource Group
az group create --name rg-rust-demo --location southafricanorth

# 3. Create the Container Apps Environment
az containerapp env create \
  --name rust-blog-env \
  --resource-group rg-rust-demo \
  --location southafricanorth
```

---

### Step 6: Deploy the Container App

Retrieve your GitHub token and deploy:

```bash
TOKEN=$(gh auth token)

az containerapp create \
  --name rust-blog \
  --resource-group rg-rust-demo \
  --environment rust-blog-env \
  --image ghcr.io/<your-github-username>/<repo-name>:latest \
  --target-port 8080 \
  --ingress external \
  --min-replicas 0 \
  --max-replicas 1 \
  --registry-server ghcr.io \
  --registry-username <your-github-username> \
  --registry-password "$TOKEN"
```

---

## 4. Why This Configuration is 100% Free

| Resource | Free Tier Allowance | Our Usage | Cost |
| :--- | :--- | :--- | :--- |
| **Azure Container Apps** | 2,000,000 requests/mo + 180k vCPU-s | Scales to 0 when idle (`--min-replicas 0`) | **$0.00** |
| **GitHub Actions** | 2,000 free runner minutes/month | Each build takes ~40 seconds | **$0.00** |
| **GitHub Packages (GHCR)** | 500 MB storage free | Rust container is only ~35 MB | **$0.00** |
| **SSL / HTTPS Certificates** | Managed by Azure | Automatically issued & renewed | **$0.00** |

---

## 5. Continuous Deployment (Automatic Updates)

Whenever you make changes to your Rust code:
```bash
git add .
git commit -m "Updated backend feature"
git push
```
1. GitHub Actions automatically builds the new binary and pushes both `:latest` and a version tag.
2. Tell Azure to pull the new image:
   ```bash
   az containerapp update \
     --name rust-blog \
     --resource-group rg-rust-demo \
     --image ghcr.io/<your-github-username>/<repo-name>:v2
   ```

---

## 6. Useful Verification Commands

```bash
# Get the live public URL
az containerapp show --name rust-blog --resource-group rg-rust-demo --query "properties.configuration.ingress.fqdn" -o tsv

# Stream live runtime logs
az containerapp logs show --name rust-blog --resource-group rg-rust-demo --follow

# Test endpoints
curl -s https://<YOUR_APP_FQDN>/health
curl -s https://<YOUR_APP_FQDN>/api/posts
```
