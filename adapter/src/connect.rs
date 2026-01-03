//! Connection utilities for ScyllaDB

use std::num::NonZeroUsize;

use latte_core::config::{ConnectionConf, Consistency, RetryInterval, SerialConsistency, ValidationStrategy};
use latte_core::scripting::context::Context;

use crate::config::BenchmarkConfig;
use crate::error::{CharybdisError, Result};

/// Convert our BenchmarkConfig to latte's ConnectionConf
fn to_connection_conf(config: &BenchmarkConfig) -> ConnectionConf {
    ConnectionConf {
        count: NonZeroUsize::new(1).unwrap(),
        addresses: config.hosts.iter().map(|h| format!("{}:{}", h, config.port)).collect(),
        user: config.username.clone().unwrap_or_default(),
        password: config.password.clone().unwrap_or_default(),
        ssl: false, // TODO: wire up SSL config
        ssl_ca_cert_file: None,
        ssl_cert_file: None,
        ssl_key_file: None,
        ssl_peer_verification: false,
        datacenter: None,
        rack: None,
        consistency: to_latte_consistency(&config.consistency),
        serial_consistency: SerialConsistency::LocalSerial,
        request_timeout: std::time::Duration::from_secs(30),
        page_size: NonZeroUsize::new(1000).unwrap(),
        retry_number: 10,
        retry_interval: RetryInterval::new("100ms,5s").unwrap(),
        validation_strategy: ValidationStrategy::FailFast,
    }
}

/// Convert our consistency to latte's
fn to_latte_consistency(c: &crate::config::Consistency) -> Consistency {
    match c {
        crate::config::Consistency::Any => Consistency::Any,
        crate::config::Consistency::One => Consistency::One,
        crate::config::Consistency::Two => Consistency::Two,
        crate::config::Consistency::Three => Consistency::Three,
        crate::config::Consistency::Quorum => Consistency::Quorum,
        crate::config::Consistency::All => Consistency::All,
        crate::config::Consistency::LocalQuorum => Consistency::LocalQuorum,
        crate::config::Consistency::EachQuorum => Consistency::EachQuorum,
        crate::config::Consistency::LocalOne => Consistency::LocalOne,
    }
}

/// Connect to ScyllaDB and return a latte Context
pub async fn connect(config: &BenchmarkConfig) -> Result<Context> {
    let conn_conf = to_connection_conf(config);
    latte_core::scripting::connect::connect(&conn_conf)
        .await
        .map_err(|e| CharybdisError::Connection(e.to_string()))
}
