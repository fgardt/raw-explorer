use super::DedupValue;

pub async fn fetch_data<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        let msg = resp.text().await.map_err(|e| e.to_string())?;

        if msg.is_empty() {
            return Err(format!("Request failed with status: {status}"));
        } else {
            return Err(format!("Request failed ({status}): {msg}"));
        }
    }

    let json = resp.json::<T>().await.map_err(|e| e.to_string())?;
    Ok(json)
}

pub async fn fetch_from_resolver<T: serde::de::DeserializeOwned>(uri: &str) -> Result<T, String> {
    fetch_data(&format!("https://modname_resolver.bpbin.com/{uri}")).await
}

pub async fn get_dump(variant: String) -> Result<DedupValue, String> {
    let res = fetch_from_resolver(&format!("raw/{variant}")).await?;
    Ok(res)
}
