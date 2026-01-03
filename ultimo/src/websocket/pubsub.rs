//! Pub/Sub channel manager for WebSocket broadcasting

use super::frame::Message;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Manages WebSocket pub/sub channels and message broadcasting
pub struct ChannelManager {
    /// Maps topic -> set of connection IDs
    subscriptions: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,
    /// Maps connection ID -> sender
    connections: Arc<RwLock<HashMap<Uuid, mpsc::Sender<Message>>>>,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe a connection to a topic
    pub async fn subscribe(
        &self,
        connection_id: Uuid,
        topic: &str,
        sender: mpsc::Sender<Message>,
    ) -> Result<(), std::io::Error> {
        // Register connection if not already registered
        {
            let mut connections = self.connections.write().await;
            connections.entry(connection_id).or_insert(sender);
        }

        // Add to topic subscriptions
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions
                .entry(topic.to_string())
                .or_insert_with(HashSet::new)
                .insert(connection_id);
        }

        tracing::debug!(
            "Connection {} subscribed to topic: {}",
            connection_id,
            topic
        );
        Ok(())
    }

    /// Unsubscribe a connection from a topic
    pub async fn unsubscribe(
        &self,
        connection_id: Uuid,
        topic: &str,
    ) -> Result<(), std::io::Error> {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(subscribers) = subscriptions.get_mut(topic) {
            subscribers.remove(&connection_id);

            // Clean up empty topics
            if subscribers.is_empty() {
                subscriptions.remove(topic);
            }
        }

        tracing::debug!(
            "Connection {} unsubscribed from topic: {}",
            connection_id,
            topic
        );
        Ok(())
    }

    /// Publish a message to all subscribers of a topic
    pub async fn publish(&self, topic: &str, message: Message) -> Result<usize, std::io::Error> {
        let subscriptions = self.subscriptions.read().await;
        let connections = self.connections.read().await;

        let subscribers = match subscriptions.get(topic) {
            Some(subs) => subs,
            None => return Ok(0),
        };

        let mut sent_count = 0;
        let mut failed_connections = Vec::new();

        for connection_id in subscribers {
            if let Some(sender) = connections.get(connection_id) {
                // Use try_send to avoid blocking on backpressure
                match sender.try_send(message.clone()) {
                    Ok(_) => sent_count += 1,
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Connection is backpressured, skip but don't disconnect
                        tracing::warn!("Connection {} backpressured, skipping message", connection_id);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Connection closed, mark for removal
                        failed_connections.push(*connection_id);
                    }
                }
            }
        }

        // Clean up failed connections
        if !failed_connections.is_empty() {
            drop(subscriptions);
            drop(connections);
            for conn_id in failed_connections {
                self.disconnect(conn_id).await;
            }
        }

        tracing::debug!("Published to topic '{}': {} subscribers", topic, sent_count);
        Ok(sent_count)
    }

    /// Disconnect a connection and clean up all its subscriptions
    pub async fn disconnect(&self, connection_id: Uuid) {
        // Remove from all topics
        {
            let mut subscriptions = self.subscriptions.write().await;
            let topics_to_clean: Vec<String> = subscriptions
                .iter()
                .filter_map(|(topic, subscribers)| {
                    if subscribers.contains(&connection_id) {
                        Some(topic.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for topic in topics_to_clean {
                if let Some(subscribers) = subscriptions.get_mut(&topic) {
                    subscribers.remove(&connection_id);
                    if subscribers.is_empty() {
                        subscriptions.remove(&topic);
                    }
                }
            }
        }

        // Remove connection
        {
            let mut connections = self.connections.write().await;
            connections.remove(&connection_id);
        }

        tracing::debug!("Connection {} disconnected and cleaned up", connection_id);
    }

    /// Get number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get number of active topics
    pub async fn topic_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Get subscriber count for a topic
    pub async fn subscriber_count(&self, topic: &str) -> usize {
        self.subscriptions
            .read()
            .await
            .get(topic)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    /// Broadcast a message to all connected clients (for graceful shutdown)
    pub async fn broadcast_all(&self, message: Message) -> usize {
        let connections = self.connections.read().await;
        let mut count = 0;

        for sender in connections.values() {
            // Use try_send to avoid blocking
            if sender.try_send(message.clone()).is_ok() {
                count += 1;
            }
        }

        count
    }

    /// Get all connection IDs
    pub async fn all_connection_ids(&self) -> Vec<Uuid> {
        self.connections.read().await.keys().copied().collect()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let manager = ChannelManager::new();
        let (tx, _rx) = mpsc::channel(100);
        let conn_id = Uuid::new_v4();

        manager
            .subscribe(conn_id, "test-topic", tx.clone())
            .await
            .unwrap();
        assert_eq!(manager.subscriber_count("test-topic").await, 1);

        manager.unsubscribe(conn_id, "test-topic").await.unwrap();
        assert_eq!(manager.subscriber_count("test-topic").await, 0);
    }

    #[tokio::test]
    async fn test_publish() {
        let manager = ChannelManager::new();
        let (tx1, mut rx1) = mpsc::channel(100);
        let (tx2, mut rx2) = mpsc::channel(100);

        let conn1 = Uuid::new_v4();
        let conn2 = Uuid::new_v4();

        manager.subscribe(conn1, "test-topic", tx1).await.unwrap();
        manager.subscribe(conn2, "test-topic", tx2).await.unwrap();

        let sent = manager
            .publish("test-topic", Message::Text("Hello".to_string()))
            .await
            .unwrap();

        assert_eq!(sent, 2);

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();

        match (msg1, msg2) {
            (Message::Text(t1), Message::Text(t2)) => {
                assert_eq!(t1, "Hello");
                assert_eq!(t2, "Hello");
            }
            _ => panic!("Expected text messages"),
        }
    }

    #[tokio::test]
    async fn test_disconnect_cleanup() {
        let manager = ChannelManager::new();
        let (tx, _rx) = mpsc::channel(100);
        let conn_id = Uuid::new_v4();

        manager
            .subscribe(conn_id, "topic1", tx.clone())
            .await
            .unwrap();
        manager.subscribe(conn_id, "topic2", tx).await.unwrap();

        assert_eq!(manager.connection_count().await, 1);
        assert_eq!(manager.topic_count().await, 2);

        manager.disconnect(conn_id).await;

        assert_eq!(manager.connection_count().await, 0);
        assert_eq!(manager.topic_count().await, 0);
    }
}
