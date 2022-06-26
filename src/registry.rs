// TODO: Enable derive feature and convert dynamic to typed json

pub struct Registry {
    http_client: reqwest::blocking::Client,
}

impl Default for Registry {
    fn default() -> Self {
        return Registry {
            http_client: reqwest::blocking::Client::new(),
        };
    }
}

impl Registry {
    fn build_challenge_url(auth_header: &str) -> reqwest::Url {
        let challenge_part = auth_header.split(" ").nth(1).unwrap();
        let mut field_map = std::collections::HashMap::<&str, &str>::new();
        for field in challenge_part.split(",") {
            let mut field_split = field.split("=");
            let key = field_split.next().unwrap();
            let value = field_split.next().unwrap();
            field_map.insert(key, &value[1..value.len() - 1]);
        }
        let realm = field_map.get("realm").unwrap().to_owned();
        field_map.remove("realm").unwrap();
        let mut url = reqwest::Url::parse(realm).unwrap();

        {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in field_map {
                query_pairs.append_pair(key, value);
            }
        }

        return url;
    }

    fn fetch_manifest(
        &self,
        image: &ImageIdentifier,
        additional_headers: Option<reqwest::header::HeaderMap>,
    ) -> reqwest::blocking::Response {
        let image_url = format!(
            "https://registry.hub.docker.com/v2/{}/{}/manifests/{}",
            image.author, image.name, image.tag
        );
        let image_manifest = self
            .http_client
            .get(&image_url)
            .headers(additional_headers.unwrap_or_default())
            .send()
            .unwrap();

        return image_manifest;
    }

    pub fn pull(&mut self, image: &ImageIdentifier, destination: &str) {
        let mut header_map = reqwest::header::HeaderMap::new();

        let mut manifest_resp = self.fetch_manifest(image, None);

        // Perform the little auth dance if necessary
        if manifest_resp.status() != reqwest::StatusCode::OK {
            let auth_header = manifest_resp
                .headers()
                .get(reqwest::header::WWW_AUTHENTICATE)
                .unwrap()
                .to_str()
                .unwrap();

            let challenge_url = Registry::build_challenge_url(auth_header);
            let challenge_body: serde_json::Value = self
                .http_client
                .get(challenge_url)
                .send()
                .unwrap()
                .json()
                .unwrap();
            let access_token = challenge_body["token"].as_str().unwrap();
            header_map.append(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", access_token).parse().unwrap(),
            );

            manifest_resp = self.fetch_manifest(image, Some(header_map.to_owned()));
        }

        let image_manifest: serde_json::Value = manifest_resp.json().unwrap();

        let temp_path = std::env::temp_dir();
        for layer in image_manifest["fsLayers"].as_array().unwrap() {
            let digest = layer["blobSum"].as_str().unwrap();
            let blob_url = format!(
                "https://registry.hub.docker.com/v2/{}/{}/blobs/{}",
                image.author, image.name, digest
            );
            let blob = self
                .http_client
                .get(&blob_url)
                .headers(header_map.to_owned())
                .send()
                .unwrap()
                .bytes()
                .unwrap();

            let layer_path = temp_path.join(digest);
            std::fs::write(layer_path.to_owned(), blob).unwrap();

            // TODO: handle exit code
            std::process::Command::new("tar")
                .args(["-xf", layer_path.to_str().unwrap(), "-C", destination])
                .output()
                .unwrap();

            std::fs::remove_file(layer_path).unwrap();
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ImageIdentifier {
    author: String,
    name: String,
    tag: String,
}

impl ImageIdentifier {
    pub fn from_string(image: &String) -> Self {
        let mut iter = image.splitn(2, ':');
        let mut loc_iter = iter.next().unwrap().split('/').rev();
        let name = loc_iter
            .next()
            .expect("No image name was supplied")
            .to_string();
        let author = loc_iter.next().unwrap_or("library").to_string();
        let tag = iter.next().unwrap_or("latest").to_string();
        return ImageIdentifier { author, name, tag };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_name() {
        assert_eq!(
            ImageIdentifier::from_string(&"library/ubuntu:latest".to_string()),
            ImageIdentifier {
                author: "library".to_string(),
                name: "ubuntu".to_string(),
                tag: "latest".to_string()
            }
        );

        assert_eq!(
            ImageIdentifier::from_string(&"alpine".to_string()),
            ImageIdentifier {
                author: "library".to_string(),
                name: "alpine".to_string(),
                tag: "latest".to_string(),
            }
        );

        assert_eq!(
            ImageIdentifier::from_string(&"ghcr.io/dusk-labs/dim:dev".to_string()),
            ImageIdentifier {
                author: "dusk-labs".to_string(),
                name: "dim".to_string(),
                tag: "dev".to_string()
            }
        );

        assert_eq!(
            ImageIdentifier::from_string(&"bitnami/redis:7.0".to_string()),
            ImageIdentifier {
                author: "bitnami".to_string(),
                name: "redis".to_string(),
                tag: "7.0".to_string()
            }
        );
    }
}
