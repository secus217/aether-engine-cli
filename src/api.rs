use crate::{AetherError, Result};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Application {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub runtime: String,
    pub deployment_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deployment {
    pub id: uuid::Uuid,
    pub app_id: uuid::Uuid,
    pub version: String,
    pub status: String,
    pub artifact_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateAppRequest {
    pub name: String,
    pub description: Option<String>,
    pub runtime: String,
}

#[derive(Debug, Serialize)]
pub struct DeployRequest {
    pub app_id: uuid::Uuid,
    pub version: String,
    pub artifact_url: String,
}

// Authentication models
#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize)]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Custom Domain models
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomDomain {
    pub id: uuid::Uuid,
    pub domain: String,
    pub verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct AddCustomDomainRequest {
    pub domain: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomDomainResponse {
    pub id: uuid::Uuid,
    pub domain: String,
    pub verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Presigned URL models
#[derive(Debug, Serialize)]
pub struct GeneratePresignedUrlRequest {
    pub app_id: uuid::Uuid,
    pub version: String,
    pub filename: String,
}

#[derive(Debug, Deserialize)]
pub struct GeneratePresignedUrlResponse {
    pub upload_url: String,
    pub s3_key: String,
    pub expires_in: u64,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
}

impl Clone for ApiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            auth_token: self.auth_token.clone(),
        }
    }
}

impl ApiClient {
    pub fn new(base_url: String, auth_token: Option<String>) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            base_url,
            auth_token,
        })
    }

    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            // Control Plane returns direct JSON, not wrapped in ApiResponse
            let data: T = serde_json::from_str(&body)?;
            Ok(data)
        } else {
            Err(AetherError::Api {
                status: status.as_u16(),
                message: body,
            })
        }
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AetherError::Api {
                status: response.status().as_u16(),
                message: "Health check failed".to_string(),
            })
        }
    }

    // Authentication methods
    pub async fn register(&self, email: String, password: String) -> Result<AuthResponse> {
        let url = format!("{}/api/v1/auth/register", self.base_url);
        let request = RegisterRequest { email, password };

        let response = self.client.post(&url).json(&request).send().await?;
        self.handle_response(response).await
    }

    pub async fn login(&self, email: String, password: String) -> Result<AuthResponse> {
        let url = format!("{}/api/v1/auth/login", self.base_url);
        let request = LoginRequest { email, password };

        let response = self.client.post(&url).json(&request).send().await?;
        self.handle_response(response).await
    }

    pub async fn get_me(&self) -> Result<UserResponse> {
        let url = format!("{}/api/v1/auth/me", self.base_url);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn create_application(&self, request: CreateAppRequest) -> Result<Application> {
        let url = format!("{}/api/v1/apps", self.base_url);
        let mut req = self.client.post(&url).json(&request);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn list_applications(&self) -> Result<Vec<Application>> {
        let url = format!("{}/api/v1/apps", self.base_url);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn get_application(&self, app_id: uuid::Uuid) -> Result<Application> {
        let url = format!("{}/api/v1/apps/{}", self.base_url, app_id);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn deploy_application(
        &self,
        app_id: uuid::Uuid,
        version: String,
        artifact_url: String,
    ) -> Result<Deployment> {
        let url = format!("{}/api/v1/apps/{}/deployments", self.base_url, app_id);

        // Create JSON payload with just S3 URL - backend will generate presigned URL
        let deploy_request = DeployRequest {
            app_id,
            version,
            artifact_url,
        };

        let mut req = self.client.post(&url).json(&deploy_request);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        let status = response.status();

        // Check if the response is an error before parsing
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(AetherError::Api {
                status: status.as_u16(),
                message: error_text,
            });
        }

        self.handle_response(response).await
    }

    pub async fn list_deployments(&self, app_id: uuid::Uuid) -> Result<Vec<Deployment>> {
        let url = format!("{}/api/v1/apps/{}/deployments", self.base_url, app_id);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn monitor_deployment(&self, app_id: uuid::Uuid) -> Result<Vec<String>> {
        let url = format!("{}/api/v1/apps/{}/monitor", self.base_url, app_id);

        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn get_logs(&self, app_id: uuid::Uuid, lines: Option<u32>) -> Result<String> {
        self.get_logs_with_follow(app_id, lines, false).await
    }

    pub async fn get_logs_with_follow(
        &self,
        app_id: uuid::Uuid,
        lines: Option<u32>,
        follow: bool,
    ) -> Result<String> {
        let mut url = format!("{}/api/v1/apps/{}/logs", self.base_url, app_id);

        let mut query_params = Vec::new();
        if let Some(lines) = lines {
            query_params.push(format!("lines={}", lines));
        }
        if follow {
            query_params.push("follow=true".to_string());
        }

        if !query_params.is_empty() {
            url = format!("{}?{}", url, query_params.join("&"));
        }

        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        if response.status().is_success() {
            let json_response: serde_json::Value = response.json().await?;

            if let Some(logs_array) = json_response.get("logs").and_then(|v| v.as_array()) {
                // Join log lines with newlines
                let logs_text = logs_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(logs_text)
            } else {
                Ok("No logs found".to_string())
            }
        } else {
            Err(AetherError::Api {
                status: response.status().as_u16(),
                message: "Failed to fetch logs".to_string(),
            })
        }
    }

    pub async fn delete_application(&self, app_id: uuid::Uuid) -> Result<()> {
        let url = format!("{}/api/v1/apps/{}", self.base_url, app_id);
        let mut req = self.client.delete(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            Err(AetherError::Api {
                status: status.as_u16(),
                message: body,
            })
        }
    }

    // Custom Domain methods
    pub async fn add_custom_domain(
        &self,
        app_id: uuid::Uuid,
        domain: String,
    ) -> Result<CustomDomainResponse> {
        let url = format!("{}/api/v1/apps/{}/domains", self.base_url, app_id);
        let request = AddCustomDomainRequest { domain };

        let mut req = self.client.post(&url).json(&request);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn list_custom_domains(
        &self,
        app_id: uuid::Uuid,
    ) -> Result<Vec<CustomDomainResponse>> {
        let url = format!("{}/api/v1/apps/{}/domains", self.base_url, app_id);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    pub async fn delete_custom_domain(
        &self,
        app_id: uuid::Uuid,
        domain_id: uuid::Uuid,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/apps/{}/domains/{}",
            self.base_url, app_id, domain_id
        );
        let mut req = self.client.delete(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            Err(AetherError::Api {
                status: status.as_u16(),
                message: body,
            })
        }
    }

    pub async fn verify_custom_domain(
        &self,
        app_id: uuid::Uuid,
        domain_id: uuid::Uuid,
    ) -> Result<CustomDomainResponse> {
        let url = format!(
            "{}/api/v1/apps/{}/domains/{}/verify",
            self.base_url, app_id, domain_id
        );
        let mut req = self.client.post(&url);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }

    // Presigned URL methods
    pub async fn get_presigned_upload_url(
        &self,
        app_id: uuid::Uuid,
        version: &str,
        filename: &str,
    ) -> Result<GeneratePresignedUrlResponse> {
        let url = format!("{}/api/v1/uploads/presigned-url", self.base_url);
        let request_body = GeneratePresignedUrlRequest {
            app_id,
            version: version.to_string(),
            filename: filename.to_string(),
        };

        let mut req = self.client.post(&url).json(&request_body);

        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;
        self.handle_response(response).await
    }
}
