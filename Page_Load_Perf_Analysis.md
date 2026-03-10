Here's the full picture ranked by actual impact:

HIGH impact
1. No gzip/brotli compression (entire framework)
The single biggest win. A 70KB HTML page compresses to ~15-20KB — saving 30-50ms on mobile networks. Nothing in the codebase implements Accept-Encoding handling. This alone likely doubles page load time on slow connections.

2. Repository + session cloned on every render (context.rs:276-326)
ctx.view() serializes the entire repository HashMap and clones the entire session Value before rendering. On pages with large model data, this adds measurable latency every request.

3. Double file I/O for static files (app.rs:1195-1267)
Static file serving does metadata() (for ETag) then read() (for content) as two separate async I/O calls. Every cache miss pays two round-trips to disk.

4. Full request body buffered in memory (request.rs:181)
hyper::body::to_bytes() buffers the entire body before the handler runs. No streaming. Large uploads consume RAM linearly.

MEDIUM impact
5. Middleware sorted per request (middleware/mod.rs:116)
get_sorted() allocates a Vec and sorts it on every single request. Middleware order is fixed at startup — sort once, store the result.

6. Template struct cloned from cache (engine.rs:77-119)
Templates are cached as Arc<Template> but then the inner struct is cloned out of the Arc on every render. This defeats the Arc entirely — the render should work on a reference.

7. HTTP date parsed per-request (app.rs:1280-1312)
parse_http_date() tries three sequential chrono format parsers on every If-Modified-Since header. Should be a single fast parse.

8. No ETag on rendered pages
View responses set no ETag or Cache-Control headers. Even a short-lived max-age=60 or a content hash ETag would eliminate redundant renders for identical pages.

LOW-MEDIUM impact
9. Route params HashMap allocated per match (trie.rs:140)
Even parameterless routes allocate an empty HashMap per match.

10. Response headers: Vec<(String, String)> (response.rs:9)
Every header is two string allocations. A typical response with 10 headers = 20 heap allocations. SmallVec or a fixed-size stack array would eliminate most of this.

11. Context allocates HashMaps eagerly (context.rs:65)
repository and data HashMaps are allocated on every request even for pure JSON API endpoints that never use them.

Priority order for fixes
Priority	Fix	Effort
1	gzip compression middleware	Medium
2	Cache middleware sort order at startup	Low
3	Stop cloning template from Arc	Low
4	Merge static file metadata+read into one I/O	Low
5	Repository/session pass by ref to renderer	Medium
6	ETag + Cache-Control on rendered views	Low
Want to tackle any of these? The middleware sort fix and the template Arc clone fix are 5-minute changes with immediate measurable effect.