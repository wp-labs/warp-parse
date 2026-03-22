# rust-async-patterns

Async Rust 最佳实践：任务管理、错误处理、常见陷阱规避。

## 适用场景

- 设计异步模块或服务
- 排查异步代码中的编译错误或运行时问题
- 审查异步代码是否遵循最佳实践

## 核心概念

### Future 是惰性的

```rust
// 这不会执行！只是创建了一个 Future
async_function();

// 必须 .await 或 spawn
async_function().await;
tokio::spawn(async_function());
```

### 选择正确的运行时

项目使用 `tokio`（Rust 社区标准）时：

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

**按需启用 features，不要无脑 `features = ["full"]`：**

| Feature | 用途 |
|---------|------|
| `rt` | 单线程运行时 |
| `rt-multi-thread` | 多线程运行时 |
| `macros` | `#[tokio::main]` 和 `#[tokio::test]` |
| `time` | `sleep`, `timeout`, `interval` |
| `io-util` | `AsyncReadExt`, `AsyncWriteExt` |
| `net` | TCP/UDP/Unix socket |
| `fs` | 异步文件操作 |
| `sync` | 异步同步原语 |
| `signal` | 信号处理 |

## 最佳实践

### 1. 结构化并发

```rust
use tokio::task::JoinSet;

// Good: 使用 JoinSet 管理一组任务
async fn process_items(items: Vec<Item>) -> Vec<Result<Output>> {
    let mut set = JoinSet::new();

    for item in items {
        set.spawn(async move {
            process_item(item).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        results.push(result.unwrap()); // JoinError 表示 panic
    }
    results
}

// Good: 有限并发
use tokio::sync::Semaphore;

async fn process_with_limit(items: Vec<Item>, max_concurrent: usize) {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut set = JoinSet::new();

    for item in items {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        set.spawn(async move {
            let result = process_item(item).await;
            drop(permit);
            result
        });
    }

    while let Some(_) = set.join_next().await {}
}
```

### 2. 取消安全（Cancellation Safety）

```rust
// Bad: select! 中使用非取消安全的操作
loop {
    tokio::select! {
        // read_exact 不是取消安全的，部分读取的数据会丢失
        data = reader.read_exact(&mut buf) => { ... }
        _ = cancel_token.cancelled() => break,
    }
}

// Good: 使用取消安全的替代方案
loop {
    tokio::select! {
        // read 是取消安全的
        n = reader.read(&mut buf) => { ... }
        _ = cancel_token.cancelled() => break,
    }
}
```

**取消安全的操作：**
- `tokio::sync::mpsc::Receiver::recv()`
- `tokio::io::AsyncRead::read()`
- `tokio::time::sleep()`

**不取消安全的操作：**
- `tokio::io::AsyncRead::read_exact()`
- `tokio::io::BufReader::read_line()`

### 3. 避免在 async 中持有锁跨 await

```rust
// Bad: MutexGuard 跨 await 点
async fn bad_example(data: Arc<Mutex<Vec<i32>>>) {
    let mut guard = data.lock().unwrap();
    some_async_operation().await;  // guard 在这里仍然被持有！
    guard.push(42);
}

// Good: 在 await 前释放锁
async fn good_example(data: Arc<Mutex<Vec<i32>>>) {
    {
        let mut guard = data.lock().unwrap();
        guard.push(42);
    } // guard 在这里释放
    some_async_operation().await;
}

// 或者使用 tokio::sync::Mutex（async-aware）
async fn also_good(data: Arc<tokio::sync::Mutex<Vec<i32>>>) {
    let mut guard = data.lock().await;
    some_async_operation().await;
    guard.push(42);
}
```

### 4. Send + Sync 约束

```rust
// tokio::spawn 要求 Future 是 Send
// 这意味着 Future 持有的所有值都必须是 Send

// Bad: Rc 不是 Send
async fn bad() {
    let data = Rc::new(42);
    some_async_call().await;
    println!("{}", data);  // data 跨越 await，但 Rc 不是 Send
}

// Good: 使用 Arc
async fn good() {
    let data = Arc::new(42);
    some_async_call().await;
    println!("{}", data);
}
```

### 5. 异步测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_function() {
        let result = my_async_function().await;
        assert_eq!(result, expected);
    }

    // 带超时的测试（防止挂起）
    #[tokio::test]
    async fn test_with_timeout() {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            my_async_function()
        ).await;
        assert!(result.is_ok(), "test timed out");
    }
}
```

### 6. 优雅关闭（Graceful Shutdown）

```rust
use tokio_util::sync::CancellationToken;

async fn run_service(cancel: CancellationToken) {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                // 清理资源
                break;
            }
            request = accept_request() => {
                handle_request(request).await;
            }
        }
    }
}
```

## 常见编译错误

| 错误 | 原因 | 修复 |
|------|------|------|
| `future is not Send` | Future 持有非 Send 类型跨 await | 改用 Send 类型或在 await 前 drop |
| `cannot borrow as mutable` | async 中借用冲突 | 缩小借用范围或使用 async Mutex |
| `lifetime may not live long enough` | 'static bound on spawn | 使用 `move` 闭包，确保 owned data |
| `async fn in trait` | trait 中使用 async fn | 使用 `async-trait` crate 或 RPITIT（Rust 1.75+） |

## 性能建议

- **避免不必要的 async** — 纯计算函数不要标记为 async
- **使用 `spawn_blocking`** — CPU 密集型任务不要在 async 上下文中执行
- **合理设置 buffer 大小** — channel 和 IO buffer 根据负载调整
- **避免 async 递归** — 使用循环 + 栈代替，或 `Box::pin`
- **批量操作** — 合并多个小 await 为一个批量操作

```rust
// CPU 密集型任务使用 spawn_blocking
let result = tokio::task::spawn_blocking(move || {
    expensive_computation(&data)
}).await?;
```

## 审查清单

- [ ] 所有 Future 都被 `.await` 或 `spawn`（没有被忽略的 Future）
- [ ] `select!` 中的操作是取消安全的
- [ ] `std::sync::Mutex` 不跨 await 点持有
- [ ] `spawn` 的 Future 满足 `Send + 'static`
- [ ] CPU 密集型操作使用 `spawn_blocking`
- [ ] 有优雅关闭机制
- [ ] 异步测试有超时保护
