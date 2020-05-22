use criterion::{criterion_group, criterion_main, Criterion, Bencher};
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use futures::{AsyncReadExt, AsyncWriteExt};
use loopback::Loopback;

const BUF_SIZE: usize = 8192;
async fn receiver_first_task(mut lb: Loopback, n: usize) {
    let mut buf = [8u8; BUF_SIZE];
    for _ in 0..n {
        lb.read(&mut buf).await.unwrap();
        lb.write(&buf).await.unwrap();
        lb.flush().await.unwrap();
    }
}

async fn sender_first_task(mut lb: Loopback, n: usize) {
    let mut buf = [8u8; BUF_SIZE];
    for _ in 0..n {
        lb.write(&buf).await.unwrap();
        lb.flush().await.unwrap();
        lb.read(&mut buf).await.unwrap();
    }
}

fn echo(n: usize, bencher: &mut Bencher) {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    bencher.iter(|| {
        let (a, b) = Loopback::pair();
        spawner.spawn(sender_first_task(b, n)).unwrap();
        spawner.spawn(receiver_first_task(a, n)).unwrap();
        pool.run();
    });
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("echo 200", |b| echo(200, b));
    c.bench_function("echo 90", |b| echo(90, b));
    c.bench_function("echo 30", |b| echo(30, b));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
