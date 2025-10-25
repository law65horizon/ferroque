use time::OffsetDateTime;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use time::serde::rfc3339;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Succedded,
    Failed,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub priority: i16,
    pub attempts: i16,
    pub max_attempts: i16,
    pub error: Option<String>,
    #[serde(with = "rfc3339")]
    pub run_at: OffsetDateTime,
    #[serde(with = "rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub job_type: String,
    pub payload: Option<serde_json::Value>,
    pub priority: Option<i16>,
    pub max_attempts: Option<i16>,
    #[serde(default, with = "rfc3339::option")]
    pub run_at: Option<OffsetDateTime>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_job_request_defaults_are_sensible() {
        let req = CreateJobRequest {
            job_type: "send_email".to_string(),
            payload: None,
            priority: None,
            max_attempts: None,
            run_at: None
        };

        assert_eq!(req.job_type, "send_email");
        assert!(req.payload.is_none());
        assert!(req.priority.is_none());
        assert!(req.max_attempts.is_none());
    }

    #[test]
    fn job_type_can_be_any_string() {
        let types = vec!["send_email", "resize_image", "sync_crm", "generate_pdf"];
        for t in types {
            let req = CreateJobRequest {
                job_type: t.to_string(),
                payload: Some(json!({"key": "value"})),
                priority: Some(1),
                max_attempts: None,
                run_at: None,
            };

            assert_eq!(req.job_type, t)
        }
    }

    #[test]
    fn payload_accepts_arbitary_json() {
        let payloads = vec![
            json!({}),
            json!({"to": "user@example.com"}),
            json!({"nested": {"key": [1,2,3]}})
        ];

        for payload in payloads {
            let req = CreateJobRequest {
                job_type: "test".to_string(),
                payload: Some(payload.clone()),
                priority: None,
                max_attempts: None,
                run_at: None
            };

            assert_eq!(req.payload.unwrap(), payload)
        }
    }
}