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

### Production Config

Create `config.prod.toml`:

```toml
environment = "production"

[server]
host = "0.0.0.0"
port = 8080
timeout = 30

[database]
url = "postgresql://user:pass@localhost/dbname"
pool_size = 20

[session]
timeout = 7200
secure = true
http_only = true
cookie_name = "app_session"

[views]
cache_enabled = true
directory = "views"

[logging]
level = "warn"
output = "file"
file_path = "/var/log/app/error.log"
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
WorkingDirectory=/opt/rustf-app
ExecStart=/opt/rustf-app/target/release/rustf-app
Restart=always
RestartSec=10
Environment="RUSTF_ENV=production"
Environment="RUST_LOG=warn"

[Install]
WantedBy=multi-user.target
```

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

### Run Migrations

```bash
# In production
rustf-cli migrate up
```

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
pool_size = 20
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


