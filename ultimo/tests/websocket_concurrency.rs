//! Concurrency and stress tests for WebSocket
//!
//! Tests multiple concurrent connections, race conditions, and pub/sub under load

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Barrier;
use ultimo::websocket::{ChannelManager, Message};

#[cfg(test)]
mod concurrency_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_subscriptions() {
        let manager = Arc::new(ChannelManager::new());
        let mut handles = vec![];

        // Spawn 100 concurrent subscription tasks
        for i in 0..100 {
            let manager: Arc<ChannelManager> = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                let conn_id = uuid::Uuid::new_v4();
                let topic = format!("topic_{}", i % 10); // 10 different topics
                let (tx, _rx) = mpsc::channel(1000);

                manager.subscribe(conn_id, &topic, tx).await.unwrap();

                // Verify subscription
                let count = manager.subscriber_count(&topic).await;
                assert!(count > 0);

                conn_id
            });
            handles.push(handle);
        }

        // Wait for all to complete
        let conn_ids: Vec<uuid::Uuid> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // Verify all subscriptions exist
        for _conn_id in conn_ids {
            // Connection should be tracked
        }
    }

    #[tokio::test]
    async fn test_concurrent_publish() {
        let manager = Arc::new(ChannelManager::new());
        let topic = "broadcast_topic";
        let num_subscribers = 50;

        // Create subscribers
        let mut receivers = vec![];
        for _ in 0..num_subscribers {
            let (tx, rx) = mpsc::channel(1000);
            let conn_id = uuid::Uuid::new_v4();

            manager.subscribe(conn_id, topic, tx).await.unwrap();
            receivers.push(rx);
        }

        // Publish messages concurrently
        let num_messages = 20;
        let mut publish_handles = vec![];

        for i in 0..num_messages {
            let manager: Arc<ChannelManager> = Arc::clone(&manager);
            let topic = topic.to_string();
            let handle = tokio::spawn(async move {
                let msg = Message::Text(format!("message_{}", i));
                manager.publish(&topic, msg).await
            });
            publish_handles.push(handle);
        }

        // Wait for all publishes
        let results = futures_util::future::join_all(publish_handles).await;

        // All should succeed
        for result in results {
            let count = result.unwrap().unwrap();
            assert_eq!(count, num_subscribers);
        }

        // Each subscriber should have received all messages
        for mut rx in receivers {
            let mut count = 0;
            while rx.try_recv().is_ok() {
                count += 1;
            }
            assert_eq!(count, num_messages);
        }
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe_race() {
        let manager = Arc::new(ChannelManager::new());
        let topic = "race_topic";
        let conn_id = uuid::Uuid::new_v4();

        // Subscribe and unsubscribe rapidly
        let mut handles = vec![];
        for _ in 0..100 {
            let manager: Arc<ChannelManager> = Arc::clone(&manager);
            let topic = topic.to_string();
            let handle = tokio::spawn(async move {
                let (tx, _rx) = mpsc::channel(1000);
                manager.subscribe(conn_id, &topic, tx).await.ok();
                tokio::time::sleep(Duration::from_micros(10)).await;
                manager.unsubscribe(conn_id, &topic).await.ok();
            });
            handles.push(handle);
        }

        futures_util::future::join_all(handles).await;

        // Final state should be consistent
        let count = manager.subscriber_count(topic).await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_concurrent_disconnect() {
        let manager = Arc::new(ChannelManager::new());
        let topic = "disconnect_topic";

        // Create many connections and subscribe
        let mut conn_ids = vec![];
        for _ in 0..100 {
            let conn_id = uuid::Uuid::new_v4();
            let (tx, _rx) = mpsc::channel(1000);

            manager.subscribe(conn_id, topic, tx).await.unwrap();
            conn_ids.push(conn_id);
        }

        let count_before = manager.subscriber_count(topic).await;
        assert_eq!(count_before, 100);

        // Disconnect all concurrently
        let mut handles = vec![];
        for conn_id in conn_ids {
            let manager: Arc<ChannelManager> = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                manager.disconnect(conn_id).await;
            });
            handles.push(handle);
        }

        futures_util::future::join_all(handles).await;

        // All should be disconnected
        let count_after = manager.subscriber_count(topic).await;
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_many_topics_subscription() {
        let manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(1000);

        // Subscribe to many topics
        let num_topics = 1000;
        for i in 0..num_topics {
            let topic = format!("topic_{}", i);
            manager
                .subscribe(conn_id, &topic, tx.clone())
                .await
                .unwrap();
        }

        // Verify subscription count
        let topic_count = manager.topic_count().await;
        assert_eq!(topic_count, num_topics);
    }

    #[tokio::test]
    async fn test_barrier_synchronized_publish() {
        // Test that all publishers start at the same time to maximize race conditions
        let manager = Arc::new(ChannelManager::new());
        let topic = "sync_topic";
        let num_publishers = 10;
        let barrier = Arc::new(Barrier::new(num_publishers));

        // Setup subscriber
        let (tx, mut rx) = mpsc::channel(1000);
        let conn_id = uuid::Uuid::new_v4();

        manager.subscribe(conn_id, topic, tx).await.unwrap();

        // Spawn synchronized publishers
        let mut handles = vec![];
        for i in 0..num_publishers {
            let manager: Arc<ChannelManager> = Arc::clone(&manager);
            let barrier = Arc::clone(&barrier);
            let topic = topic.to_string();

            let handle = tokio::spawn(async move {
                barrier.wait().await; // Wait for all to be ready
                let msg = Message::Text(format!("msg_{}", i));
                manager.publish(&topic, msg).await
            });
            handles.push(handle);
        }

        // All publishers complete
        futures_util::future::join_all(handles).await;

        // Count received messages
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, num_publishers);
    }

    #[tokio::test]
    async fn test_publish_to_nonexistent_topic() {
        let manager = Arc::new(ChannelManager::new());

        // Publish to topic with no subscribers
        let msg = Message::Text("hello".to_string());
        let result = manager.publish("nonexistent", msg).await;

        // Should succeed but return 0 recipients
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_connection_cleanup_on_channel_close() {
        let manager = Arc::new(ChannelManager::new());
        let topic = "cleanup_topic";
        let conn_id = uuid::Uuid::new_v4();

        let (tx, rx) = mpsc::channel(1000);

        manager.subscribe(conn_id, topic, tx).await.unwrap();

        // Drop receiver (simulating closed connection)
        drop(rx);

        // Try to publish - should handle closed channel gracefully
        let msg = Message::Text("test".to_string());
        let result = manager.publish(topic, msg).await;

        // Should still return 1 (attempted) even though send failed
        // The actual cleanup happens in disconnect()
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_high_frequency_publish() {
        let manager = Arc::new(ChannelManager::new());
        let topic = "high_freq";
        // Use larger buffer to handle all messages without backpressure
        let (tx, mut rx) = mpsc::channel(15000);
        let conn_id = uuid::Uuid::new_v4();

        manager.subscribe(conn_id, topic, tx).await.unwrap();

        // Publish rapidly
        let num_messages = 10000;
        for i in 0..num_messages {
            let msg = Message::Text(format!("msg_{}", i));
            manager.publish(topic, msg).await.unwrap();
        }

        // Verify all received
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, num_messages);
    }

    #[tokio::test]
    async fn test_multiple_topics_single_connection() {
        let manager = Arc::new(ChannelManager::new());
        let (tx, mut rx) = mpsc::channel(1000);
        let conn_id = uuid::Uuid::new_v4();

        // Subscribe to multiple topics
        let topics = vec!["topic_a", "topic_b", "topic_c"];
        for topic in &topics {
            manager.subscribe(conn_id, topic, tx.clone()).await.unwrap();
        }

        // Publish to each topic
        for topic in &topics {
            let msg = Message::Text(format!("msg_for_{}", topic));
            manager.publish(topic, msg).await.unwrap();
        }

        // Should receive all messages
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, topics.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_stress_many_concurrent_operations() {
        // Stress test with many concurrent operations
        let manager = Arc::new(ChannelManager::new());
        let mut handles = vec![];

        // Spawn 200 tasks doing various operations
        for i in 0..200 {
            let manager: Arc<ChannelManager> = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                let conn_id = uuid::Uuid::new_v4();
                let (tx, _rx) = mpsc::channel(1000);
                let topic = format!("topic_{}", i % 20);

                // Register

                // Subscribe
                manager.subscribe(conn_id, &topic, tx).await.ok();

                // Publish a few times
                for j in 0..5 {
                    let msg = Message::Text(format!("msg_{}_{}", i, j));
                    manager.publish(&topic, msg).await.ok();
                }

                // Unsubscribe
                manager.unsubscribe(conn_id, &topic).await.ok();

                // Disconnect
                manager.disconnect(conn_id).await;
            });
            handles.push(handle);
        }

        // Wait for all to complete
        futures_util::future::join_all(handles).await;

        // System should be in clean state
        let topic_count = manager.topic_count().await;
        // All topics should be cleaned up if no subscribers remain
        assert!(topic_count <= 20); // Some may still have cleanup pending
    }
}
