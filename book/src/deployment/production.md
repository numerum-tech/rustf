# Production Deployment

This guide covers deploying RustF applications to production environments.

## Prerequisites

- Rust 1.70+ installed
- Production-ready database
- Reverse proxy (nginx, Caddy, etc.)
- SSL certificate (Let's Encrypt recommended)

## Build for Production

### Release Build

```bash
# Build optimized release binary
cargo build --release

# Binary will be in target/release/your-app-name
```

### Build Flags

For maximum optimization:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Configuration

### Deployment Layout

Place all files next to the binary — in release builds the binary looks for `config.toml` **in the same directory as itself**, not in the working directory:

```
/opt/myapp/
├── myapp              ← binary
├── config.toml        ← [app] environment = "production" set here
├── config.prod.toml   ← production overrides, loaded automatically
├── public/            ← static files
└── views/             ← templates (if using filesystem storage)
```

### Production Config

In `config.toml` declare the environment under `[app]`:

```toml
[app]
environment = "production"   # triggers config.prod.toml overlay

[server]
host = "0.0.0.0"
port = 8080
```

Create `config.prod.toml` for production-specific overrides (merged on top of `config.toml`):

```toml
[server]
timeout = 60

[database]
url = "postgresql://user:pass@localhost/dbname"
max_connections = 20

[session]
idle_timeout = 7200
cookie_name = "app_session"

[views]
cache_enabled = true

[logging]
level = "warn"
file = "/var/log/app/error.log"
```

### Environment Variables

Set environment variables:

```bash
export RUSTF_ENV=production
export DATABASE_URL=postgresql://user:pass@localhost/dbname
export SECRET_KEY=your-secret-key-here
```

## Systemd Service

Create `/etc/systemd/system/rustf-app.service`:

```ini
[Unit]
Description=RustF Application
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/myapp
ExecStart=/opt/myapp/myapp
Restart=always
RestartSec=10
Environment="RUST_LOG=warn"

[Install]
WantedBy=multi-user.target
```

> **Note**: `RUSTF_ENV` is not needed here — environment is declared via `[app] environment = "production"` in `config.toml`. The binary always finds `config.toml` next to itself regardless of the working directory.

Enable and start:

```bash
sudo systemctl enable rustf-app
sudo systemctl start rustf-app
sudo systemctl status rustf-app
```

## Nginx Configuration

### Reverse Proxy Setup

Create `/etc/nginx/sites-available/rustf-app`:

```nginx
server {
    listen 80;
    server_name your-domain.com;
    
    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;
    
    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;
    
    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    
    # Proxy to RustF app
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
    
    # Static files
    location /static/ {
        alias /opt/rustf-app/public/;
        expires 30d;
        add_header Cache-Control "public, immutable";
    }
}
```

Enable site:

```bash
sudo ln -s /etc/nginx/sites-available/rustf-app /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

## Database Setup

### Apply schema changes

RustF is **database-first**: the live DB is the source of truth, not an
ordered migration log. The supported flow is:

```bash
# 1. Define / update YAML schemas/<table>.yaml from your design.

# 2. Generate the full SQL DDL from the YAML (writes sql/schema.sql, a complete
#    CREATE TABLE snapshot of the whole schema, not an incremental diff/ALTER).
rustf-cli schema generate sql

# 3. Apply the generated SQL with your tool of choice (psql, mysql, sqlite3,
#    Flyway, Liquibase, etc.). RustF does not ship its own migration runner.

# 4. After the DB is updated, sync the YAML back from the live DB and
#    keep the canonical DDL in source control:
rustf-cli db generate-schema       # writes schemas/*.yaml + schemas/_database_dump.sql
```

For a one-off SQL dump without regenerating the YAML, use
`rustf-cli db dump-schema`.

### Backup Strategy

Set up regular database backups:

```bash
# PostgreSQL
pg_dump -U user dbname > backup_$(date +%Y%m%d).sql

# SQLite
cp db.sqlite backup_$(date +%Y%m%d).sqlite
```

## Monitoring

### Log Management

- Use structured logging
- Set up log rotation
- Monitor error logs
- Use log aggregation (ELK, Loki, etc.)

### Health Checks

Add health check endpoint:

```rust
async fn health(ctx: &mut Context) -> Result<()> {
    // Check database connection
    let db_ok = check_database().is_ok();
    
    ctx.json(json!({
        "status": if db_ok { "healthy" } else { "unhealthy" },
        "database": if db_ok { "connected" } else { "disconnected" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
```

### Metrics

Consider adding:
- Request rate
- Response times
- Error rates
- Database connection pool status

## Security Checklist

- [ ] Use HTTPS (SSL/TLS)
- [ ] Set secure session cookies
- [ ] Enable CSRF protection
- [ ] Set security headers
- [ ] Use environment variables for secrets
- [ ] Keep dependencies updated
- [ ] Use strong database passwords
- [ ] Enable rate limiting
- [ ] Set up firewall rules
- [ ] Regular security audits

## Performance Optimization

### Enable View Caching

```toml
[views]
cache_enabled = true
```

### Database Connection Pooling

```toml
[database]
max_connections = 50
```

### Static File Serving

Serve static files via nginx/CDN, not the application.

### Enable Compression

Configure nginx gzip:

```nginx
gzip on;
gzip_types text/plain text/css application/json application/javascript;
```

## Scaling

### Horizontal Scaling

- Use load balancer (nginx, HAProxy)
- Use shared session storage (Redis)
- Use shared database
- Use CDN for static assets

### Vertical Scaling

- Increase server resources
- Optimize database queries
- Use connection pooling
- Enable caching

## Troubleshooting

### Check Logs

```bash
# Application logs
sudo journalctl -u rustf-app -f

# Nginx logs
sudo tail -f /var/log/nginx/error.log
```

### Common Issues

**Issue: Application won't start**
- Check systemd service status
- Verify configuration file
- Check database connection

**Issue: 502 Bad Gateway**
- Check if app is running
- Verify port configuration
- Check firewall rules

**Issue: Database connection errors**
- Verify database URL
- Check database is running
- Verify credentials

## Next Steps

- Set up CI/CD pipeline
- Configure monitoring alerts
- Set up automated backups
- Plan for disaster recovery











