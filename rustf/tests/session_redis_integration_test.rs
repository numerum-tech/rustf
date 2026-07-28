#[cfg(feature = "redis")]
mod redis_integration {
    use rustf::http::Request;
    use rustf::middleware::builtin::SessionMiddleware;
    use rustf::middleware::traits::{InboundMiddleware, OutboundMiddleware};
    use rustf::prelude::*;
    use rustf::session::manager::SessionConfig;
    use rustf::session::redis::RedisSessionStorage;
    use rustf::session::{FingerprintMode, SessionStorage};
    use rustf::views::ViewEngine;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn create_test_context(path: &str, cookie: Option<&str>) -> Context {
        let view_engine = Arc::new(ViewEngine::new());
        let mut request = Request::new("GET", path, "HTTP/1.1");
        request.headers.insert(
            "user-agent".to_string(),
            "redis-session-integration-test".to_string(),
        );
        request
            .headers
            .insert("x-forwarded-for".to_string(), "127.0.0.1".to_string());

        if let Some(cookie) = cookie {
            request
                .headers
                .insert("cookie".to_string(), cookie.to_string());
        }

        Context::new(request, view_engine)
    }

    fn extract_cookie_pair(set_cookie: &str) -> &str {
        set_cookie
            .split(';')
            .next()
            .expect("Set-Cookie should contain a cookie pair")
    }

    fn session_cookie(response: &rustf::http::Response) -> String {
        response
            .headers
            .iter()
            .find(|(name, value)| name == "Set-Cookie" && !value.contains("Max-Age=0"))
            .map(|(_, value)| value.clone())
            .expect("expected session Set-Cookie header")
    }

    #[tokio::test]
    async fn redis_session_middleware_round_trip_and_destroy() {
        let unique_prefix = format!(
            "rustf:test:http-session:{}:",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // `from_url` issues a timeout-bounded PING, so this doubles as the
        // availability check: with no server reachable the test skips rather
        // than failing on a machine or CI runner without Redis. Set
        // `RUSTF_TEST_REDIS=1` to require it instead.
        let storage = match RedisSessionStorage::from_url(
            "redis://localhost:6379",
            &unique_prefix,
            4,
            FingerprintMode::Soft,
            Duration::from_secs(60),
            Duration::from_secs(5),
            Duration::from_secs(3),
        )
        .await
        {
            Ok(storage) => Arc::new(storage),
            Err(err) => {
                if std::env::var_os("RUSTF_TEST_REDIS").is_some() {
                    panic!("RUSTF_TEST_REDIS is set but Redis is unreachable: {err}");
                }
                eprintln!(
                    "skipping Redis session integration test — no server on 127.0.0.1:6379 ({err})"
                );
                return;
            }
        };

        let mut config = SessionConfig::new();
        config.cookie_name = "redis_test_sid".to_string();
        config.secure = false;
        config.idle_timeout = Duration::from_secs(60);
        config.absolute_timeout = Duration::from_secs(300);

        let middleware = SessionMiddleware::with_storage(storage.clone(), config);

        let mut ctx1 = create_test_context("/redis-session", None);
        let action = middleware.process_request(&mut ctx1).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));

        let session1 = ctx1.session_arc().expect("session should be created");
        session1.set("user_id", 123_i32).unwrap();
        session1.flash_set("notice", "hello from redis").unwrap();
        let first_session_id = session1.id().to_string();

        middleware.process_response(&mut ctx1).await.unwrap();

        let first_cookie = session_cookie(ctx1.res.as_ref().unwrap());
        let first_cookie_pair = extract_cookie_pair(&first_cookie).to_string();
        assert!(first_cookie.starts_with("redis_test_sid="));
        assert!(first_cookie.contains("Max-Age=60"));

        let mut ctx2 = create_test_context("/redis-session", Some(&first_cookie_pair));
        let action = middleware.process_request(&mut ctx2).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));

        let session2 = ctx2.session_arc().expect("session should be loaded");
        assert_eq!(session2.id(), first_session_id);
        assert_eq!(session2.get::<i32>("user_id"), Some(123));
        assert_eq!(
            session2.flash_get::<String>("notice"),
            Some("hello from redis".to_string())
        );

        middleware.process_response(&mut ctx2).await.unwrap();

        let second_cookie = session_cookie(ctx2.res.as_ref().unwrap());
        let second_cookie_pair = extract_cookie_pair(&second_cookie).to_string();

        let mut ctx3 = create_test_context("/redis-session", Some(&second_cookie_pair));
        let action = middleware.process_request(&mut ctx3).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));

        let session3 = ctx3
            .session_arc()
            .expect("session should still exist after flash consumption");
        assert_eq!(session3.id(), first_session_id);
        assert_eq!(session3.get::<i32>("user_id"), Some(123));
        assert_eq!(session3.flash_get::<String>("notice"), None);

        ctx3.session_destroy();
        middleware.process_response(&mut ctx3).await.unwrap();

        let destroy_cookie = ctx3
            .res
            .as_ref()
            .unwrap()
            .headers
            .iter()
            .find(|(name, value)| name == "Set-Cookie" && value.contains("Max-Age=0"))
            .map(|(_, value)| value.clone())
            .expect("expected destroy Set-Cookie header");
        assert!(destroy_cookie.starts_with("redis_test_sid="));

        let mut ctx4 = create_test_context("/redis-session", Some(&second_cookie_pair));
        let action = middleware.process_request(&mut ctx4).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));

        let session4 = ctx4
            .session_arc()
            .expect("a new session should be created after destroy");
        assert_ne!(session4.id(), first_session_id);
        assert_eq!(session4.get::<i32>("user_id"), None);
        assert_eq!(session4.flash_get::<String>("notice"), None);

        storage.delete(session4.id()).await.unwrap();
    }
}
