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
    pub fn pull(&mut self, image: &ImageIdentifier, destination: &str) {
        // Perform the little auth dance
        let auth_url = format!("https://auth.docker.io/token?service=registry.docker.io&scope=repository%3A{}%2F{}%3Apull", image.author, image.name);
        let auth: serde_json::Value = self
            .http_client
            .get(&auth_url)
            .send()
            .unwrap()
            .json()
            .unwrap();
        let access_token = auth["token"].as_str().unwrap();

        // Download the image manifest
        let auth_header = format!("Bearer {}", access_token);
        let image_url = format!(
            "https://registry.hub.docker.com/v2/{}/{}/manifests/{}",
            image.author, image.name, image.tag
        );
        let image_manifest: serde_json::Value = self
            .http_client
            .get(&image_url)
            .header(reqwest::header::AUTHORIZATION, auth_header.to_owned())
            .send()
            .unwrap()
            .json()
            .unwrap();

        // Download the image layers and extracts them
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
                .header(reqwest::header::AUTHORIZATION, auth_header.to_owned())
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
                tag: "latest".to_string()
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
