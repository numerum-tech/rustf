# Docker Deployment

This guide covers containerizing and deploying RustF applications with Docker.

## Dockerfile

### Multi-Stage Build

Create `Dockerfile`:

```dockerfile
# Build stage
FROM rust:1.70 as builder

WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY rustf/Cargo.toml ./rustf/
COPY rustf-macros/Cargo.toml ./rustf-macros/

# Copy source code
COPY . .

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/rustf-app /app/rustf-app

# Copy configuration and static files
COPY config.toml ./
COPY views ./views
COPY public ./public

# Create non-root user
RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

EXPOSE 8000

CMD ["./rustf-app"]
```

## Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "8000:8000"
    environment:
      - RUSTF_ENV=production
      - DATABASE_URL=postgresql://user:pass@db:5432/dbname
    depends_on:
      - db
      - redis
    volumes:
      - ./config.toml:/app/config.toml
      - ./views:/app/views
      - ./public:/app/public
    restart: unless-stopped

  db:
    image: postgres:15
    environment:
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
      - POSTGRES_DB=dbname
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
```

## Build and Run

### Build Image

```bash
docker build -t rustf-app .
```

### Run Container

```bash
docker run -d \
  -p 8000:8000 \
  -e RUSTF_ENV=production \
  -v $(pwd)/config.toml:/app/config.toml \
  --name rustf-app \
  rustf-app
```

### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## Production Dockerfile

Optimized for production:

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy and build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 appuser

WORKDIR /app

COPY --from=builder /app/target/release/rustf-app /app/
COPY --from=builder /app/config.toml ./
COPY --from=builder /app/views ./views
COPY --from=builder /app/public ./public

RUN chown -R appuser:appuser /app

USER appuser

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8000/health || exit 1

CMD ["./rustf-app"]
```

## .dockerignore

Create `.dockerignore`:

```
target/
.git/
.gitignore
*.md
.env
.env.local
*.log
```

## Kubernetes

### Deployment

Create `k8s/deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustf-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rustf-app
  template:
    metadata:
      labels:
        app: rustf-app
    spec:
      containers:
      - name: rustf-app
        image: rustf-app:latest
        ports:
        - containerPort: 8000
        env:
        - name: RUSTF_ENV
          value: "production"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: url
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### Service

Create `k8s/service.yaml`:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: rustf-app
spec:
  selector:
    app: rustf-app
  ports:
  - port: 80
    targetPort: 8000
  type: LoadBalancer
```

## Best Practices

1. **Use multi-stage builds** to reduce image size
2. **Run as non-root user** for security
3. **Use .dockerignore** to exclude unnecessary files
4. **Set resource limits** in production
5. **Use health checks** for monitoring
6. **Keep images updated** with security patches
7. **Use secrets management** for sensitive data
8. **Enable logging** for debugging

## Troubleshooting

### Container won't start

```bash
# Check logs
docker logs rustf-app

# Run interactively
docker run -it rustf-app /bin/bash
```

### Database connection issues

- Verify database URL
- Check network connectivity
- Verify credentials

### Performance issues

- Increase resource limits
- Enable connection pooling
- Use caching layer


