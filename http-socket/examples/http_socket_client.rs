#[cfg(feature = "client")]
mod enabled {
    use std::collections::BTreeSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    use http_socket::protocol::handshake::ClientAdvertise;
    use http_socket::{
        CapabilityKey, CapabilityMap, CapabilityRequirement, CapabilityValue, ClientBuilder,
        ProtocolVersion, TransportKind,
    };

    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let base_url = std::env::args()
            .nth(1)
            .or_else(|| std::env::var("HTTP_SOCKET_SERVER").ok())
            .unwrap_or_else(|| "http://127.0.0.1:4000".to_string());

        let mut shared_caps = CapabilityMap::new();
        shared_caps.insert(CapabilityKey::new("resume"), CapabilityValue::Bool(true));
        shared_caps.insert(
            CapabilityKey::new("codec.json"),
            CapabilityValue::Bool(true),
        );
        shared_caps.insert(
            CapabilityKey::new("compression"),
            CapabilityValue::Text("gzip".to_string()),
        );

        let client = ClientBuilder::new()
            .preferred_transports(vec![TransportKind::Ws, TransportKind::Poll])
            .supported_versions(vec![ProtocolVersion::new(1)])
            .capabilities(shared_caps.clone())
            .build()?;
        let required = {
            let mut values = BTreeSet::new();
            values.insert(CapabilityKey::new("resume"));
            values
        };
        let advertise = ClientAdvertise::from_required_keys(
            client.preferred_transports.clone(),
            client.supported_versions.clone(),
            client.capabilities.clone(),
            required,
        );
        let connect_payload = encode_advertise(&advertise);

        let http = reqwest::Client::new();
        println!("server: {base_url}");
        println!("connect payload: {connect_payload}");

        let connect_response = http
            .post(format!("{base_url}/socket/connect"))
            .body(connect_payload)
            .send()
            .await?;
        let connect_status = connect_response.status();
        let connect_text = connect_response.text().await?;
        println!("connect => {connect_status} {connect_text}");
        if !connect_status.is_success() {
            return Ok(());
        }

        let session_id = match parse_response_field(&connect_text, "session_id") {
            Some(value) => value.to_string(),
            None => {
                println!("connect response missing session_id");
                return Ok(());
            }
        };

        let send_body = format!(
            "hello-http-socket:{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
        );
        let send_response = http
            .post(format!("{base_url}/socket/send/{session_id}"))
            .body(send_body)
            .send()
            .await?;
        let send_status = send_response.status();
        let send_text = send_response.text().await?;
        println!("send => {send_status} {send_text}");

        let status_response = http
            .get(format!("{base_url}/socket/session/{session_id}"))
            .send()
            .await?;
        let status_status = status_response.status();
        let status_text = status_response.text().await?;
        println!("status(before swap) => {status_status} {status_text}");

        let swap_response = http
            .post(format!("{base_url}/socket/swap/{session_id}"))
            .body("resume-token-demo")
            .send()
            .await?;
        let swap_status = swap_response.status();
        let swap_text = swap_response.text().await?;
        println!("swap => {swap_status} {swap_text}");

        let status_response = http
            .get(format!("{base_url}/socket/session/{session_id}"))
            .send()
            .await?;
        let status_status = status_response.status();
        let status_text = status_response.text().await?;
        println!("status(after swap) => {status_status} {status_text}");

        Ok(())
    }

    fn encode_advertise(advertise: &ClientAdvertise) -> String {
        let transports = advertise
            .transports
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let versions = advertise
            .versions
            .iter()
            .map(|value| value.as_u16().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let capabilities = advertise
            .capabilities
            .iter()
            .map(|(key, value)| format!("{}:{}", key.as_str(), format_capability_value(value)))
            .collect::<Vec<_>>()
            .join(",");
        let required = advertise
            .required_capabilities
            .iter()
            .map(|(key, requirement)| {
                format!(
                    "{}:{}",
                    key.as_str(),
                    format_capability_requirement(requirement)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "transports={transports};versions={versions};caps={capabilities};required={required}"
        )
    }

    fn format_capability_value(value: &CapabilityValue) -> String {
        match value {
            CapabilityValue::Bool(value) => value.to_string(),
            CapabilityValue::Number(value) => value.to_string(),
            CapabilityValue::Text(value) => value.clone(),
        }
    }

    fn format_capability_requirement(requirement: &CapabilityRequirement) -> String {
        match requirement {
            CapabilityRequirement::Present => "present".to_string(),
            CapabilityRequirement::BoolTrue => "true".to_string(),
            CapabilityRequirement::Equals(value) => format_capability_value(value),
        }
    }

    fn parse_response_field<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
        for segment in raw.split(';') {
            let (field, value) = match segment.split_once('=') {
                Some(pair) => pair,
                None => continue,
            };
            if field.trim() == key {
                return Some(value.trim());
            }
        }
        None
    }
}

#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enabled::run().await
}

#[cfg(not(feature = "client"))]
fn main() {
    println!("http_socket_client requires `client` feature");
}
