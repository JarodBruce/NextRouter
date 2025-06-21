use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrometheusData {
    #[serde(rename = "resultType")]
    pub result_type: String,
    pub result: Vec<PrometheusResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrometheusResult {
    pub metric: HashMap<String, String>,
    pub value: Option<PrometheusValue>,
    pub values: Option<Vec<PrometheusValue>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrometheusValue(pub f64, pub String);

#[derive(Debug, Deserialize, Serialize)]
pub struct LabelResponse {
    pub status: String,
    pub data: Vec<String>,
}
