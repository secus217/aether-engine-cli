use crate::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{primitives::ByteStream, Client};
use std::env;
use std::path::Path;
use uuid::Uuid;

pub struct S3Uploader {
    pub client: Client,
    pub bucket_name: String,
    output_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl S3Uploader {
    pub fn with_output_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.output_callback = Some(Box::new(callback));
        self
    }

    #[allow(dead_code)]
    fn output(&self, message: &str) {
        println!("{}", message);
    }

    pub async fn new() -> Result<Self> {
        // Verify required AWS credentials are set
        let _access_key = env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| anyhow::anyhow!("AWS_ACCESS_KEY_ID must be set"))?;

        let _secret_key = env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY must be set"))?;

        // Get S3 config from environment variables
        let bucket_name = env::var("AETHER_S3_BUCKET")
            .map_err(|_| anyhow::anyhow!("AETHER_S3_BUCKET environment variable not set"))?;

        let region = env::var("AETHER_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let endpoint = env::var("AETHER_S3_ENDPOINT").ok(); // Optional custom endpoint

        // Create AWS config
        let mut config_loader =
            aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region));

        // Set custom endpoint if provided (for Storj, MinIO, etc.)
        if let Some(endpoint_url) = &endpoint {
            config_loader = config_loader.endpoint_url(endpoint_url);
        }

        let config = config_loader.load().await;

        // Create S3 client with path-style addressing (required for Storj)
        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&config);

        if endpoint.is_some() {
            // Force path style and disable checksum for S3-compatible services like Storj
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket_name,
            output_callback: None,
        })
    }

    pub async fn upload_artifact(
        &self,
        artifact_path: &Path,
        app_id: Uuid,
        version: &str,
    ) -> Result<(String, String)> {
        // Test S3 connection first
        self.test_bucket_access().await?;

        // Generate S3 key
        let key = format!(
            "artifacts/{}/{}/{}.tar.gz",
            app_id,
            version,
            chrono::Utc::now().timestamp()
        );

        // Read file content as bytes (better compatibility with Storj)
        let file_content = std::fs::read(artifact_path)
            .map_err(|e| anyhow::anyhow!("Failed to read artifact file: {}", e))?;
        let body = ByteStream::from(file_content);

        // Upload to S3
        println!("🔄 Starting upload...");
        println!("   Key: {}", key);

        let mut put_request = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .body(body)
            .content_type("application/gzip")
            .metadata("app_id", app_id.to_string())
            .metadata("version", version)
            .metadata("uploaded_at", chrono::Utc::now().to_rfc3339());

        // For Storj/S3-compatible services, disable content SHA256
        if env::var("AETHER_S3_ENDPOINT").is_ok() {
            put_request =
                put_request.checksum_algorithm(aws_sdk_s3::types::ChecksumAlgorithm::Sha256);
        }

        let result = put_request.send().await;

        match result {
            Ok(_) => {
                // Don't print here - let the caller handle progress/success messages
            }
            Err(e) => {
                eprintln!("❌ Upload failed: {:?}", e);
                return Err(anyhow::anyhow!(
                    "Failed to upload to S3: {}\nBucket: {}\nKey: {}\nError: {:?}",
                    e,
                    self.bucket_name,
                    key,
                    e
                )
                .into());
            }
        }

        // Generate presigned URL (valid for 24 hours)
        let presigned_url = self.get_presigned_url(&key, 86400).await?;

        // Return S3 URL and presigned URL
        let s3_url = format!("s3://{}/{}", self.bucket_name, key);
        Ok((s3_url, presigned_url))
    }

    async fn test_bucket_access(&self) -> Result<()> {
        println!("🔍 Testing S3 bucket access...");
        println!("   Bucket: {}", self.bucket_name);

        // Try to head bucket to test access
        let result = self
            .client
            .head_bucket()
            .bucket(&self.bucket_name)
            .send()
            .await;

        match result {
            Ok(_) => {
                println!("✅ S3 bucket access successful");
                Ok(())
            }
            Err(e) => {
                println!("❌ S3 bucket access failed: {}", e);

                // Try to list all buckets to test general AWS access
                println!("🔍 Testing general AWS S3 access...");
                let list_result = self.client.list_buckets().send().await;
                match list_result {
                    Ok(buckets) => {
                        println!("✅ AWS S3 access working. Available buckets:");
                        let bucket_list = buckets.buckets();
                        for bucket in bucket_list {
                            if let Some(name) = bucket.name() {
                                println!("   - {}", name);
                            }
                        }
                        Err(anyhow::anyhow!(
                            "Bucket '{}' not found or no access. See available buckets above.",
                            self.bucket_name
                        )
                        .into())
                    }
                    Err(list_err) => Err(anyhow::anyhow!(
                        "Cannot access AWS S3 at all: {}\nCheck your credentials and permissions",
                        list_err
                    )
                    .into()),
                }
            }
        }
    }

    pub async fn get_presigned_url(&self, s3_key: &str, expires_in_secs: u64) -> Result<String> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(
            std::time::Duration::from_secs(expires_in_secs),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create presigning config: {}", e))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;

        Ok(presigned_request.uri().to_string())
    }
}
