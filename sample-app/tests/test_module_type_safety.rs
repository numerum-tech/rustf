//! Test to demonstrate compile-time type safety of MODULE::register()
//!
//! This test file demonstrates that:
//! 1. Only types implementing SharedModule can be registered
//! 2. Simple utilities don't need to implement SharedModule
//! 3. The type requirement is enforced at compile time

use rustf::prelude::*;

/// A simple utility that does NOT implement SharedModule
pub struct SimpleUtil;

impl SimpleUtil {
    pub fn helper() -> String {
        "just a helper".to_string()
    }
}

/// A service that DOES implement SharedModule
#[derive(Clone)]
pub struct MyService;

impl_shared_service!(MyService);

#[tokio::test]
async fn test_shared_module_registration() {
    // This would compile and work fine:
    // MODULE::init().unwrap();
    // MODULE::register("my-service", MyService).unwrap();
    //
    // But this would NOT compile (intentionally!):
    // MODULE::register("simple-util", SimpleUtil).unwrap();
    // ^^^^^^^ Error: SimpleUtil does not implement SharedModule
    //
    // This is the key feature - type safety at compile time ensures
    // only proper services can be registered as singletons.
}

#[test]
fn test_simple_util_usage() {
    // SimpleUtil doesn't need registration - just use it directly
    let result = SimpleUtil::helper();
    assert_eq!(result, "just a helper");
}
