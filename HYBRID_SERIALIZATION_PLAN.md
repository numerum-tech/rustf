# Hybrid Serialization Strategy: simd-json + rkyv

## Key Finding: simd-json is NOT a Complete Drop-in

**Important:** simd-json is primarily optimized for **parsing** (deserialization), not serialization. It doesn't provide significant speedup for `to_string()` operations.

### What simd-json Supports:
- ✅ Fast JSON **parsing** (`from_str`, `from_slice`) - **2-3x faster**
- ✅ Compatible with serde traits
- ⚠️ Requires `ordered-float` feature for `serde_json::Value` compatibility
- ❌ **No significant speedup for serialization** (`to_string()`)

### What We Need:
- **Parsing:** Use simd-json (2-3x faster)
- **Serialization:** Keep serde_json (simd-json doesn't help)
- **Internal Data:** Use rkyv (6-10x faster, zero-copy)

---

## Recommended Strategy

### 1. **Create a Compatibility Layer**

Create a `json` module that abstracts the choice:

```rust
// rustf/src/utils/json.rs
#[cfg(feature = "simd-json")]
pub use simd_json::*;

#[cfg(not(feature = "simd-json"))]
pub use serde_json::*;

// Re-export Value for compatibility
pub use serde_json::Value;
```

### 2. **Use simd-json for Parsing Only**

Replace `serde_json::from_str()` with `simd_json::from_str()`:

```rust
// Before
let data: SessionData = serde_json::from_str(&json)?;

// After
let data: SessionData = simd_json::from_str(&mut json.into_bytes())?;
// Note: simd_json mutates the input, so we need owned bytes
```

### 3. **Keep serde_json for Serialization**

Continue using `serde_json::to_string()` (simd-json doesn't help here):

```rust
// Keep this
let json = serde_json::to_string(&data)?;
```

### 4. **Use rkyv for Internal Data Structures**

Migrate session storage and cache to rkyv:

```rust
// Sessions: Use rkyv
let bytes = rkyv::to_bytes::<_, 256>(&session_data)?;

// API responses: Keep JSON (use serde_json)
let json = serde_json::to_string(&response)?;
```

---

## Implementation Plan

### Phase 1: Add simd-json for Parsing (Low Risk)

**Files to modify:**
1. `rustf/src/session/redis.rs` - Session deserialization
2. `rustf/src/http/request.rs` - Request body parsing
3. `rustf/src/context.rs` - Form data parsing

**Changes:**
```rust
// Add to Cargo.toml
[dependencies]
simd-json = { version = "0.13", features = ["serde", "alloc", "ordered-float"] }

// Replace parsing calls
// Before:
let data: T = serde_json::from_str(&json)?;

// After:
let mut bytes = json.into_bytes();
let data: T = simd_json::from_slice(&mut bytes)?;
```

**Estimated improvement:** 2-3x faster JSON parsing

---

### Phase 2: Migrate Sessions to rkyv (High Impact)

**Files to modify:**
1. `rustf/src/session/mod.rs` - Add `#[derive(Archive)]` to `SessionData`
2. `rustf/src/session/redis.rs` - Replace JSON with rkyv

**Changes:**
```rust
// Add to Cargo.toml
[dependencies]
rkyv = { version = "0.7", features = ["alloc", "std"] }
rkyv_derive = "0.7"

// Modify SessionData
#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(compare(PartialEq))]
pub struct SessionData {
    // ... existing fields
}

// Replace serialization
// Before:
let json = serde_json::to_string(&data)?;
conn.set(&key, &json).await?;

// After:
let bytes = rkyv::to_bytes::<_, 256>(&data)?;
conn.set(&key, &bytes).await?;

// Replace deserialization
// Before:
let json: String = conn.get(&key).await?;
let data: SessionData = serde_json::from_str(&json)?;

// After:
let bytes: Vec<u8> = conn.get(&key).await?;
let archived = rkyv::check_archived_root::<SessionData>(&bytes[..])?;
let data = archived.deserialize(&mut rkyv::Infallible)?;
```

**Estimated improvement:** 6-10x faster session operations

---

### Phase 3: Create JSON Compatibility Module (Optional)

Create a unified interface for JSON operations:

```rust
// rustf/src/utils/json.rs
use serde::{Deserialize, Serialize};

#[cfg(feature = "simd-json")]
use simd_json;

#[cfg(not(feature = "simd-json"))]
use serde_json;

/// Parse JSON from string (uses simd-json if available)
pub fn from_str<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T> {
    #[cfg(feature = "simd-json")]
    {
        let mut bytes = s.as_bytes().to_vec();
        simd_json::from_slice(&mut bytes)
            .map_err(|e| crate::error::Error::Json(e.into()))
    }
    
    #[cfg(not(feature = "simd-json"))]
    {
        serde_json::from_str(s)
            .map_err(|e| crate::error::Error::Json(e))
    }
}

/// Serialize to JSON string (always uses serde_json)
pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|e| crate::error::Error::Json(e))
}

// Re-export Value for compatibility
pub use serde_json::Value;
```

---

## Detailed Migration Guide

### Step 1: Add Dependencies

```toml
# rustf/Cargo.toml
[dependencies]
# ... existing deps ...

# For fast JSON parsing
simd-json = { version = "0.13", features = ["serde", "alloc", "ordered-float"], optional = true }

# For internal data serialization
rkyv = { version = "0.7", features = ["alloc", "std"], optional = true }
rkyv_derive = "0.7"

[features]
default = ["simd-json", "rkyv"]
simd-json = ["dep:simd-json"]
rkyv = ["dep:rkyv", "dep:rkyv_derive"]
```

### Step 2: Migrate Session Parsing (simd-json)

**File:** `rustf/src/session/redis.rs`

```rust
// Before
let mut session_data: SessionData = serde_json::from_str(&data)?;

// After
#[cfg(feature = "simd-json")]
{
    let mut bytes = data.into_bytes();
    let mut session_data: SessionData = simd_json::from_slice(&mut bytes)?;
}

#[cfg(not(feature = "simd-json"))]
{
    let mut session_data: SessionData = serde_json::from_str(&data)?;
}
```

### Step 3: Migrate Session Storage (rkyv)

**File:** `rustf/src/session/mod.rs`

```rust
// Add Archive derive
#[cfg(feature = "rkyv")]
use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "rkyv", derive(Archive, RkyvSerialize, RkyvDeserialize))]
#[cfg_attr(feature = "rkyv", archive(compare(PartialEq)))]
pub struct SessionData {
    // ... existing fields
}
```

**File:** `rustf/src/session/redis.rs`

```rust
// Serialization
#[cfg(feature = "rkyv")]
{
    let bytes = rkyv::to_bytes::<_, 256>(data)?;
    let _: () = conn.set(&key, &bytes).await?;
}

#[cfg(not(feature = "rkyv"))]
{
    let json = serde_json::to_string(data)?;
    let _: () = conn.set(&key, &json).await?;
}

// Deserialization
#[cfg(feature = "rkyv")]
{
    let bytes: Vec<u8> = conn.get(&key).await?;
    let archived = rkyv::check_archived_root::<SessionData>(&bytes[..])?;
    let session_data = archived.deserialize(&mut rkyv::Infallible)?;
}

#[cfg(not(feature = "rkyv"))]
{
    let data: String = conn.get(&key).await?;
    let session_data: SessionData = serde_json::from_str(&data)?;
}
```

---

## Performance Comparison

### Current (serde_json everywhere):
- Session serialization: ~5-10µs
- Session deserialization: ~5-10µs
- JSON parsing: ~2-3µs per KB
- **Total per request:** ~12-23µs

### With Hybrid Approach:
- Session serialization (rkyv): ~1-2µs
- Session deserialization (rkyv, zero-copy): ~0.5-1µs
- JSON parsing (simd-json): ~0.8-1.2µs per KB
- **Total per request:** ~2.3-4.2µs

### Improvement: **5-10x faster** overall

---

## Compatibility Considerations

### simd-json Requirements:
1. **Valid UTF-8:** simd-json requires valid UTF-8 input
2. **Mutable input:** `from_slice()` mutates the input buffer
3. **Float handling:** Need `ordered-float` feature for `Value` compatibility

### rkyv Requirements:
1. **Type changes:** Must add `#[derive(Archive)]` to types
2. **Binary format:** Not human-readable (but much faster)
3. **Schema evolution:** Breaking changes require migration

---

## Testing Strategy

1. **Unit tests:** Ensure compatibility with existing code
2. **Integration tests:** Test session storage with both formats
3. **Performance benchmarks:** Measure actual improvements
4. **Feature flags:** Allow gradual rollout

---

## Rollout Plan

### Week 1: Add simd-json (Parsing)
- Add dependency with feature flag
- Migrate JSON parsing to simd-json
- Test thoroughly
- **Risk:** Low (parsing only, can fallback)

### Week 2: Add rkyv (Sessions)
- Add dependency with feature flag
- Migrate session storage to rkyv
- Keep JSON as fallback
- **Risk:** Medium (requires type changes)

### Week 3: Benchmark & Optimize
- Run performance benchmarks
- Compare before/after
- Fine-tune based on results

---

## Conclusion

**Yes, we can use simd-json + rkyv, but:**
- ✅ Use **simd-json for parsing** (2-3x faster)
- ✅ Keep **serde_json for serialization** (simd-json doesn't help)
- ✅ Use **rkyv for internal data** (6-10x faster)
- ✅ Use **feature flags** for gradual migration

**This gives us the best of all worlds:**
- Fast JSON parsing (simd-json)
- Fast internal serialization (rkyv)
- JSON compatibility where needed (serde_json)

