use criterion::{criterion_group, criterion_main, Criterion};
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use futures::{AsyncReadExt, AsyncWriteExt};
use loopback::Loopback;

const BUF_SIZE: usize = 5120;
async fn receiver_first_task(mut lb: Loopback, n: usize) {
    let mut buf = [8u8; BUF_SIZE];
    for _ in 0..n {
        let _ = lb.read(&mut buf).await;
        let _ = lb.write(&buf).await;
    }
}

async fn sender_first_task(mut lb: Loopback, n: usize) {
    let mut buf = [8u8; BUF_SIZE];
    for _ in 0..n {
        let _ = lb.write(&buf).await;
        let _ = lb.read(&mut buf).await;
    }
}

fn echo(n: usize) {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let (a, b) = Loopback::pair();
    spawner.spawn(sender_first_task(b, n)).unwrap();
    spawner.spawn(receiver_first_task(a, n)).unwrap();
    pool.run();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("echo 200", |b| b.iter(|| echo(200)));
    c.bench_function("echo 90", |b| b.iter(|| echo(90)));
    c.bench_function("echo 30", |b| b.iter(|| echo(30)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
