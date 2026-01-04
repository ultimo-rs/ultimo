//! Benchmark tests for WebSocket pub/sub system performance

#![cfg(feature = "websocket")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::sync::mpsc;
use ultimo::websocket::test_helpers::ChannelManager;
use ultimo::websocket::Message;
use uuid::Uuid;

/// Benchmark subscribing clients to topics
fn bench_subscribe(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_subscribe");

    let rt = tokio::runtime::Runtime::new().unwrap();

    for num_topics in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_topics),
            num_topics,
            |b, &num_topics| {
                b.iter(|| {
                    let manager = ChannelManager::new();
                    let (tx, _rx) = mpsc::channel(100);
                    let client_id = Uuid::new_v4();

                    rt.block_on(async {
                        for i in 0..num_topics {
                            let topic = format!("topic-{}", i);
                            manager
                                .subscribe(client_id, &topic, tx.clone())
                                .await
                                .unwrap();
                        }
                        black_box(manager.subscriber_count("topic-0").await)
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark publishing messages to topics
fn bench_publish(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_publish");

    let rt = tokio::runtime::Runtime::new().unwrap();

    for num_subscribers in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_subscribers as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(num_subscribers),
            num_subscribers,
            |b, &num_subscribers| {
                let manager = ChannelManager::new();
                let topic = "benchmark-topic";

                // Set up subscribers
                rt.block_on(async {
                    for _ in 0..num_subscribers {
                        let (tx, _rx) = mpsc::channel(1000);
                        let client_id = Uuid::new_v4();
                        manager.subscribe(client_id, topic, tx).await.unwrap();
                    }
                });

                let message = Message::Text("benchmark message".to_string());

                b.iter(|| {
                    rt.block_on(async {
                        black_box(manager.publish(&topic, message.clone()).await.unwrap())
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark publishing with different message sizes
fn bench_publish_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_publish_sizes");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let num_subscribers = 100;

    for size in [10, 100, 1000, 10000].iter() {
        let message_text = "x".repeat(*size);
        group.throughput(Throughput::Bytes((size * num_subscribers) as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &message_text,
            |b, message_text| {
                let manager = ChannelManager::new();
                let topic = "benchmark-topic";

                // Set up subscribers
                rt.block_on(async {
                    for _ in 0..num_subscribers {
                        let (tx, _rx) = mpsc::channel(1000);
                        let client_id = Uuid::new_v4();
                        manager.subscribe(client_id, topic, tx).await.unwrap();
                    }
                });

                let message = Message::Text(message_text.clone());

                b.iter(|| {
                    rt.block_on(async {
                        black_box(manager.publish(&topic, message.clone()).await.unwrap())
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark unsubscribing from topics
fn bench_unsubscribe(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_unsubscribe");

    let rt = tokio::runtime::Runtime::new().unwrap();

    for num_topics in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_topics),
            num_topics,
            |b, &num_topics| {
                b.iter(|| {
                    let manager = ChannelManager::new();
                    let (tx, _rx) = mpsc::channel(100);
                    let client_id = Uuid::new_v4();

                    rt.block_on(async {
                        // Subscribe to topics
                        for i in 0..num_topics {
                            let topic = format!("topic-{}", i);
                            manager
                                .subscribe(client_id, &topic, tx.clone())
                                .await
                                .unwrap();
                        }

                        // Unsubscribe from topics
                        for i in 0..num_topics {
                            let topic = format!("topic-{}", i);
                            manager.unsubscribe(client_id, &topic).await.unwrap();
                        }

                        black_box(manager.topic_count().await)
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark disconnecting clients (cleanup)
fn bench_disconnect(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_disconnect");

    let rt = tokio::runtime::Runtime::new().unwrap();

    for num_topics in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_topics),
            num_topics,
            |b, &num_topics| {
                b.iter(|| {
                    let manager = ChannelManager::new();
                    let (tx, _rx) = mpsc::channel(100);
                    let client_id = Uuid::new_v4();

                    rt.block_on(async {
                        // Subscribe to many topics
                        for i in 0..num_topics {
                            let topic = format!("topic-{}", i);
                            manager
                                .subscribe(client_id, &topic, tx.clone())
                                .await
                                .unwrap();
                        }

                        // Disconnect (should clean up all subscriptions)
                        manager.disconnect(client_id).await;

                        black_box(manager.connection_count().await)
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark multi-topic publishing
fn bench_publish_multi_topic(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_multi_topic");

    let rt = tokio::runtime::Runtime::new().unwrap();

    for num_topics in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*num_topics as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(num_topics),
            num_topics,
            |b, &num_topics| {
                let manager = ChannelManager::new();

                // Set up subscribers on multiple topics
                rt.block_on(async {
                    for topic_id in 0..num_topics {
                        let (tx, _rx) = mpsc::channel(1000);
                        let client_id = Uuid::new_v4();
                        let topic = format!("topic-{}", topic_id);
                        manager.subscribe(client_id, &topic, tx).await.unwrap();
                    }
                });

                let message = Message::Text("broadcast".to_string());

                b.iter(|| {
                    rt.block_on(async {
                        for topic_id in 0..num_topics {
                            manager
                                .publish(&format!("topic-{}", topic_id), message.clone())
                                .await
                                .unwrap();
                        }
                        black_box(())
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark connection count queries
fn bench_connection_count(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let manager = ChannelManager::new();

    // Set up some connections
    rt.block_on(async {
        for i in 0..100 {
            let (tx, _rx) = mpsc::channel(100);
            let client_id = Uuid::new_v4();
            let topic = format!("topic-{}", i);
            manager.subscribe(client_id, &topic, tx).await.unwrap();
        }
    });

    c.bench_function("connection_count", |b| {
        b.iter(|| rt.block_on(async { black_box(manager.connection_count().await) }));
    });
}

/// Benchmark topic count queries
fn bench_topic_count(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let manager = ChannelManager::new();

    // Set up some topics
    rt.block_on(async {
        let (tx, _rx) = mpsc::channel(100);
        let client_id = Uuid::new_v4();

        for i in 0..100 {
            let topic = format!("topic-{}", i);
            manager
                .subscribe(client_id, &topic, tx.clone())
                .await
                .unwrap();
        }
    });

    c.bench_function("topic_count", |b| {
        b.iter(|| rt.block_on(async { black_box(manager.topic_count().await) }));
    });
}

criterion_group!(
    benches,
    bench_subscribe,
    bench_publish,
    bench_publish_message_sizes,
    bench_unsubscribe,
    bench_disconnect,
    bench_publish_multi_topic,
    bench_connection_count,
    bench_topic_count,
);

criterion_main!(benches);
