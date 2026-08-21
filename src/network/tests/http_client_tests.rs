use crate::network::{HttpClient, HttpClientStats};
use std::time::Duration;

#[test]
fn test_http_client_creation() {
    let client = HttpClient::new();
    assert!(client.is_ok());

    let client = client.unwrap();
    let stats = client.get_stats();

    assert_eq!(stats.total_user_agents, 3);
    assert_eq!(stats.current_user_agent_index, 0);
    assert!(stats.default_timeout_secs > 0);
}

#[test]
fn test_http_client_configuration() {
    let user_agents = vec!["TestAgent/1.0".to_string(), "TestAgent/2.0".to_string()];

    let client = HttpClient::new()
        .unwrap()
        .with_timeout(Duration::from_secs(15))
        .with_user_agents(user_agents.clone())
        .with_max_content_size(5 * 1024 * 1024);

    let stats = client.get_stats();
    assert_eq!(stats.total_user_agents, 2);
    assert_eq!(stats.default_timeout_secs, 15);
}
