use std::collections::HashMap;

use anyhow::{Context, Result};
use log::*;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::pikpak::RetrySend;

use super::{Client, Resp};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMagnetResp {
    #[serde(rename = "upload_type")]
    pub upload_type: String,
    pub url: Url,
    pub file: Value,
    pub task: Task,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Url {
    pub kind: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub kind: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "user_id")]
    pub user_id: String,
    pub statuses: Vec<Value>,
    #[serde(rename = "status_size")]
    pub status_size: i64,
    #[serde(rename = "file_id")]
    pub file_id: String,
    #[serde(rename = "file_name")]
    pub file_name: String,
    #[serde(rename = "file_size")]
    pub file_size: String,
    pub message: String,
    #[serde(rename = "created_time")]
    pub created_time: String,
    #[serde(rename = "updated_time")]
    pub updated_time: String,

    pub progress: i64,
    #[serde(rename = "icon_link")]
    pub icon_link: String,

    pub space: String,
}

impl Client {
    pub async fn new_folder(&mut self, parent: &str, name: &str) -> Result<()> {
        let parent = self.get_path_id(parent).await?;

        let body = json!({
            "kind":      "drive#folder",
            "parent_id": parent.get_id(),
            "name":      name,
        });

        let mut headers = HeaderMap::new();
        headers.insert("Country", "CN".parse()?);
        headers.insert("X-Peer-Id", self.device_id.parse()?);
        headers.insert("X-User-Region", "1".parse()?);
        headers.insert("X-Alt-Capability", "3".parse()?);
        headers.insert("X-Client-Version-Code", "10083".parse()?);
        headers.insert("X-Captcha-Token", self.captcha_token.parse()?);

        let req = self
            .client
            .post("https://api-drive.mypikpak.com/drive/v1/files")
            .json(&body)
            .bearer_auth(&self.jwt_token)
            .headers(headers);

        debug!("req: {:?}", req);

        for _ in 0..2 {
            match req
                .try_clone()
                .ok_or(anyhow::anyhow!("clone request failed"))?
                .retry_send(self.retry_times)
                .await
                .context("[new_folder]")?
                .json::<Resp<Value>>()
                .await
                .context("[new_folder]")?
            {
                Resp::Success(resp) => {
                    debug!("resp: {:?}", resp);
                    return Ok(());
                }
                Resp::Err(err) => {
                    if err.error_code == 9 {
                        if let Err(err) =
                            self.auth_captcha_token("GET:/drive/v1/files".into()).await
                        {
                            error!("[new_folder] failed, err: {:#?}", err);
                            return Err(anyhow::anyhow!("[new_folder]  failed"));
                        }
                    } else {
                        error!("[get_file_by_id] failed, err: {:#?}", err);
                        return Err(anyhow::anyhow!("[new_folder] failed"));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("[new_folder] failed"))
    }

    pub async fn new_magnet(&mut self, path: &str, magnet_url: &str) -> Result<NewMagnetResp> {
        let parent = self.get_path_id(path).await?;

        let body = json!({
            "kind":      "drive#file",
            "parent_id": parent.get_id(),
            "upload_type": "UPLOAD_TYPE_URL",
            "url": {
                "url": magnet_url,
            },
        });

        let mut headers = HeaderMap::new();
        headers.insert("Country", "CN".parse()?);
        headers.insert("X-Peer-Id", self.device_id.parse()?);
        headers.insert("X-User-Region", "1".parse()?);
        headers.insert("X-Alt-Capability", "3".parse()?);
        headers.insert("X-Client-Version-Code", "10083".parse()?);
        headers.insert("X-Captcha-Token", self.captcha_token.parse()?);

        let req = self
            .client
            .post("https://api-drive.mypikpak.com/drive/v1/files")
            .json(&body)
            .bearer_auth(&self.jwt_token)
            .headers(headers);

        debug!("req: {:?}", req);

        req.try_clone()
            .ok_or(anyhow::anyhow!("clone request failed"))?
            .retry_send(self.retry_times)
            .await
            .context("[new_magnet]")?
            .json::<NewMagnetResp>()
            .await
            .context("[new_magnet]")
    }
}

#[cfg(test)]
mod tests {
    use rand::distributions::{Alphanumeric, DistString};

    use crate::{
        config::{get_client_options, load_config},
        logger::setup_test_logger,
    };

    use super::*;

    #[tokio::test]
    async fn test_new_folder() -> Result<()> {
        setup_test_logger().ok();
        if load_config("config.yml").is_err() {
            return Ok(());
        }

        if let Ok(mut client) = Client::new(get_client_options()) {
            client.login().await.ok();

            let file_name = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);

            client.new_folder("cli/test", file_name.as_str()).await?;

            let new_folder = client
                .get_path_id(format!("cli/test/{}", file_name).as_str())
                .await?;

            info!("{:#?}", new_folder);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_new_magnet() -> Result<()> {
        setup_test_logger().ok();
        if load_config("config.yml").is_err() {
            return Ok(());
        }

        if let Ok(mut client) = Client::new(get_client_options()) {
            client.login().await.ok();

            let res = client
                .new_magnet(
                    "cli/test",
                    "magnet:?xt=urn:btih:768505aa03a891a59e4af4a7dc7a6c2131cfe296",
                )
                .await?;

            info!("{:#?}", res.task);
        }

        Ok(())
    }
}
