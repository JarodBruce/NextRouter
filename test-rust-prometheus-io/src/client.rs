use reqwest::Client;
use chrono::{DateTime, Utc};
use crate::types::*;

#[derive(Clone)]
pub struct PrometheusClient {
    client: Client,
    base_url: String,
}

impl PrometheusClient {
    pub fn new(prometheus_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: prometheus_url.to_string(),
        }
    }

    /// 即時クエリを実行（現在の値を取得）
    pub async fn query(&self, query: &str) -> Result<PrometheusResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/query", self.base_url);
        
        let response = self
            .client
            .get(&url)
            .query(&[("query", query)])
            .send()
            .await?;

        let prometheus_response: PrometheusResponse = response.json().await?;
        Ok(prometheus_response)
    }

    /// 範囲クエリを実行（時系列データを取得）
    pub async fn query_range(
        &self,
        query: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: &str,
    ) -> Result<PrometheusResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/query_range", self.base_url);
        
        let response = self
            .client
            .get(&url)
            .query(&[
                ("query", query),
                ("start", &start.timestamp().to_string()),
                ("end", &end.timestamp().to_string()),
                ("step", step),
            ])
            .send()
            .await?;

        let prometheus_response: PrometheusResponse = response.json().await?;
        Ok(prometheus_response)
    }

    /// 利用可能なメトリクス名を取得
    pub async fn get_label_names(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/label/__name__/values", self.base_url);
        
        let response = self.client.get(&url).send().await?;
        let label_response: LabelResponse = response.json().await?;
        Ok(label_response.data)
    }

    /// 特定のメトリクスのラベル値を取得
    pub async fn get_label_values(&self, label: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/label/{}/values", self.base_url, label);
        
        let response = self.client.get(&url).send().await?;
        let label_response: LabelResponse = response.json().await?;
        Ok(label_response.data)
    }
}
